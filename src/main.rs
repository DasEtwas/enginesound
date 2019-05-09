use crate::gen::LowPassFilter;
use parking_lot::RwLock;

mod audio;
mod deser;
mod gen;
mod gui;
mod recorder;

use crate::gui::MenuState;
use clap::{App, Arg};
use conrod_core::text::Font;
use glium::Surface;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

mod support;

/// recommended 48000hz, as any other freq produced whining for me on windows
const SAMPLE_RATE: u32 = 48000;
const SPEED_OF_SOUND: f32 = 343.0; // m/s
const SAMPLES_PER_CALLBACK: u32 = 512;
const WINDOW_WIDTH: f64 = 800.0;
const WINDOW_HEIGHT: f64 = 800.0;
const DC_OFFSET_LP_FREQ: f32 = 4.0; // the frequency of the low pass filter which is subtracted from all samples to reduce dc offset and thus clipping
const MAX_CYLINDERS: usize = 16;
const MUFFLER_ELEMENT_COUNT: usize = 4;

const DEFAULT_CONFIG: &[u8] = include_bytes!("default.es");

fn main() {
    let clap = App::new("Engine Sound Generator")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(Arg::with_name("headless").short("h").long("headless").help("CLI mode without GUI or audio playback").requires("config"))
        .arg(Arg::with_name("config").short("c").long("config").help("Sets the input file to load as an engine config").takes_value(true))
        .arg(Arg::with_name("warmup_time").short("w").long("warmup_time").help("Sets the time to wait in seconds before recording").takes_value(true).default_value("3.0"))
        .arg(Arg::with_name("reclen").short("l").long("length").help("Sets the time to record in seconds").takes_value(true).default_value("5.0"))
        .arg(Arg::with_name("output_file").short("o").long("output").help("Sets the output .wav file path").takes_value(true).default_value("output.wav"))
        .arg(Arg::with_name("crossfade").short("f").long("crossfade").help("Crossfades the recording in the middle end-to-start to create a seamless loop, although adjusting the recording's length to the rpm is recommended"))
        .get_matches();

    let mut bytes;
    let engine = match ron::de::from_bytes({
        bytes = match clap.value_of("config") {
            Some(path) => match File::open(path) {
                Ok(mut file) => {
                    let mut bytes = Vec::new();
                    file.read_to_end(&mut bytes).unwrap();
                    println!("Loaded config file \"{}\"", path);
                    bytes
                }
                Err(e) => panic!("Failed to open config file \"{}\": {}", path, e),
            },
            None => DEFAULT_CONFIG.to_vec(),
        };
        &bytes
    }) {
        Ok(engine) => {
            println!("Successfully loaded config");
            engine
        }
        Err(e) => panic!("Failed to parse config: {}", e),
    };

    // sound generator
    let generator = Arc::new(RwLock::new(gen::Generator::new(SAMPLE_RATE, engine, LowPassFilter::new(DC_OFFSET_LP_FREQ, SAMPLE_RATE))));

    let audio = match audio::init(generator.clone(), SAMPLE_RATE) {
        Ok(audio) => audio,
        Err(e) => panic!("Failed to initialize audio {}", e),
    };

    // GUI
    {
        let mut menu_state = MenuState::new();

        // Build the window.
        let mut events_loop = glium::glutin::EventsLoop::new();
        let window = glium::glutin::WindowBuilder::new()
            .with_title("Engine Sound Generator")
            .with_dimensions((WINDOW_WIDTH, WINDOW_HEIGHT).into())
            .with_max_dimensions((WINDOW_WIDTH + 1.0, WINDOW_HEIGHT + 1000.0).into())
            .with_min_dimensions((WINDOW_WIDTH, WINDOW_HEIGHT).into())
            .with_resizable(true);
        let context = glium::glutin::ContextBuilder::new().with_vsync(true).with_multisampling(4);
        let display = glium::Display::new(window, context, &events_loop).unwrap();
        let display = support::GliumDisplayWinitWrapper(display);

        let mut ui = conrod_core::UiBuilder::new([WINDOW_WIDTH, WINDOW_HEIGHT]).theme(gui::theme()).build();
        let ids = gui::Ids::new(ui.widget_id_generator());

        ui.fonts.insert(Font::from_bytes(&include_bytes!("../fonts/NotoSans/NotoSans-Regular.ttf")[..]).unwrap());

        let mut renderer = conrod_glium::Renderer::new(&display.0).unwrap();

        let image_map = conrod_core::image::Map::<glium::texture::Texture2d>::new();

        let mut event_loop = support::EventLoop::new();
        'main: loop {
            event_loop.needs_update();
            for event in event_loop.next(&mut events_loop) {
                if let Some(event) = conrod_winit::convert_event(event.clone(), &display) {
                    ui.handle_event(event);
                }

                match event {
                    glium::glutin::Event::WindowEvent { event, .. } => {
                        match event {
                            // Break from the loop upon `Escape`.
                            glium::glutin::WindowEvent::CloseRequested
                            | glium::glutin::WindowEvent::KeyboardInput {
                                input: glium::glutin::KeyboardInput { virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape), .. },
                                ..
                            } => break 'main,
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }

            gui::gui(&mut ui.set_widgets(), &ids, generator.clone(), &mut menu_state);

            if let Some(primitives) = ui.draw_if_changed() {
                renderer.fill(&display.0, primitives, &image_map);
                let mut target = display.0.draw();
                target.clear_color(0.0, 0.0, 0.0, 1.0);
                renderer.draw(&display.0, &mut target, &image_map).unwrap();
                target.finish().unwrap();
            }
        }
    }

    // audio lives until here
    std::mem::drop(audio);
}

/// converts a given amount of time into samples
pub fn seconds_to_samples(seconds: f32) -> usize {
    (seconds * SAMPLE_RATE as f32).max(1.0) as usize
}

/// converts a given distance into samples via the speed of sound
pub fn distance_to_samples(meters: f32) -> usize {
    seconds_to_samples(meters / SPEED_OF_SOUND)
}

pub fn samples_to_seconds(samples: usize) -> f32 {
    samples as f32 / SAMPLE_RATE as f32
}

/// returns meters
pub fn samples_to_distance(samples: usize) -> f32 {
    samples_to_seconds(samples) * SPEED_OF_SOUND
}
