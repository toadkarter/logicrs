#![feature(const_fn_floating_point_arithmetic)]
#![feature(let_chains)]
#![feature(result_flattening)]
#![feature(const_trait_impl)]

mod application;
mod ui;
mod renderer;
mod simulator;
mod config;
mod fatal;
mod project;
mod export;

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

use adw::prelude::ApplicationExtManual;
use application::Application;

trait FileExtension {
    const FILE_EXTENSION: &'static str;
    const FILE_PATTERN: &'static str;

    fn file_filter() -> gtk::FileFilter;
}

fn main() {
    env_logger::init();
    info!("Starting up LogicRs...");    
    
    let application = Application::new();
    std::process::exit(application.run());
}

pub fn new_uuid() -> uuid::Uuid {
    /*static mut NODE: u64 = 0;
    unsafe {
        NODE += 1; 
        uuid::Uuid::now_v1(&NODE.to_le_bytes()[0..6].try_into().expect("could not generate node for uuid"))
    }*/

    uuid::Uuid::new_v4()
}
