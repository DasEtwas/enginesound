use crate::gen::{Engine, LowPassFilter};
use parking_lot::RwLock;

mod audio;
mod deser;
mod gen;
mod gui;
mod recorder;

use crate::recorder::Recorder;
use clap::{value_t, App, Arg};
use conrod_core::text::Font;
use glium::Surface;
use std::{fs::File, io::Read, sync::Arc};

mod support;

const SAMPLE_RATE: u32 = 48000;
const SPEED_OF_SOUND: f32 = 343.0; // m/s
const SAMPLES_PER_CALLBACK: u32 = 512;
const WINDOW_WIDTH: f64 = 800.0;
const WINDOW_HEIGHT: f64 = 800.0;
const DC_OFFSET_LP_FREQ: f32 = 4.0; // the frequency of the low pass filter which is subtracted from all samples to reduce dc offset and thus clipping
const MAX_CYLINDERS: usize = 16;
const MUFFLER_ELEMENT_COUNT: usize = 4;

const DEFAULT_CONFIG: &[u8] = include_bytes!("default.esc");

fn main() {
    let matches = App::new("Engine Sound Generator")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(Arg::with_name("headless").short("h").long("headless").help("CLI mode without GUI or audio playback").requires("config"))
        .arg(Arg::with_name("config").short("c").long("config").help("Sets the input file to load as an engine config").takes_value(true))
        .arg(Arg::with_name("volume").short("v").long("volume").help("Sets the master volume").default_value( "0.1"))
        .arg(Arg::with_name("rpm").short("r").long("rpm").help("Engine RPM").takes_value(true))
        .arg(Arg::with_name("warmup_time").short("w").long("warmup_time").help("Sets the time to wait in seconds before recording").default_value_if("headless", None, "3.0"))
        .arg(Arg::with_name("reclen").short("l").long("length").help("Sets the time to record in seconds").default_value_if("headless", None, "5.0"))
        .arg(Arg::with_name("output_file").short("o").long("output").help("Sets the output .wav file path").default_value_if("headless", None, "output.wav"))
        .arg(Arg::with_name("crossfade").short("f").long("crossfade").help("Crossfades the recording in the middle end-to-start to create a seamless loop, although adjusting the recording's length to the rpm is recommended. The value sets the size of the crossfade, where the final output is decreased in length by crossfade_time/2.").default_value_if("headless", None, "0.00133"))
        .get_matches();

    let bytes;
    let mut engine: Engine = match ron::de::from_bytes({
        bytes = match matches.value_of("config") {
            Some(path) => {
                match File::open(path) {
                    Ok(mut file) => {
                        let mut bytes = Vec::new();
                        file.read_to_end(&mut bytes).unwrap();
                        println!("Loaded config file \"{}\"", path);
                        bytes
                    },
                    Err(e) => {
                        eprintln!("Failed to open config file \"{}\": {}", path, e);
                        std::process::exit(1);
                    },
                }
            },
            None => DEFAULT_CONFIG.to_vec(),
        };
        &bytes
    }) {
        Ok(engine) => {
            println!("Successfully loaded config");
            engine
        },
        Err(e) => {
            eprintln!("Failed to parse config: {}", e);
            std::process::exit(2);
        },
    };

    match value_t!(matches.value_of("rpm"), f32) {
        Ok(rpm) => engine.rpm = rpm.max(0.0),
        Err(_) => (),
    }

    let cli_mode = matches.is_present("headless");

    // sound generator
    let mut generator = gen::Generator::new(SAMPLE_RATE, engine, LowPassFilter::new(DC_OFFSET_LP_FREQ, SAMPLE_RATE));

    generator.volume = value_t!(matches.value_of("volume"), f32).unwrap();

    if cli_mode {
        let warmup_time = value_t!(matches.value_of("warmup_time"), f32).unwrap().max(0.0); // has default value
        let record_time = value_t!(matches.value_of("reclen"), f32).unwrap().max(0.0); // has default value
        let output_filename = matches.value_of("output_file").unwrap(); // has default value

        println!("Warming up..");

        // warm up
        generator.generate(&mut vec![0.0; seconds_to_samples(warmup_time)]);

        println!("Recording..");

        // record
        let mut output = vec![0.0; seconds_to_samples(record_time)];

        generator.generate(&mut output);

        if matches.occurrences_of("crossfade") != 0 {
            let crossfade_duration = value_t!(matches.value_of("crossfade"), f32).unwrap();
            let crossfade_size = seconds_to_samples(crossfade_duration.max(1.0 / SAMPLE_RATE as f32));

            if crossfade_size >= output.len() {
                println!("Crossfade duration is too long {}", crossfade_duration);
                std::process::exit(4);
            }

            println!("Crossfading..");

            let len = output.len();
            let half_len = len / 2;

            let mut shifted = output.clone();

            shifted.iter_mut().enumerate().for_each(|(i, x)| *x = output[(half_len + i) % len]);

            output = Vec::with_capacity(shifted.len() - crossfade_size / 2);
            output.extend_from_slice(&shifted[..half_len]);
            output.extend_from_slice(&shifted[(half_len + crossfade_size / 2)..]);

            let fade_len = crossfade_size / 2;
            let start = half_len - fade_len;
            let end = half_len;
            for i in start..end {
                let fade = (i - start) as f32 / fade_len as f32;
                output[i] = shifted[i] * (1.0 - fade) + shifted[i + fade_len] * fade;
            }
        }

        let mut recorder = Recorder::new(output_filename.to_owned());

        println!("Started recording to \"{}\"", output_filename);

        // records into wav file asynchronously
        recorder.record(output.to_vec());
        recorder.stop_wait();
    } else {
        let generator = Arc::new(RwLock::new(generator));

        let audio = match audio::init(generator.clone(), SAMPLE_RATE) {
            Ok(audio) => audio,
            Err(e) => {
                eprintln!("Failed to initialize SDL2 audio: {}", e);
                std::process::exit(3);
            },
        };

        // GUI
        {
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
                        glium::glutin::Event::WindowEvent {
                            event, ..
                        } => {
                            match event {
                                glium::glutin::WindowEvent::DroppedFile(path) => {
                                    if let Some(new_engine) = crate::load_engine(path.to_str().unwrap_or("<invalid UTF-8>").to_owned()) {
                                        generator.write().engine = new_engine;
                                    }
                                },
                                glium::glutin::WindowEvent::CloseRequested
                                | glium::glutin::WindowEvent::KeyboardInput {
                                    input:
                                        glium::glutin::KeyboardInput {
                                            virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape), ..
                                        },
                                    ..
                                } => break 'main,
                                _ => (),
                            }
                        },
                        _ => (),
                    }
                }

                gui::gui(&mut ui.set_widgets(), &ids, generator.clone());

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

pub fn load_engine(path: String) -> Option<Engine> {
    match File::open(&path) {
        Ok(file) => {
            match ron::de::from_reader::<_, Engine>(file) {
                Ok(engine) => {
                    println!("Successfully loaded engine config \"{}\"", &path);
                    Some(engine)
                },
                Err(e) => {
                    eprintln!("Failed to load config \"{}\": {}", &path, e);
                    None
                },
            }
        },
        Err(e) => {
            eprintln!("Failed to load file \"{}\": {}", &path, e);
            None
        },
    }
}
