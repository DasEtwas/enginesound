use gtk::Builder;

mod audio;
mod gen;

fn main() {
    let application = gtk::Application::new("dasetwas.enginesound", Default::default()).expect("Failed to initialize GTK Application");

    let glade_src = include_str!("gui.glade");
    let builder = Builder::new_from_string(glade_src);
}
