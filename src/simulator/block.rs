use std::{
    sync::Arc, 
    f64::consts::PI,
    cmp
};
use gtk::cairo::{
    Context,
    Error
};

use crate::{
    modules::Module,
    renderer::{
        self,
        Renderable
    }
};

#[derive(Default, Debug)]
pub struct Block {
    module: Arc<Module>,
    position: (i32, i32),
    start_pos: (i32, i32), // starting position of drag movements
    size: (i32, i32),
    highlighted: bool,
}

impl Block {    
    pub fn new(module: Arc<Module>, position: (i32, i32)) -> Self {
        let num_inputs = module.get_num_inputs();
        let num_outputs = module.get_num_outputs();
        Self {
            module,
            position,
            start_pos: (0, 0),
            size: (75, cmp::max(num_inputs, num_outputs) * 25 + 50),
            highlighted: false
        }
    }

    pub fn is_in_area(&self, area: (i32, i32, i32, i32)) -> bool {
        !(
            self.position.0 > area.2 || 
            self.position.1 > area.3 ||
            self.position.0 + self.size.0 < area.0 || 
            self.position.1 + self.size.1 < area.1
        )
    }

    pub fn touches(&self, point: (i32, i32)) -> bool {
        point.0 > self.position.0 && point.0 < self.position.0 + self.size.0 &&
        point.1 > self.position.1 && point.1 < self.position.1 + self.size.1
    }

    pub fn set_highlighted(&mut self, highlighted: bool) {
        self.highlighted = highlighted;
    }

    pub fn highlighted(&self) -> bool {
        self.highlighted
    }

    pub fn set_position(&mut self, position: (i32, i32)) {
        self.position = position;
    }

    pub fn position(&self) -> (i32, i32) {
        self.position
    }

    pub fn set_start_pos(&mut self, start_pos: (i32, i32)) {
        self.start_pos = start_pos
    }

    pub fn start_pos(&self) -> (i32, i32) {
        self.start_pos
    }

    fn draw_connector(&self, context: &Context, position: (i32, i32)) -> Result<(), Error> {
        context.arc(position.0 as f64, position.1 as f64, 6., 0., 2. * PI);
        match self.highlighted {
            true => context.set_source_rgb(0.2078, 0.5176, 0.894),
            false => context.set_source_rgb(0.23, 0.23, 0.23)       
        }
        context.fill()?;
    
        context.arc(position.0 as f64, position.1 as f64, 5., 0., 2. * PI);
        context.set_source_rgb(0.5, 0.1, 0.7);
        context.fill()?;
        
        Ok(())
    }
}


impl Renderable for Block {
    fn render(&self, context: &Context) -> Result<(), Error> {
        renderer::draw_rounded_rect(context, self.position, self.size, 5);

        context.set_source_rgb(0.13, 0.13, 0.13);
        context.fill()?;

        renderer::draw_top_rounded_rect(context, self.position, (self.size.0, 25), 5);
        context.set_source_rgb(0.23, 0.23, 0.23);        
        context.fill()?;

        context.move_to(self.position.0 as f64 + 5., self.position.1 as f64 + 18.);
        context.set_source_rgb(1., 1., 1.);
        context.show_text(self.module.get_name().as_str())?;

        renderer::draw_rounded_rect(context, self.position, self.size, 5);
        match self.highlighted {
            true => context.set_source_rgb(0.2078, 0.5176, 0.894),
            false => context.set_source_rgb(0.23, 0.23, 0.23)       
        }
        context.stroke()?;

        let num_inputs = self.module.get_num_inputs();
        for i in 0..num_inputs {
            self.draw_connector(context, (self.position.0, self.position.1 + 25 * i + 50))?;
        }

        let num_outputs = self.module.get_num_outputs();
        for i in 0..num_outputs {
            self.draw_connector(context, (self.position.0 + self.size.0, self.position.1 + 25 * i + 50))?;
        }

        Ok(())
    }
}