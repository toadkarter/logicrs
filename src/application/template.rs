use gtk::{prelude::*, subclass::prelude::*, gio, glib, gdk};
use adw::subclass::prelude::*;
use std::cell::RefCell;
use adw::ColorScheme;
use crate::{
    fatal::*,
    project::*,
    renderer::Theme,
    simulator::*,
    ui::{circuit_view::CircuitView, dialogs, main_window::MainWindow},
};
use crate::application::gactions;
use crate::application::user_settings::UserSettings;
use crate::application::user_settings::UserSettingsKey::ThemeKey;
use crate::application::user_settings::UserSettingsValue::ThemeValue;

use super::{action::*, clipboard::Clipboard, selection::*, Application};

#[derive(Default)]
pub struct ApplicationTemplate {
    project: ProjectRef,
    window: RefCell<Option<MainWindow>>,
    simulator: RefCell<Option<Simulator>>,
    file: RefCell<Option<gio::File>>,
    action_stack: RefCell<ActionStack>,
    user_settings: RefCell<UserSettings>,
}

impl ApplicationTemplate {
    const CSS_RESOURCE: &'static str = "/style/style.css";

    fn start_simulation(&self) {
        *self.simulator.borrow_mut() =
            Some(Simulator::new(self.project.clone(), self.window.clone()))
    }

    fn stop_simulation(&self) {
        if let Some(simulator) = self.simulator.replace(None) {
            simulator.join();
        }
    }

    fn create_window(&self, application: &super::Application) {
        Theme::init();

        let provider = gtk::CssProvider::new();
        provider.load_from_resource(Self::CSS_RESOURCE);
        // We give the CssProvided to the default screen so the CSS rules we added
        // can be applied to our window.
        gtk::StyleContext::add_provider_for_display(
            &gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // build the application window and UI
        let window = MainWindow::new(application);
        window.show();
        self.window.replace(Some(window));

        let user_settings = self.user_settings.borrow_mut();
        let theme: gactions::Theme = match user_settings.get_setting(ThemeKey) {
            Some(ThemeValue(custom_theme)) => *custom_theme,
            _ => gactions::Theme::SystemPreference
        };

        let color_scheme = Into::<ColorScheme>::into(theme);
        adw::StyleManager::default().set_color_scheme(color_scheme);
    }

    pub fn save(&self, then: fn(&Application)) -> Result<(), String> {
        if let Some(file) = self.file.borrow().as_ref() {
            let project = self.project.lock().unwrap();
            project.write_to(file)?;
            if let Some(window) = self.window.borrow().as_ref() {
                window.set_subtitle(&self.file_name());
            }
            then(&self.instance());
        } else {
            self.instance().save_as(then);
        }

        self.action_stack().borrow_mut().set_dirty(false);
        Ok(())
    }

    pub fn set_project(&self, project: Project, file: Option<gio::File>) {
        self.stop_simulation();

        let mut old = self.project.lock().unwrap();
        *old = project;
        drop(old);

        self.file.replace(file);
        self.action_stack.borrow_mut().reset();
        if let Some(window) = self.window.borrow().as_ref() {
            window.reset_ui(&self.instance());
        }

        self.start_simulation();
    }

    pub fn window(&self) -> &RefCell<Option<MainWindow>> {
        &self.window
    }

    pub fn project(&self) -> &ProjectRef {
        &self.project
    }

    pub fn set_file(&self, file: gio::File) {
        self.file.replace(Some(file));
    }

    pub fn reset(&self) {
        self.set_project(Project::default(), None);
        if let Some(window) = self.window.borrow().as_ref() {
            window.reset_ui(&self.instance());
        }
    }

    pub fn file_name(&self) -> String {
        match self.file.borrow().as_ref() {
            Some(file) => file.path().unwrap().into_os_string().into_string().unwrap(),
            None => String::from("New File"),
        }
    }

    pub fn current_plot(&self) -> Option<PlotProvider> {
        self.current_circuit_view().map(|view| view.plot_provider())
    }

    pub fn current_circuit_view(&self) -> Option<CircuitView> {
        self.window
            .borrow()
            .as_ref()
            .and_then(|window| window.imp().circuit_panel.imp().view.selected_page())
            .and_then(|page| page.child().downcast::<CircuitView>().ok())
    }

    pub fn with_current_plot<T>(&self, func: impl Fn(&Plot) -> T) -> Option<T> {
        self.current_circuit_view()
            .and_then(|view| view.plot_provider().with(func))
    }

    pub fn with_current_plot_mut<T>(&self, func: impl Fn(&mut Plot) -> T) -> Option<T> {
        self.current_circuit_view()
            .and_then(|view| view.plot_provider().with_mut(func))
    }

    pub fn rerender_editor(&self) {
        if let Some(view) = self.current_circuit_view() {
            view.rerender();
        }
    }

    pub fn undo_button(&self) -> gtk::Button {
        self.window
            .borrow()
            .as_ref()
            .unwrap()
            .panel()
            .undo_button()
            .to_owned()
    }

    pub fn redo_button(&self) -> gtk::Button {
        self.window
            .borrow()
            .as_ref()
            .unwrap()
            .panel()
            .redo_button()
            .to_owned()
    }

    pub fn action_stack(&self) -> &RefCell<ActionStack> {
        &self.action_stack
    }

    pub fn is_dirty(&self) -> bool {
        self.action_stack.borrow().is_dirty()
    }

    pub fn user_settings(&self) -> &RefCell<UserSettings> { &self.user_settings }

    pub fn generate_clipboard(&self) -> Clipboard {
        if let Some(selected) = self.with_current_plot(|plot| !matches!(plot.selection(), Selection::None)) && selected {
            self.with_current_plot(|plot| Clipboard::from(plot)).unwrap_or_default()
        } else {
            Clipboard::Empty
        }
    }

    pub fn delete_module(&self, module_name: &String) {
        let mut locked = self.project.lock().unwrap();
        if let Some(module) = locked.module(module_name) {
            let owned_module = module.to_owned();
            let remove_dependencies = |plot: &mut Plot| {
                let delete = plot
                    .blocks_mut()
                    .iter()
                    .filter(|(_, block)| block.module_id() == owned_module.name())
                    .map(|(id, _)| *id)
                    .collect::<Vec<BlockID>>();

                delete.iter().for_each(|id| {
                    plot.delete_block(*id);
                });
            };

            remove_dependencies(locked.main_plot_mut());
            locked.modules_mut().iter_mut().for_each(|(_, module)| {
                if let Some(plot) = module.plot_mut() {
                    remove_dependencies(plot);
                }
            });

            drop(locked);
            self.instance()
                .new_action(Action::DeleteModule(self.project.clone(), owned_module));
        }
    }

    pub fn edit_module(&self, module_name: String) {
        let project = self.project.lock().unwrap();
        if let Some(module) = project.module(&module_name) {
            let module_name = module.name().clone();
            let provider = PlotProvider::Module(self.project.clone(), module_name);
            drop(project);
            self.window
                .borrow()
                .as_ref()
                .unwrap()
                .panel()
                .open_tab(provider);
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ApplicationTemplate {
    const NAME: &'static str = "Application";
    type Type = super::Application;
    type ParentType = adw::Application;
}

impl ObjectImpl for ApplicationTemplate {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.instance();
        obj.setup_gactions();
    }
}

impl ApplicationImpl for ApplicationTemplate {
    fn activate(&self) {
        self.create_window(&self.instance());
        self.start_simulation();
    }

    fn open(&self, files: &[gio::File], _hint: &str) {
        assert!(!files.is_empty());

        let file = &files[0];
        if file.path().is_none() {
            die("File path is None");
        }

        match Project::load_from(file) {
            Ok(data) => {
                let mut old_data = self.project.lock().unwrap();
                *old_data = data;
                std::mem::drop(old_data);

                self.file.replace(Some(file.to_owned()));
                self.create_window(&self.instance());
                self.start_simulation();
            }
            Err(err) => {
                self.create_window(&self.instance());
                self.start_simulation();

                dialogs::run(
                    self.instance().to_owned(),
                    self.instance().active_window().unwrap(),
                    err,
                    dialogs::basic_error,
                );
            }
        }
    }

    fn shutdown(&self) {
        self.stop_simulation();

        if let Some(window) = self.window.replace(None) {
            window.destroy();
        }
    }
}

impl GtkApplicationImpl for ApplicationTemplate {}

impl AdwApplicationImpl for ApplicationTemplate {}

impl WidgetImpl for ApplicationTemplate {}