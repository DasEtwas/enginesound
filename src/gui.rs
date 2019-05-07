use crate::{distance_to_samples, gen::Generator, recorder::Recorder, samples_to_distance, MAX_CYLINDERS, MUFFLER_ELEMENT_COUNT, SAMPLE_RATE, SPEED_OF_SOUND};
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
pub struct Ids {
    pub canvas:                                 widget::Id,
    pub title:                                  widget::Id,
    pub duty_display:                           widget::Id,
    pub record_button:                          widget::Id,
    pub reset_button:                           widget::Id,
    pub engine_rpm_slider:                      widget::Id,
    pub engine_master_volume_slider:            widget::Id,
    pub engine_intake_volume_slider:            widget::Id,
    pub engine_intake_lp_filter_freq:           widget::Id,
    pub engine_exhaust_volume_slider:           widget::Id,
    pub engine_engine_vibrations_volume_slider: widget::Id,
    pub engine_title:                           widget::Id,
    pub engine_intake_noise_factor:             widget::Id,
    pub engine_intake_valve_shift:              widget::Id,
    pub engine_exhaust_valve_shift:             widget::Id,
    pub engine_crankshaft_fluctuation_lp_freq:  widget::Id,
    pub engine_crankshaft_fluctuation:          widget::Id,
    pub muffler_title:                          widget::Id,
    pub muffler_straight_pipe_alpha:            widget::Id,
    pub muffler_straight_pipe_beta:             widget::Id,
    pub muffler_straight_pipe_length:           widget::Id,
    pub muffler_element_length:                 Vec<widget::Id>,
    pub cylinder_title:                         widget::Id,
    pub cylinder_offset_growl:                  widget::Id,
    pub cylinder_num:                           widget::Id,
    pub cylinder_crank_offset:                  widget::Id,
    pub cylinder_intake_open_refl:              widget::Id,
    pub cylinder_intake_closed_refl:            widget::Id,
    pub cylinder_exhaust_open_refl:             widget::Id,
    pub cylinder_exhaust_closed_refl:           widget::Id,
    pub cylinder_intake_open_end_refl:          widget::Id,
    pub cylinder_extractor_open_end_refl:       widget::Id,
    pub cylinder_piston_motion_factor:          widget::Id,
    pub cylinder_ignition_factor:               widget::Id,
    pub cylinder_ignition_time:                 widget::Id,
    pub cylinder_pressure_release_factor:       widget::Id,
    pub cylinder_intake_pipe_length:            Vec<widget::Id>,
    pub cylinder_exhaust_pipe_length:           Vec<widget::Id>,
    pub cylinder_extractor_pipe_length:         Vec<widget::Id>,
    pub graph:                                  widget::Id,
    pub canvas_scrollbar:                       widget::Id,
}

// expanded widget_ids! generator macro
impl Ids {
    #[allow(unused_mut, unused_variables)]
    pub fn new(mut generator: widget::id::Generator) -> Self {
        Ids {
            canvas:                                 generator.next(),
            title:                                  generator.next(),
            duty_display:                           generator.next(),
            record_button:                          generator.next(),
            reset_button:                           generator.next(),
            engine_rpm_slider:                      generator.next(),
            engine_master_volume_slider:            generator.next(),
            engine_intake_volume_slider:            generator.next(),
            engine_intake_lp_filter_freq:           generator.next(),
            engine_exhaust_volume_slider:           generator.next(),
            engine_engine_vibrations_volume_slider: generator.next(),
            engine_title:                           generator.next(),
            engine_intake_noise_factor:             generator.next(),
            engine_intake_valve_shift:              generator.next(),
            engine_exhaust_valve_shift:             generator.next(),
            engine_crankshaft_fluctuation_lp_freq:  generator.next(),
            engine_crankshaft_fluctuation:          generator.next(),
            muffler_title:                          generator.next(),
            muffler_straight_pipe_alpha:            generator.next(),
            muffler_straight_pipe_beta:             generator.next(),
            muffler_straight_pipe_length:           generator.next(),
            muffler_element_length:                 (0..MUFFLER_ELEMENT_COUNT).map(|_| generator.next()).collect(),
            cylinder_title:                         generator.next(),
            cylinder_offset_growl:                  generator.next(),
            cylinder_num:                           generator.next(),
            cylinder_crank_offset:                  generator.next(),
            cylinder_intake_open_refl:              generator.next(),
            cylinder_intake_closed_refl:            generator.next(),
            cylinder_exhaust_open_refl:             generator.next(),
            cylinder_exhaust_closed_refl:           generator.next(),
            cylinder_intake_open_end_refl:          generator.next(),
            cylinder_extractor_open_end_refl:       generator.next(),
            cylinder_piston_motion_factor:          generator.next(),
            cylinder_ignition_factor:               generator.next(),
            cylinder_ignition_time:                 generator.next(),
            cylinder_pressure_release_factor:       generator.next(),
            cylinder_intake_pipe_length:            (0..MAX_CYLINDERS).map(|_| generator.next()).collect(),
            cylinder_exhaust_pipe_length:           (0..MAX_CYLINDERS).map(|_| generator.next()).collect(),
            cylinder_extractor_pipe_length:         (0..MAX_CYLINDERS).map(|_| generator.next()).collect(),
            graph:                                  generator.next(),
            canvas_scrollbar:                       generator.next(),
        }
    }
}

/// Instantiate a GUI demonstrating every widget available in conrod.
pub fn gui(ui: &mut conrod_core::UiCell, ids: &Ids, generator: Arc<RwLock<Generator>>) {
    const PAD_TOP: conrod_core::Scalar = 10.0;
    const PAD: conrod_core::Scalar = 30.0;
    const BUTTONWIDTH: conrod_core::Scalar = 700.0;

    widget::Canvas::new().pad(PAD).pad_right(PAD + 20.0).pad_top(0.0).scroll_kids_vertically().set(ids.canvas, ui);
    widget::Scrollbar::y_axis(ids.canvas).auto_hide(true).w(25.0).set(ids.canvas_scrollbar, ui);

    widget::Text::new("Engine Sound Generator")
        .font_size(24)
        .top_left_with_margins(PAD_TOP, PAD)
        .w(ui.win_w - PAD * 2.0)
        .mid_left_of(ids.canvas)
        .set(ids.title, ui);

    {
        let mut generator = generator.write();
        widget::Text::new(format!("Current sampler duty: {:.2}%", generator.sampler_duty * 100.0).as_str()).down(7.0).w(700.0).set(ids.duty_display, ui);

        {
            let (mut button_label, remove_recorder) = match &mut generator.recorder {
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

            if generator.recording_currently_clipping {
                button_label.push_str("   !!Recording clipping!! (decrease master volume)");
            }

            if remove_recorder {
                generator.recorder = None;
            }

            for _press in widget::Button::new().left_justify_label().label(button_label.as_str()).down(7.0).w(BUTTONWIDTH).set(ids.record_button, ui) {
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
            let mut reset_sampler_label = String::from("Reset sampler");

            if generator.waveguides_dampened {
                reset_sampler_label.push_str("   !!Resonances dampened!! (change parameters)");
            }

            for _press in widget::Button::new().left_justify_label().label(reset_sampler_label.as_str()).down(5.0).w(BUTTONWIDTH).set(ids.reset_button, ui) {
                generator.reset();
            }
        }

        {
            let prev_val = generator.get_rpm();
            for value in widget::Slider::new(prev_val, 300.0, 13000.0)
                .label(format!("Engine RPM {:.2}", prev_val).as_str())
                .label_font_size(12)
                .align_left()
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
            // intake_noise_lowpassfilter_freq
            {
                const MIN: f32 = 10.0;
                const MAX: f32 = SAMPLE_RATE as f32;
                let prev_val = generator.engine.intake_noise_lp.get_freq();
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("intake_noise_lowpassfilter_freq {:.2}hz", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(7.0)
                    .skew(10.0)
                    .set(ids.engine_intake_lp_filter_freq, ui)
                {
                    let new = generator.engine.intake_noise_lp.update(value);

                    match new {
                        Some(new) => generator.engine.intake_noise_lp = new,
                        None => (),
                    }
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

            // crankshaft_fluctuation_lowpassfilter_freq
            {
                const MIN: f32 = 10.0;
                const MAX: f32 = SAMPLE_RATE as f32;
                let prev_val = generator.engine.crankshaft_fluctuation_lp.get_freq();
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("crankshaft_fluctuation_lowpassfilter_freq {:.2}hz", prev_val).as_str())
                    .label_font_size(12)
                    .padded_w_of(ids.canvas, PAD)
                    .down(7.0)
                    .skew(10.0)
                    .set(ids.engine_crankshaft_fluctuation_lp_freq, ui)
                {
                    let new = generator.engine.crankshaft_fluctuation_lp.update(value);

                    match new {
                        Some(new) => generator.engine.crankshaft_fluctuation_lp = new,
                        None => (),
                    }
                }
            }
        }

        {
            widget::Text::new("Muffler parameters").font_size(16).down(7.0).w(ui.window_dim()[0] - PAD * 2.0).set(ids.muffler_title, ui);

            // exhaust_pipe_alpha
            {
                const MIN: f32 = -1.0;
                const MAX: f32 = 1.0;
                let prev_val = generator.engine.muffler.straight_pipe.alpha;
                for value in widget::Slider::new(prev_val, MIN, MAX)
                    .label(format!("straight_pipe_alpha {:.2}", prev_val).as_str())
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
                    .label(format!("straight_pipe_beta {:.2}", prev_val).as_str())
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
                    .label(format!("straight_pipe_length {:.2}m", prev_val).as_str())
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

            for (i, mut muffler_element) in generator.engine.muffler.muffler_elements.iter_mut().enumerate() {
                // element_length
                {
                    const MIN: f32 = 0.001;
                    const MAX: f32 = 0.6;
                    let prev_val = samples_to_distance(muffler_element.chamber0.samples.len);
                    for value in widget::Slider::new(prev_val, MIN, MAX)
                        .label(format!("{} / element_length {:.2}m", i + 1, prev_val).as_str())
                        .label_font_size(12)
                        .padded_w_of(ids.canvas, PAD)
                        .down(5.0)
                        .set(ids.muffler_element_length[i], ui)
                    {
                        let new = muffler_element.update(distance_to_samples(value), muffler_element.alpha, muffler_element.beta);

                        match new {
                            Some(new) => {
                                muffler_element.clone_from(&new);
                            },
                            None => (),
                        }
                    }
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
                const MAX: f32 = MAX_CYLINDERS as f32;
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

            // let cylinders = generator.engine.cylinders.clone();

            if changed {
                generator.engine.cylinders.clear();
                for i in 0..num_cylinders {
                    let mut cyl = cylinder.clone();
                    cyl.crank_offset = i as f32 / num_cylinders as f32 * (1.0 - growl);
                    generator.engine.cylinders.push(cyl);
                }
            }

            for (i, mut cyl) in generator.engine.cylinders.iter_mut().enumerate() {
                /*
                exhaust_waveguide: WaveGuide::new(seconds_to_samples(0.7 / speed_of_sound), -1000.0, 0.0),
                intake_waveguide:    WaveGuide::new(seconds_to_samples(0.7 / speed_of_sound), -1000.0, -0.5),
                extractor_waveguide: WaveGuide::new(seconds_to_samples(1.0 / speed_of_sound), 0.0, 0.7),
                */

                // intake_pipe_length
                {
                    const MIN: f32 = 0.0;
                    const MAX: f32 = 1.0;
                    let prev_val = samples_to_distance(cyl.intake_waveguide.chamber0.samples.len);
                    for value in widget::Slider::new(prev_val, MIN, MAX)
                        .label(format!("{} / intake_pipe_length {:.2}m", i + 1, prev_val).as_str())
                        .label_font_size(12)
                        .padded_w_of(ids.canvas, PAD)
                        .down(5.0)
                        .set(ids.cylinder_intake_pipe_length[i], ui)
                    {
                        let new = cyl.intake_waveguide.update(distance_to_samples(value), cyl.intake_waveguide.alpha, cyl.intake_waveguide.beta);

                        match new {
                            Some(new) => cyl.intake_waveguide = new,
                            None => (),
                        }
                    }
                }
                // exhaust_pipe_length
                {
                    const MIN: f32 = 0.0;
                    const MAX: f32 = 1.7;
                    let prev_val = samples_to_distance(cyl.exhaust_waveguide.chamber0.samples.len);
                    for value in widget::Slider::new(prev_val, MIN, MAX)
                        .label(format!("{} / exhaust_pipe_length {:.2}m", i + 1, prev_val).as_str())
                        .label_font_size(12)
                        .padded_w_of(ids.canvas, PAD)
                        .down(5.0)
                        .set(ids.cylinder_exhaust_pipe_length[i], ui)
                    {
                        let new = cyl.exhaust_waveguide.update(distance_to_samples(value), cyl.exhaust_waveguide.alpha, cyl.exhaust_waveguide.beta);

                        match new {
                            Some(new) => cyl.exhaust_waveguide = new,
                            None => (),
                        }
                    }
                }
                // extractor_pipe_length
                {
                    const MIN: f32 = 0.0;
                    const MAX: f32 = 10.0;
                    let prev_val = samples_to_distance(cyl.extractor_waveguide.chamber0.samples.len);
                    for value in widget::Slider::new(prev_val, MIN, MAX)
                        .label(format!("{} / extractor_pipe_length {:.2}m", i + 1, prev_val).as_str())
                        .label_font_size(12)
                        .padded_w_of(ids.canvas, PAD)
                        .down(5.0)
                        .set(ids.cylinder_extractor_pipe_length[i], ui)
                    {
                        let new = cyl.extractor_waveguide.update(distance_to_samples(value), cyl.extractor_waveguide.alpha, cyl.extractor_waveguide.beta);

                        match new {
                            Some(new) => cyl.extractor_waveguide = new,
                            None => (),
                        }
                    }
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
}

fn recording_name() -> String {
    let time = Local::now();

    format!("enginesound_{:02}{:02}{:04}-{:02}{:02}{:02}.wav", time.day(), time.month(), time.year(), time.hour(), time.minute(), time.second())
}
