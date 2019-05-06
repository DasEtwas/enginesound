#![feature(proc_macro_hygiene)]

use crate::gen::{Cylinder, Engine, LowPassFilter, Muffler, Noise, WaveGuide};
use parking_lot::RwLock;

mod audio;
mod gen;
mod gui;
mod recorder;

use conrod_core::text::Font;
use glium::Surface;
use std::sync::Arc;

mod support;

/// recommended 48000hz, as any other freq produced whining for me on windows
const SAMPLE_RATE: u32 = 48000;
const SPEED_OF_SOUND: f32 = 343.0; // m/s
const SAMPLES_PER_CALLBACK: u32 = 512;
const WINDOW_WIDTH: f64 = 800.0;
const WINDOW_HEIGHT: f64 = 800.0;
const DC_OFFSET_LP_FREQ: f32 = 4.0; // the frequency of the low pass filter which is subtracted from all samples to reduce dc offset and thus clipping

fn main() {
    let num_cylinders = 1;
    let mut cylinders = Vec::with_capacity(num_cylinders);

    /// converts a given amount of time into samples
    fn seconds_to_samples(seconds: f32) -> usize {
        (seconds * SAMPLE_RATE as f32).max(1.0) as usize
    }

    let speed_of_sound = 343.0; // m/s

    for i in 0..num_cylinders {
        cylinders.push(Cylinder {
            crank_offset: i as f32 / num_cylinders as f32,
            // alpha is set while running, exhaust_openside_refl: 0.1
            exhaust_waveguide: WaveGuide::new(seconds_to_samples(0.7 / speed_of_sound), -1000.0, 0.0),
            // alpha is set while running, beta is intake_openside_refl:  -0.5
            intake_waveguide:    WaveGuide::new(seconds_to_samples(0.7 / speed_of_sound), -1000.0, -0.5),
            extractor_waveguide: WaveGuide::new(seconds_to_samples(1.0 / speed_of_sound), 0.0, 0.7),
            intake_open_refl:    1.0,
            intake_closed_refl:  0.0,
            exhaust_open_refl:   1.0,
            exhaust_closed_refl: 0.0,

            piston_motion_factor:    0.6,
            ignition_factor:         1.9,
            ignition_time:           0.2,
            pressure_release_factor: (1.0 - 0.04f32).powf(1.0 / SAMPLE_RATE as f32),

            // running values
            cyl_sound:         0.0,
            cyl_pressure:      0.0,
            extractor_exhaust: 0.0,
        });
    }

    let engine: Engine = Engine {
        rpm: 700.0_f32,

        cylinders,
        intake_noise: Noise::default(),
        intake_noise_factor: 0.6,
        intake_lp_filter: LowPassFilter::new(2000.0, SAMPLE_RATE),
        engine_vibration_filter: LowPassFilter::new(300.0, SAMPLE_RATE),
        muffler: Muffler {
            muffler_elements: [
                WaveGuide::new(seconds_to_samples(0.05 / speed_of_sound), -0.5, -0.5),
                WaveGuide::new(seconds_to_samples(0.15 / speed_of_sound), -0.5, -0.5),
                WaveGuide::new(seconds_to_samples(0.35 / speed_of_sound), -0.5, -0.5),
                WaveGuide::new(seconds_to_samples(0.48 / speed_of_sound), -0.5, -0.5),
            ],
            straight_pipe:    WaveGuide::new(seconds_to_samples(2.0 / speed_of_sound), -0.2, -0.2),
        },

        intake_valve_shift: 0.0,
        exhaust_valve_shift: 0.0,
        crankshaft_fluctuation: 0.17,
        crankshaft_fluctuation_lp: LowPassFilter::new(350.0, SAMPLE_RATE),
        // running values
        /// crankshaft position, 0.0-1.0
        crankshaft_pos: 0.0,
        exhaust_collector: 0.0,
        intake_collector: 0.0,
    };

    // sound generator
    let generator = Arc::new(RwLock::new(gen::Generator::new(SAMPLE_RATE, engine, LowPassFilter::new(DC_OFFSET_LP_FREQ, SAMPLE_RATE))));

    let audio = match audio::init(generator.clone(), SAMPLE_RATE) {
        Ok(audio) => audio,
        Err(e) => panic!("Failed to initialize audio {}", e),
    };

    // GUI
    {
        // Build the window.
        let mut events_loop = glium::glutin::EventsLoop::new();
        let window = glium::glutin::WindowBuilder::new()
            .with_resizable(false)
            .with_title("Engine Sound Generator")
            .with_dimensions((WINDOW_WIDTH, WINDOW_HEIGHT).into());
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
                            // Break from the loop upon `Escape`.
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
