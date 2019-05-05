use crate::{gen::Generator, recorder::Recorder, SAMPLE_RATE, SPEED_OF_SOUND};
use chrono::{Datelike, Local, Timelike};
use conrod_core::{position::{Align, Direction, Padding, Relative},
                  *};
use parking_lot::RwLock;
use std::sync::Arc;

/// A set of reasonable stylistic defaults that works for the `gui` below.
pub fn theme() -> conrod_core::Theme {
    conrod_core::Theme {
        name:                   "Demo Theme".to_string(),
        padding:                Padding::none(),
        x_position:             Position::Relative(Relative::Align(Align::Start), None),
        y_position:             Position::Relative(Relative::Direction(Direction::Backwards, 20.0), None),
        background_color:       conrod_core::color::rgb(0.24, 0.24, 0.26),
        shape_color:            conrod_core::color::rgb(0.17, 0.17, 0.19),
        border_color:           conrod_core::color::rgb(0.2, 0.2, 0.22),
        border_width:           0.0,
        label_color:            conrod_core::color::rgb(0.78, 0.78, 0.80),
        font_id:                None,
        font_size_large:        26,
        font_size_medium:       18,
        font_size_small:        12,
        widget_styling:         conrod_core::theme::StyleMap::default(),
        mouse_drag_threshold:   0.0,
        double_click_threshold: std::time::Duration::from_millis(400),
    }
}

// Generate a unique `WidgetId` for each widget.
widget_ids! {
    pub struct Ids {
        // The scrollable canvas.
        canvas,
        // The title and introduction widgets.
        title,
        duty_display,
        record_button,
        reset_button,
        engine_rpm_slider,
        engine_master_volume_slider,
        engine_intake_volume_slider,
        engine_exhaust_volume_slider,
        engine_engine_vibrations_volume_slider,

        engine_title,
        muffler_straight_pipe_alpha,
        muffler_straight_pipe_beta,
        muffler_straight_pipe_length,
        engine_intake_noise_factor,
        engine_intake_valve_shift,
        engine_exhaust_valve_shift,
        engine_crankshaft_fluctuation,

        cylinder_title,
        cylinder_offset_growl,
        cylinder_num,
        cylinder_crank_offset,
        cylinder_intake_open_refl,
        cylinder_intake_closed_refl,
        cylinder_exhaust_open_refl,
        cylinder_exhaust_closed_refl,
        cylinder_intake_open_end_refl,
        cylinder_extractor_open_end_refl,
        cylinder_piston_motion_factor,
        cylinder_ignition_factor,
        cylinder_ignition_time,
        cylinder_pressure_release_factor,

        graph,

        canvas_scrollbar,
    }
}

/// Instantiate a GUI demonstrating every widget available in conrod.
pub fn gui(ui: &mut conrod_core::UiCell, ids: &Ids, generator: Arc<RwLock<Generator>>) {
    const PAD_TOP: conrod_core::Scalar = 10.0;
    const PAD: conrod_core::Scalar = 20.0;

    widget::Canvas::new().pad(PAD).pad_right(PAD + 20.0).pad_top(0.0).scroll_kids_vertically().set(ids.canvas, ui);

    widget::Text::new("Engine Sound Generator").font_size(24).top_left_with_margins(PAD_TOP, PAD).w(ui.win_w - PAD * 2.0).set(ids.title, ui);

    {
        let mut generator = generator.write();
        widget::Text::new(format!("Current sampler duty: {:.2}%", generator.sampler_duty * 100.0).as_str()).down(7.0).set(ids.duty_display, ui);

        {
            let (button_label, remove_recorder) = match &mut generator.recorder {
                None => ("Start recording".to_string(), false),
                Some(recorder) => {
                    if recorder.is_running() {
                        ui.needs_redraw();
                        (format!("Stop recording [{:.3} sec recorded]", recorder.get_len() as f32 / crate::SAMPLE_RATE as f32), false)
                    } else {
                        ("Start recording".to_string(), true)
                    }
                },
            };

            if remove_recorder {
                generator.recorder = None;
            }

            for _press in widget::Button::new().label(button_label.as_str()).down(7.0).set(ids.record_button, ui) {
                match &mut generator.recorder {
                    None => {
                        generator.recorder = Some(Recorder::new(recording_name()));
                    },
                    Some(recorder) => {
                        recorder.stop();
                    },
                }
            }
        }

        {
            for _press in widget::Button::new().label("Reset sampler").down(5.0).set(ids.reset_button, ui) {
                generator.reset();
            }
        }

        {
            let prev_val = generator.get_rpm();
            for value in widget::Slider::new(prev_val, 300.0, 9000.0)
                .label(format!("Engine RPM {:.2}", prev_val).as_str())
                .label_font_size(12)
                .padded_w_of(ids.canvas, PAD)
                .down(5.0)
                .set(ids.engine_rpm_slider, ui)
            {
                generator.set_rpm(value);
            }
        }

        ///////////////////
        // Volumes       //
        ///////////////////

        {
            {
                let prev_val = generator.get_volume();
                for value in widget::Slider::new(prev_val, 0.0, 3.0)
                    .label(format!("Master volume {:.0}%", prev_val * 100.0).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.engine_master_volume_slider, ui)
                {
                    generator.set_volume(value);
                }
            }

            {
                let prev_val = generator.get_intake_volume();
                for value in widget::Slider::new(prev_val, 0.0, 1.0)
                    .label(format!("Intake volume {:.0}%", prev_val * 100.0).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.engine_intake_volume_slider, ui)
                {
                    let mut dif = value - prev_val;
                    generator.set_intake_volume(value);
                    let v1 = generator.get_exhaust_volume();
                    let v2 = generator.get_engine_vibrations_volume();
                    if v1 < v2 {
                        let vv1 = v1.min(dif * 0.5);
                        dif -= vv1;
                        generator.set_exhaust_volume((v1 - vv1).min(1.0).max(0.0));
                        generator.set_engine_vibrations_volume((v2 - dif).min(1.0).max(0.0));
                    } else {
                        let vv2 = v2.min(dif * 0.5);
                        dif -= vv2;
                        generator.set_engine_vibrations_volume((v2 - vv2).min(1.0).max(0.0));
                        generator.set_exhaust_volume((v1 - dif).min(1.0).max(0.0));
                    }
                }
            }

            {
                let prev_val = generator.get_exhaust_volume();
                for value in widget::Slider::new(prev_val, 0.0, 1.0)
                    .label(format!("Exhaust volume {:.0}%", prev_val * 100.0).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.engine_exhaust_volume_slider, ui)
                {
                    let mut dif = value - prev_val;
                    generator.set_exhaust_volume(value);
                    let v1 = generator.get_intake_volume();
                    let v2 = generator.get_engine_vibrations_volume();
                    if v1 < v2 {
                        let vv1 = v1.min(dif * 0.5);
                        dif -= vv1;
                        generator.set_intake_volume((v1 - vv1).min(1.0).max(0.0));
                        generator.set_engine_vibrations_volume((v2 - dif).min(1.0).max(0.0));
                    } else {
                        let vv2 = v2.min(dif * 0.5);
                        dif -= vv2;
                        generator.set_engine_vibrations_volume((v2 - vv2).min(1.0).max(0.0));
                        generator.set_intake_volume((v1 - dif).min(1.0).max(0.0));
                    }
                }
            }

            {
                let prev_val = generator.get_engine_vibrations_volume();
                for value in widget::Slider::new(prev_val, 0.0, 1.0)
                    .label(format!("Engine vibrations volume {:.0}%", prev_val * 100.0).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.engine_engine_vibrations_volume_slider, ui)
                {
                    let mut dif = value - prev_val;
                    generator.set_engine_vibrations_volume(value);
                    let v1 = generator.get_exhaust_volume();
                    let v2 = generator.get_intake_volume();
                    if v1 < v2 {
                        let vv1 = v1.min(dif * 0.5);
                        dif -= vv1;
                        generator.set_exhaust_volume((v1 - vv1).min(1.0).max(0.0));
                        generator.set_intake_volume((v2 - dif).min(1.0).max(0.0));
                    } else {
                        let vv2 = v2.min(dif * 0.5);
                        dif -= vv2;
                        generator.set_intake_volume((v2 - vv2).min(1.0).max(0.0));
                        generator.set_exhaust_volume((v1 - dif).min(1.0).max(0.0));
                    }
                }
            }

            // normalize again to mitigate any floating point error
            {
                let iv = generator.get_intake_volume();
                let ev = generator.get_exhaust_volume();
                let evv = generator.get_engine_vibrations_volume();
                let sum = iv + ev + evv;
                generator.set_intake_volume(iv / sum);
                generator.set_exhaust_volume(ev / sum);
                generator.set_engine_vibrations_volume(evv / sum);
            }
        }
        widget::Text::new("Engine parameters").font_size(16).down(7.0).w(ui.window_dim()[0] - PAD * 2.0).set(ids.engine_title, ui);
        {
            // exhaust_pipe_alpha
            {
                const MIN: f32 = -1.0;
                const MAX: f32 = 1.0;
                let prev_val = generator.engine.muffler.straight_pipe.alpha;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("muffler_straight_pipe_alpha {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(7.0)
                    .set(ids.muffler_straight_pipe_alpha, ui)
                {
                    generator.engine.muffler.straight_pipe.alpha = value;
                }
            }
            // engine_exhaust_pipe_beta
            {
                const MIN: f32 = -1.0;
                const MAX: f32 = 1.0;
                let prev_val = generator.engine.muffler.straight_pipe.beta;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("muffler_straight_pipe_beta {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(7.0)
                    .set(ids.muffler_straight_pipe_beta, ui)
                {
                    generator.engine.muffler.straight_pipe.beta = value;
                }
            }

            // muffler_straight_pipe_length
            {
                const MIN: f32 = 0.1;
                const MAX: f32 = 3.0;
                let prev_val = generator.engine.muffler.straight_pipe.chamber0.samples.len as f32 * SPEED_OF_SOUND / SAMPLE_RATE as f32;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("muffler_straight_pipe_length {:.2}m", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(7.0)
                    .set(ids.muffler_straight_pipe_length, ui)
                {
                    let alpha = generator.engine.muffler.straight_pipe.alpha;
                    let beta = generator.engine.muffler.straight_pipe.beta;
                    if let Some(newgen) = generator.engine.muffler.straight_pipe.update((value / SPEED_OF_SOUND * SAMPLE_RATE as f32) as usize, alpha, beta) {
                        generator.engine.muffler.straight_pipe = newgen;
                    }
                }
            }

            // intake_noise_factor
            {
                const MIN: f32 = 0.0;
                const MAX: f32 = 3.0;
                let prev_val = generator.engine.intake_noise_factor;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("intake_noise_factor {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(7.0)
                    .set(ids.engine_intake_noise_factor, ui)
                {
                    generator.engine.intake_noise_factor = value;
                }
            }
            // intake_valve_shift
            {
                const MIN: f32 = -0.5;
                const MAX: f32 = 0.5;
                let prev_val = generator.engine.intake_valve_shift;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("intake_valve_shift {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.engine_intake_valve_shift, ui)
                {
                    generator.engine.intake_valve_shift = value;
                }
            }
            // exhaust_valve_shift
            {
                const MIN: f32 = -0.5;
                const MAX: f32 = 0.5;
                let prev_val = generator.engine.exhaust_valve_shift;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("exhaust_valve_shift {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.engine_exhaust_valve_shift, ui)
                {
                    generator.engine.exhaust_valve_shift = value;
                }
            }

            // crankshaft_fluctuation
            {
                const MIN: f32 = 0.0;
                const MAX: f32 = 0.5;
                let prev_val = generator.engine.crankshaft_fluctuation;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("crankshaft_fluctuation {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.engine_crankshaft_fluctuation, ui)
                {
                    generator.engine.crankshaft_fluctuation = value;
                }
            }
        }

        widget::Text::new("Cylinder parameters").font_size(16).down(7.0).w(ui.window_dim()[0] - PAD * 2.0).set(ids.cylinder_title, ui);

        {
            // if a ui element is being changed, the cylinders need to be replaced
            let mut changed = false;
            let mut num_cylinders = generator.engine.cylinders.len();

            // inverse of i as f32 / num_cylinders as f32 * (1.0 - growl)
            let mut growl = 1.0 - (generator.engine.cylinders[num_cylinders - 1].crank_offset * num_cylinders as f32 / (num_cylinders - 1) as f32);
            {
                const MIN: f32 = 0.0;
                const MAX: f32 = 1.0;
                for value in widget::Slider::new(growl, MIN, MAX)
                    .label(format!("growl {}", growl).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_offset_growl, ui)
                {
                    changed = true;
                    growl = if value.is_normal() { value } else { 0.0 };
                }
            }

            {
                const MIN: f32 = 1.0;
                const MAX: f32 = 16.0;
                let prev_val = num_cylinders as f32;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("number of cylinders {}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_num, ui)
                {
                    let value = value.round() as usize;
                    if value != prev_val as usize {
                        changed = true;
                        num_cylinders = value as usize;
                    }
                }
            }

            let mut cylinder = generator.engine.cylinders[0].clone();

            // intake_open_refl
            {
                const MIN: f32 = -1.0;
                const MAX: f32 = 1.0;
                let prev_val = cylinder.intake_open_refl;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("intake_open_refl {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_intake_open_refl, ui)
                {
                    changed = true;
                    cylinder.intake_open_refl = value;
                }
            }
            // intake_closed_refl
            {
                const MIN: f32 = -1.0;
                const MAX: f32 = 1.0;
                let prev_val = cylinder.intake_closed_refl;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("intake_closed_refl {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_intake_closed_refl, ui)
                {
                    changed = true;
                    cylinder.intake_closed_refl = value;
                }
            }
            // exhaust_open_refl
            {
                const MIN: f32 = -1.0;
                const MAX: f32 = 1.0;
                let prev_val = cylinder.exhaust_open_refl;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("exhaust_open_refl {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_exhaust_open_refl, ui)
                {
                    changed = true;
                    cylinder.exhaust_open_refl = value;
                }
            }
            // exhaust_closed_refl
            {
                const MIN: f32 = -1.0;
                const MAX: f32 = 1.0;
                let prev_val = cylinder.exhaust_closed_refl;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("exhaust_closed_refl {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_exhaust_closed_refl, ui)
                {
                    changed = true;
                    cylinder.exhaust_closed_refl = value;
                }
            }
            // cylinder_intake_open_end_refl
            {
                const MIN: f32 = -1.0;
                const MAX: f32 = 1.0;
                let prev_val = cylinder.intake_waveguide.beta;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("cylinder_intake_open_end_refl {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_intake_open_end_refl, ui)
                {
                    changed = true;
                    cylinder.intake_waveguide.beta = value;
                }
            }
            // cylinder_extractor_open_end_refl
            {
                const MIN: f32 = -1.0;
                const MAX: f32 = 1.0;
                let prev_val = cylinder.extractor_waveguide.beta;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("cylinder_extractor_open_end_refl {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_extractor_open_end_refl, ui)
                {
                    changed = true;
                    cylinder.extractor_waveguide.beta = value;
                }
            }
            // piston_motion_factor
            {
                const MIN: f32 = 0.0;
                const MAX: f32 = 5.0;
                let prev_val = cylinder.piston_motion_factor;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("piston_motion_factor {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_piston_motion_factor, ui)
                {
                    changed = true;
                    cylinder.piston_motion_factor = value;
                }
            }
            // ignition_factor
            {
                const MIN: f32 = 0.0;
                const MAX: f32 = 5.0;
                let prev_val = cylinder.ignition_factor;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("ignition_factor {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_ignition_factor, ui)
                {
                    changed = true;
                    cylinder.ignition_factor = value;
                }
            }
            // ignition_time
            {
                const MIN: f32 = 0.0;
                const MAX: f32 = 1.0;
                let prev_val = cylinder.ignition_time;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("ignition_time {:.2}", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_ignition_time, ui)
                {
                    changed = true;
                    cylinder.ignition_time = value;
                }
            }
            // pressure_release_factor
            {
                const MIN: f32 = 0.007;
                const MAX: f32 = 0.4;
                let prev_val = 1.0 - cylinder.pressure_release_factor.powf(SAMPLE_RATE as f32);
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("pressure_release_time {:.6} sec", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(5.0)
                    .set(ids.cylinder_pressure_release_factor, ui)
                {
                    let new_val = (1.0 - value).powf(1.0 / SAMPLE_RATE as f32);

                    if (cylinder.pressure_release_factor - new_val).abs() > 1E-12 {
                        changed = true;
                        cylinder.pressure_release_factor = new_val;
                    }
                }
            }

            if changed {
                generator.engine.cylinders.clear();
                for i in 0..num_cylinders {
                    let mut cyl = cylinder.clone();
                    cyl.crank_offset = i as f32 / num_cylinders as f32 * (1.0 - growl);
                    generator.engine.cylinders.push(cyl);
                }
            }
        }

        /*
                // $1
                {
                    const MIN: f32 = 0.0;
                    const MAX:f32 = 1.0;
                    let prev_val = generator.engine.$1;
                    for value in widget::Slider::new(prev_val, MIN, MAX)
                        .label(format!("$1 {:.2}", prev_val).as_str())
                        .label_font_size(12)
                        .padded_w_of(ids.canvas, PAD)
                        .down(5.0)
                        .set(ids.engine_$1, ui)
                        {
                            generator.engine.$1 = value;
                        }
                }
        */

        {
            let len = generator.gui_graph.len() as f32;
            widget::PlotPath::new(0.0, 1.0, -3.0, 3.0, |x| generator.gui_graph[(x * len) as usize].min(1.0).max(-1.0) * 3.0).set(ids.graph, ui);
        }
    }

    widget::Scrollbar::y_axis(ids.canvas).auto_hide(false).set(ids.canvas_scrollbar, ui);
}

fn recording_name() -> String {
    let time = Local::now();

    format!("enginesound_{:02}{:02}{:04}-{:02}{:02}{:02}.wav", time.day(), time.month(), time.year(), time.hour(), time.minute(), time.second())
}
