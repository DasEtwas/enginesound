use gtk::Builder;
use parking_lot::Mutex;
use std::sync::Arc;

mod audio;
mod gen;

/// recommended 48000hz, as any other freq produced whining for me
const SAMPLE_RATE: u32 = 48000;

fn main() {
    let application = gtk::Application::new("dasetwas.enginesound", Default::default()).expect("Failed to initialize GTK Application");

    // sound generator
    let generator = Arc::new(Mutex::new(gen::Generator::new(SAMPLE_RATE)));

    match audio::init(generator.clone(), SAMPLE_RATE) {
        Ok(_) => (),
        Err(e) => println!("Failed to initialize audio {}", e),
    }

    let glade_src = include_str!("gui.glade");
    let builder = Builder::new_from_string(glade_src);
}
