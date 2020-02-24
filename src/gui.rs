use crate::{
    distance_to_samples, gen::Generator, recorder::Recorder, samples_to_distance, MAX_CYLINDERS,
    MUFFLER_ELEMENT_COUNT, SPEED_OF_SOUND,
};
use chrono::{Datelike, Local, Timelike};
use conrod_core::{
    position::{Align, Direction, Padding, Relative},
    *,
};
use parking_lot::RwLock;
use std::{fs::File, io::Write, sync::Arc};

// must be 2^n
pub const WATERFALL_WIDTH: u32 = 512;
pub const WATERFALL_HEIGHT: u32 = 50;

/// A set of reasonable stylistic defaults that works for the `gui` below.
pub fn theme() -> conrod_core::Theme {
    conrod_core::Theme {
        name: "".to_owned(),
        padding: Padding::none(),
        x_position: Position::Relative(Relative::Align(Align::Start), None),
        y_position: Position::Relative(Relative::Direction(Direction::Backwards, 20.0), None),
        background_color: conrod_core::color::rgb(0.24, 0.24, 0.26),
        shape_color: conrod_core::color::rgb(0.3, 0.3, 0.31),
        border_color: conrod_core::color::rgb(0.2, 0.2, 0.22),
        border_width: 0.0,
        label_color: conrod_core::color::rgb(0.83, 0.83, 0.89),
        font_id: None,
        font_size_large: 20,
        font_size_medium: 14,
        font_size_small: 10,
        widget_styling: conrod_core::theme::StyleMap::default(),
        mouse_drag_threshold: 0.0,
        double_click_threshold: std::time::Duration::from_millis(400),
    }
}

// Generate a unique `WidgetId` for each widget.
pub struct Ids {
    pub canvas: widget::Id,
    pub title: widget::Id,
    pub record_button: widget::Id,
    pub reset_button: widget::Id,
    pub save_button: widget::Id,
    pub drag_drop_info: widget::Id,
    pub mix_title: widget::Id,
    pub engine_rpm_slider: widget::Id,
    pub engine_master_volume_slider: widget::Id,
    pub engine_intake_volume_slider: widget::Id,
    pub engine_intake_lp_filter_freq: widget::Id,
    pub engine_exhaust_volume_slider: widget::Id,
    pub engine_engine_vibrations_volume_slider: widget::Id,
    pub engine_title: widget::Id,
    pub engine_vibrations_lp_filter_freq: widget::Id,
    pub engine_intake_noise_factor: widget::Id,
    pub engine_intake_valve_shift: widget::Id,
    pub engine_exhaust_valve_shift: widget::Id,
    pub engine_crankshaft_fluctuation_lp_freq: widget::Id,
    pub engine_crankshaft_fluctuation: widget::Id,
    pub muffler_title: widget::Id,
    pub muffler_straight_pipe_alpha: widget::Id,
    pub muffler_straight_pipe_beta: widget::Id,
    pub muffler_straight_pipe_length: widget::Id,
    pub engine_muffler_open_end_refl: widget::Id,
    pub muffler_element_length: Vec<widget::Id>,
    pub cylinder_title: widget::Id,
    pub cylinder_offset_growl: widget::Id,
    pub cylinder_num: widget::Id,
    pub cylinder_intake_open_refl: widget::Id,
    pub cylinder_intake_closed_refl: widget::Id,
    pub cylinder_exhaust_open_refl: widget::Id,
    pub cylinder_exhaust_closed_refl: widget::Id,
    pub cylinder_intake_open_end_refl: widget::Id,
    pub cylinder_extractor_open_end_refl: widget::Id,
    pub cylinder_piston_motion_factor: widget::Id,
    pub cylinder_ignition_factor: widget::Id,
    pub cylinder_ignition_time: widget::Id,
    pub cylinder_pressure_release_factor: widget::Id,
    pub cylinder_intake_pipe_length: Vec<widget::Id>,
    pub cylinder_exhaust_pipe_length: Vec<widget::Id>,
    pub cylinder_extractor_pipe_length: Vec<widget::Id>,
    pub cylinder_crank_offset: Vec<widget::Id>,
    pub waterfall: widget::Id,
    pub canvas_scrollbar: widget::Id,
}

// expanded widget_ids! generator macro
impl Ids {
    #[allow(unused_mut, unused_variables)]
    pub fn new(mut generator: widget::id::Generator) -> Self {
        Ids {
            canvas: generator.next(),
            title: generator.next(),
            record_button: generator.next(),
            reset_button: generator.next(),
            save_button: generator.next(),
            drag_drop_info: generator.next(),
            mix_title: generator.next(),
            engine_rpm_slider: generator.next(),
            engine_master_volume_slider: generator.next(),
            engine_intake_volume_slider: generator.next(),
            engine_intake_lp_filter_freq: generator.next(),
            engine_exhaust_volume_slider: generator.next(),
            engine_engine_vibrations_volume_slider: generator.next(),
            engine_title: generator.next(),
            engine_vibrations_lp_filter_freq: generator.next(),
            engine_intake_noise_factor: generator.next(),
            engine_intake_valve_shift: generator.next(),
            engine_exhaust_valve_shift: generator.next(),
            engine_crankshaft_fluctuation_lp_freq: generator.next(),
            engine_crankshaft_fluctuation: generator.next(),
            muffler_title: generator.next(),
            muffler_straight_pipe_alpha: generator.next(),
            muffler_straight_pipe_beta: generator.next(),
            muffler_straight_pipe_length: generator.next(),
            engine_muffler_open_end_refl: generator.next(),
            muffler_element_length: (0..MUFFLER_ELEMENT_COUNT)
                .map(|_| generator.next())
                .collect(),
            cylinder_title: generator.next(),
            cylinder_offset_growl: generator.next(),
            cylinder_num: generator.next(),
            cylinder_intake_open_refl: generator.next(),
            cylinder_intake_closed_refl: generator.next(),
            cylinder_exhaust_open_refl: generator.next(),
            cylinder_exhaust_closed_refl: generator.next(),
            cylinder_intake_open_end_refl: generator.next(),
            cylinder_extractor_open_end_refl: generator.next(),
            cylinder_piston_motion_factor: generator.next(),
            cylinder_ignition_factor: generator.next(),
            cylinder_ignition_time: generator.next(),
            cylinder_pressure_release_factor: generator.next(),
            cylinder_intake_pipe_length: (0..MAX_CYLINDERS).map(|_| generator.next()).collect(),
            cylinder_exhaust_pipe_length: (0..MAX_CYLINDERS).map(|_| generator.next()).collect(),
            cylinder_extractor_pipe_length: (0..MAX_CYLINDERS).map(|_| generator.next()).collect(),
            cylinder_crank_offset: (0..MAX_CYLINDERS).map(|_| generator.next()).collect(),
            waterfall: generator.next(),
            canvas_scrollbar: generator.next(),
        }
    }
}

/// Contains the waterfall bitmap
pub struct GUIState {
    waterfall: [f32; (WATERFALL_WIDTH * WATERFALL_HEIGHT) as usize],
    input: crossbeam::Receiver<Vec<f32>>,
}

impl GUIState {
    pub fn new(input: crossbeam::Receiver<Vec<f32>>) -> Self {
        GUIState {
            waterfall: [0.07f32; (WATERFALL_WIDTH * WATERFALL_HEIGHT) as usize],
            input,
        }
    }

    fn update(&mut self) {
        while let Ok(new_line) = self.input.try_recv() {
            let log_scale = (0..WATERFALL_WIDTH as usize)
                .map(|i| {
                    let new = (1.0 - (i + 1) as f32 / WATERFALL_WIDTH as f32).log2()
                        / (WATERFALL_WIDTH as f32).recip().log2()
                        * (WATERFALL_WIDTH - 1) as f32;
                    new_line[(new.floor() as usize).saturating_sub(1)] * (1.0 - new.fract())
                        + new_line[new.floor() as usize] * new.fract()
                })
                .collect::<Vec<f32>>();
            self.add_line(&log_scale);
        }
    }

    /// Shift the waterfall down by one and add the new line
    fn add_line(&mut self, line: &[f32]) {
        assert_eq!(
            line.len(),
            WATERFALL_WIDTH as usize,
            "wrong waterfall line width"
        );

        self.waterfall.copy_within(
            0..((WATERFALL_WIDTH * (WATERFALL_HEIGHT - 1)) as usize),
            WATERFALL_WIDTH as usize,
        );
        self.waterfall[..WATERFALL_WIDTH as usize].copy_from_slice(line);
    }
}

/// Draws everything, handles updating parts of the generator and returns the imagemap with a newly updated waterfall
// huge state machine.. ew
#[allow(clippy::cognitive_complexity)]
pub fn gui(
    ui: &mut conrod_core::UiCell,
    ids: &Ids,
    generator: Arc<RwLock<Generator>>,
    gui_state: &mut GUIState,
    display: &glium::Display,
) -> conrod_core::image::Map<glium::texture::Texture2d> {
    const TOP_MARGIN: conrod_core::Scalar = 10.0;
    const MARGIN: conrod_core::Scalar = 15.0;
    const BUTTON_WIDTH: conrod_core::Scalar = 700.0;
    const BUTTON_LINE_SIZE: conrod_core::Scalar = 16.0;
    const DOWN_SPACE: conrod_core::Scalar = 6.0;
    const LABEL_FONT_SIZE: u32 = 10;

    widget::Canvas::new()
        .pad(MARGIN)
        .pad_right(MARGIN + 25.0)
        .pad_top(0.0)
        .scroll_kids_vertically()
        .set(ids.canvas, ui);
    widget::Scrollbar::y_axis(ids.canvas)
        .auto_hide(true)
        .w(20.0)
        .set(ids.canvas_scrollbar, ui);

    fn mix(x: f32, colors: &[([f32; 3], f32)]) -> [f32; 3] {
        let colors = colors
            .windows(2)
            .find(|colors| {
                let (_, start) = colors[0];
                let (_, end) = colors[1];
                start <= x && x < end
            })
            .expect("invalid color mix range");

        let (low_color, low) = colors[0];
        let (high_color, high) = colors[1];

        let ratio = (x - low) / (high - low);
        [
            low_color[0] + (high_color[0] - low_color[0]) * ratio,
            low_color[1] + (high_color[1] - low_color[1]) * ratio,
            low_color[2] + (high_color[2] - low_color[2]) * ratio,
        ]
    }

    let image_map = {
        // receives (maybe) new FFT data
        gui_state.update();

        let raw_image = glium::texture::RawImage2d::from_raw_rgb_reversed(
            gui_state
                .waterfall
                .iter()
                .flat_map(|x| {
                    let color = mix(
                        x.max(0.0).min(10.0),
                        &[
                            ([0.0, 0.0, 0.0], 0.0),
                            ([0.0, 0.2, 0.23], 0.21),
                            ([0.0, 0.3, 0.6], 0.325),
                            ([0.51, 0.36, 1.0], 0.44),
                            ([1.0, 0.55, 0.0], 0.69),
                            ([1.0, 0.86, 0.69], 0.85),
                            ([1.0, 1.0, 1.0], 1.0),
                            ([1.0, 1.0, 1.0], 10.01),
                        ],
                    );

                    color
                        .to_vec()
                        .into_iter()
                        .map(|x| (x.max(0.0).min(1.0) * 255.0) as u8)
                })
                .collect::<Vec<_>>()
                .as_slice(),
            (WATERFALL_WIDTH, WATERFALL_HEIGHT),
        );

        let mut image_map = conrod_core::image::Map::<glium::texture::Texture2d>::new();
        let waterfall_image_id =
            image_map.insert(glium::texture::Texture2d::new(display, raw_image).unwrap());

        widget::Image::new(waterfall_image_id)
            .mid_top_with_margin(TOP_MARGIN)
            .mid_left_of(ids.canvas)
            .w(BUTTON_WIDTH)
            .h(140.0)
            .set(ids.waterfall, ui);

        image_map
    };

    {
        let mut generator = generator.write();
        let sample_rate = generator.samples_per_second;

        {
            let (mut button_label, remove_recorder) = match &mut generator.recorder {
                None => ("Start recording".to_string(), false),
                Some(recorder) => {
                    if recorder.is_running() {
                        ui.needs_redraw();
                        (
                            format!(
                                "Stop recording [{:.3} sec recorded]",
                                recorder.get_len() as f32 / sample_rate as f32
                            ),
                            false,
                        )
                    } else {
                        ("Start recording".to_string(), true)
                    }
                }
            };

            if generator.recording_currently_clipping {
                button_label.push_str("   !!Recording clipping!! (decrease master volume)");
            }

            if remove_recorder {
                generator.recorder = None;
            }

            for _press in widget::Button::new()
                .label(button_label.as_str())
                .down(DOWN_SPACE + 2.0)
                .w(BUTTON_WIDTH)
                .h(BUTTON_LINE_SIZE)
                .set(ids.record_button, ui)
            {
                let sample_rate = sample_rate;
                match &mut generator.recorder {
                    None => {
                        generator.recorder = Some(Recorder::new(recording_name(), sample_rate));
                    }
                    Some(recorder) => {
                        recorder.stop();
                    }
                }
            }
        }

        {
            let mut reset_sampler_label = String::from("Reset sampler");

            if generator.waveguides_dampened {
                reset_sampler_label.push_str("   !!Resonances dampened!! (change parameters)");
            }

            for _press in widget::Button::new()
                .left_justify_label()
                .label(reset_sampler_label.as_str())
                .down(DOWN_SPACE)
                .w(BUTTON_WIDTH)
                .h(BUTTON_LINE_SIZE)
                .set(ids.reset_button, ui)
            {
                generator.reset();
            }
        }
        // save
        {
            for _press in widget::Button::new()
                .left_justify_label()
                .label("Save")
                .down(DOWN_SPACE)
                .w(BUTTON_WIDTH)
                .h(BUTTON_LINE_SIZE)
                .set(ids.save_button, ui)
            {
                let pretty = ron::ser::PrettyConfig {
                    depth_limit: 6,
                    separate_tuple_members: true,
                    enumerate_arrays: true,
                    ..ron::ser::PrettyConfig::default()
                };

                match ron::ser::to_string_pretty(&generator.engine, pretty) {
                    Ok(s) => {
                        let name = config_name();
                        match File::create(&name) {
                            Ok(mut file) => {
                                file.write_all(s.as_bytes()).unwrap();

                                println!("Successfully saved engine config \"{}\"", &name);
                            }
                            Err(e) => {
                                eprintln!("Failed to create file for saving engine config: {}", e)
                            }
                        }
                    }
                    Err(e) => eprintln!("Failed to save engine config: {}", e),
                }
            }

            widget::Text::new("Drop a file into the window to load an enginesound config (.esc)")
                .font_size(12)
                .down(DOWN_SPACE)
                .w(ui.window_dim()[0] - MARGIN * 2.0)
                .set(ids.drag_drop_info, ui);

            widget::Text::new("Mix")
                .font_size(16)
                .down(DOWN_SPACE)
                .w(ui.window_dim()[0] - MARGIN * 2.0)
                .set(ids.mix_title, ui);
        }

        {
            let prev_val = generator.get_rpm();
            if let Some(value) = widget::Slider::new(prev_val, 300.0, 13000.0)
                .label(format!("Engine RPM {:.2} ({:.1} hz)", prev_val, prev_val / 60.0).as_str())
                .label_font_size(LABEL_FONT_SIZE)
                .align_left()
                .padded_w_of(ids.canvas, MARGIN)
                .down(DOWN_SPACE)
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
                if let Some(value) = widget::Slider::new(prev_val, 0.0, 3.0)
                    .label(format!("Master volume {:.0}%", prev_val * 100.0).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.engine_master_volume_slider, ui)
                {
                    generator.set_volume(value);
                }
            }

            {
                let prev_val = generator.get_intake_volume();
                if let Some(value) = widget::Slider::new(prev_val, 0.0, 1.0)
                    .label(format!("Intake volume {:.0}%", prev_val * 100.0).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
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
                if let Some(value) = widget::Slider::new(prev_val, 0.0, 1.0)
                    .label(format!("Exhaust volume {:.0}%", prev_val * 100.0).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
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
                if let Some(value) = widget::Slider::new(prev_val, 0.0, 1.0)
                    .label(format!("Engine vibrations volume {:.0}%", prev_val * 100.0).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
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

        widget::Text::new("Engine parameters")
            .font_size(16)
            .down(DOWN_SPACE)
            .w(ui.window_dim()[0] - MARGIN * 2.0)
            .set(ids.engine_title, ui);

        {
            // engine_vibrations_lowpassfilter_freq
            {
                const MIN: f32 = 10.0;
                let max: f32 = sample_rate as f32;
                let prev_val = generator
                    .engine
                    .engine_vibration_filter
                    .get_freq(sample_rate);
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!(
                            "Engine vibrations Lowpass-Filter Frequency {:.2}hz",
                            prev_val
                        )
                        .as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .skew(10.0)
                    .set(ids.engine_vibrations_lp_filter_freq, ui)
                {
                    let new = generator
                        .engine
                        .engine_vibration_filter
                        .get_changed(value, sample_rate);

                    if let Some(new) = new {
                        generator.engine.engine_vibration_filter = new;
                    }
                }
            }
            // intake_noise_factor
            {
                const MIN: f32 = 0.0;
                let max: f32 = 3.0;
                let prev_val = generator.engine.intake_noise_factor;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(format!("Intake noise volume {:.2}", prev_val).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.engine_intake_noise_factor, ui)
                {
                    generator.engine.intake_noise_factor = value;
                }
            }
            // intake_noise_lowpassfilter_freq
            {
                const MIN: f32 = 10.0;
                let max: f32 = sample_rate as f32;
                let prev_val = generator.engine.intake_noise_lp.get_freq(sample_rate);
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!("Intake noise Lowpass-Filter Frequency {:.2}hz", prev_val).as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .skew(10.0)
                    .set(ids.engine_intake_lp_filter_freq, ui)
                {
                    let new = generator
                        .engine
                        .intake_noise_lp
                        .get_changed(value, sample_rate);

                    if let Some(new) = new {
                        generator.engine.intake_noise_lp = new;
                    }
                }
            }
            // intake_valve_shift
            {
                const MIN: f32 = -0.5;
                let max: f32 = 0.5;
                let prev_val = generator.engine.intake_valve_shift;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(format!("Intake valve cam shift {:.2} cycles", -prev_val).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.engine_intake_valve_shift, ui)
                {
                    generator.engine.intake_valve_shift = value;
                }
            }
            // exhaust_valve_shift
            {
                const MIN: f32 = -0.5;
                let max: f32 = 0.5;
                let prev_val = generator.engine.exhaust_valve_shift;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(format!("Exhaust valve cam shift {:.2} cycles", -prev_val).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.engine_exhaust_valve_shift, ui)
                {
                    generator.engine.exhaust_valve_shift = value;
                }
            }

            // crankshaft_fluctuation
            {
                const MIN: f32 = 0.0;
                let max: f32 = 2.5; // lower filter frequencies require more amplitude so its noticable
                let prev_val = generator.engine.crankshaft_fluctuation;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(format!("Crankshaft fluctuation factor {:.2}x", prev_val).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.engine_crankshaft_fluctuation, ui)
                {
                    generator.engine.crankshaft_fluctuation = value;
                }
            }

            // crankshaft_fluctuation_lowpassfilter_freq
            {
                const MIN: f32 = 10.0;
                let max: f32 = sample_rate as f32;
                let prev_val = generator
                    .engine
                    .crankshaft_fluctuation_lp
                    .get_freq(sample_rate);
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!(
                            "Crankshaft fluctuation noise Lowpass-Filter frequency {:.2}hz",
                            prev_val
                        )
                        .as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .skew(10.0)
                    .set(ids.engine_crankshaft_fluctuation_lp_freq, ui)
                {
                    let new = generator
                        .engine
                        .crankshaft_fluctuation_lp
                        .get_changed(value, sample_rate);

                    if let Some(new) = new {
                        generator.engine.crankshaft_fluctuation_lp = new;
                    }
                }
            }
        }

        {
            widget::Text::new("Muffler parameters")
                .font_size(16)
                .down(DOWN_SPACE)
                .w(ui.window_dim()[0] - MARGIN * 2.0)
                .set(ids.muffler_title, ui);

            // engine_muffler_straight_pipe_alpha
            {
                const MIN: f32 = -1.0;
                let max: f32 = 1.0;
                let prev_val = generator.engine.muffler.straight_pipe.alpha;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!("Straight Pipe extractor-side reflectivity {:.2}", prev_val)
                            .as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.muffler_straight_pipe_alpha, ui)
                {
                    generator.engine.muffler.straight_pipe.alpha = value;
                }
            }
            // engine_muffler_straight_pipe_beta
            {
                const MIN: f32 = -1.0;
                let max: f32 = 1.0;
                let prev_val = generator.engine.muffler.straight_pipe.beta;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!("Straight Pipe muffler-side reflectivity {:.2}", prev_val).as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.muffler_straight_pipe_beta, ui)
                {
                    generator.engine.muffler.straight_pipe.beta = value;
                }
            }

            // muffler_straight_pipe_length
            {
                const MIN: f32 = 0.1;
                let max: f32 = 3.0;
                let prev_val = generator.engine.muffler.straight_pipe.chamber0.samples.len as f32
                    * SPEED_OF_SOUND
                    / sample_rate as f32;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!(
                            "Straight Pipe length {:.2}m ({:.1}hz sine peak)",
                            prev_val,
                            SPEED_OF_SOUND / prev_val * 2.0
                        )
                        .as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.muffler_straight_pipe_length, ui)
                {
                    let alpha = generator.engine.muffler.straight_pipe.alpha;
                    let beta = generator.engine.muffler.straight_pipe.beta;

                    if let Some(newgen) = generator.engine.muffler.straight_pipe.get_changed(
                        (value / SPEED_OF_SOUND * sample_rate as f32) as usize,
                        alpha,
                        beta,
                        sample_rate,
                    ) {
                        generator.engine.muffler.straight_pipe = newgen;
                    }
                }
            }

            // muffler_open_end_refl
            let mut muffler_elements_beta;
            {
                const MIN: f32 = -1.0;
                let max: f32 = 0.3;
                let prev_val = generator.engine.muffler.muffler_elements[0].beta;
                muffler_elements_beta = prev_val;

                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!(
                            "Muffler elements output-side (exhaust) reflectivity {:.2}x",
                            prev_val
                        )
                        .as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.engine_muffler_open_end_refl, ui)
                {
                    muffler_elements_beta = value;
                }
            }

            for (i, muffler_element) in generator
                .engine
                .muffler
                .muffler_elements
                .iter_mut()
                .enumerate()
            {
                // element_length
                {
                    const MIN: f32 = 0.001;
                    let max: f32 = 0.6;
                    let prev_val =
                        samples_to_distance(muffler_element.chamber0.samples.len, sample_rate);
                    if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                        .label(
                            format!(
                                "{} / Muffler cavity length {:.2}m ({:.1}hz sine peak)",
                                i + 1,
                                prev_val,
                                SPEED_OF_SOUND / prev_val * 2.0
                            )
                            .as_str(),
                        )
                        .label_font_size(LABEL_FONT_SIZE)
                        .padded_w_of(ids.canvas, MARGIN)
                        .down(DOWN_SPACE)
                        .set(ids.muffler_element_length[i], ui)
                    {
                        let new = muffler_element.get_changed(
                            distance_to_samples(value, sample_rate),
                            muffler_element.alpha,
                            muffler_element.beta,
                            sample_rate,
                        );

                        if let Some(new) = new {
                            muffler_element.clone_from(&new);
                        }
                    }
                }
                muffler_element.beta = muffler_elements_beta;
            }
        }

        widget::Text::new("Cylinder parameters")
            .font_size(16)
            .down(DOWN_SPACE)
            .w(ui.window_dim()[0] - MARGIN * 2.0)
            .set(ids.cylinder_title, ui);

        {
            // if a ui element is being changed, the cylinders need to be replaced
            let mut changed = false;
            let mut num_cylinders = generator.engine.cylinders.len();

            {
                const MIN: f32 = 1.0;
                let max: f32 = MAX_CYLINDERS as f32;
                let prev_val = num_cylinders as f32;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(format!("Cylinder count {}", prev_val).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
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
                let max: f32 = 1.0;
                let prev_val = cylinder.intake_open_refl;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!(
                            "Opened intake valve intake-cavity reflectivity {:.2}",
                            prev_val
                        )
                        .as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.cylinder_intake_open_refl, ui)
                {
                    changed = true;
                    cylinder.intake_open_refl = value;
                }
            }
            // intake_closed_refl
            {
                const MIN: f32 = -1.0;
                let max: f32 = 1.0;
                let prev_val = cylinder.intake_closed_refl;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!(
                            "Closed intake valve intake-cavity reflectivity {:.2}",
                            prev_val
                        )
                        .as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.cylinder_intake_closed_refl, ui)
                {
                    changed = true;
                    cylinder.intake_closed_refl = value;
                }
            }
            // exhaust_open_refl
            {
                const MIN: f32 = -1.0;
                let max: f32 = 1.0;
                let prev_val = cylinder.exhaust_open_refl;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!(
                            "Opened exhaust valve exhaust-cavity reflectivity {:.2}",
                            prev_val
                        )
                        .as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.cylinder_exhaust_open_refl, ui)
                {
                    changed = true;
                    cylinder.exhaust_open_refl = value;
                }
            }
            // exhaust_closed_refl
            {
                const MIN: f32 = -1.0;
                let max: f32 = 1.0;
                let prev_val = cylinder.exhaust_closed_refl;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!(
                            "Closed exhaust valve exhaust-cavity reflectivity {:.2}",
                            prev_val
                        )
                        .as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.cylinder_exhaust_closed_refl, ui)
                {
                    changed = true;
                    cylinder.exhaust_closed_refl = value;
                }
            }
            // cylinder_intake_open_end_refl
            {
                const MIN: f32 = -1.0;
                let max: f32 = 1.0;
                let prev_val = cylinder.intake_waveguide.beta;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(format!("Intake-cavity open end reflectivity {:.2}", prev_val).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.cylinder_intake_open_end_refl, ui)
                {
                    changed = true;
                    cylinder.intake_waveguide.beta = value;
                }
            }
            // cylinder_extractor_open_end_refl
            {
                const MIN: f32 = -1.0;
                let max: f32 = 1.0;
                let prev_val = cylinder.extractor_waveguide.beta;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(
                        format!(
                            "Extractor-cavity straight pipe side reflectivity {:.2}",
                            prev_val
                        )
                        .as_str(),
                    )
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.cylinder_extractor_open_end_refl, ui)
                {
                    changed = true;
                    cylinder.extractor_waveguide.beta = value;
                }
            }
            // piston_motion_factor
            {
                const MIN: f32 = 0.0;
                let max: f32 = 5.0;
                let prev_val = cylinder.piston_motion_factor;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(format!("Piston motion volume {:.2}", prev_val).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.cylinder_piston_motion_factor, ui)
                {
                    changed = true;
                    cylinder.piston_motion_factor = value;
                }
            }
            // ignition_factor
            {
                const MIN: f32 = 0.0;
                let max: f32 = 5.0;
                let prev_val = cylinder.ignition_factor;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(format!("Ignition volume {:.2}", prev_val).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.cylinder_ignition_factor, ui)
                {
                    changed = true;
                    cylinder.ignition_factor = value;
                }
            }
            // ignition_time
            {
                const MIN: f32 = 0.0;
                let max: f32 = 1.0;
                let prev_val = cylinder.ignition_time;
                if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                    .label(format!("Ignition time {:.2}", prev_val).as_str())
                    .label_font_size(LABEL_FONT_SIZE)
                    .padded_w_of(ids.canvas, MARGIN)
                    .down(DOWN_SPACE)
                    .set(ids.cylinder_ignition_time, ui)
                {
                    changed = true;
                    cylinder.ignition_time = value;
                }
            }

            if changed {
                // copy all previous waveguides but modify the values that all cylinders have in common

                generator.engine.cylinders = if num_cylinders <= generator.engine.cylinders.len() {
                    let mut new_cylinders = generator.engine.cylinders[0..num_cylinders].to_vec();

                    for cyl in new_cylinders.iter_mut() {
                        cyl.intake_open_refl = cylinder.intake_open_refl;
                        cyl.intake_closed_refl = cylinder.intake_closed_refl;
                        cyl.exhaust_open_refl = cylinder.exhaust_open_refl;
                        cyl.exhaust_closed_refl = cylinder.exhaust_closed_refl;
                        cyl.piston_motion_factor = cylinder.piston_motion_factor;
                        cyl.ignition_factor = cylinder.ignition_factor;
                        cyl.ignition_time = cylinder.ignition_time;
                        cyl.intake_waveguide.beta = cylinder.intake_waveguide.beta;
                        cyl.extractor_waveguide.beta = cylinder.extractor_waveguide.beta;
                    }

                    new_cylinders
                } else {
                    let mut new_cylinders = generator.engine.cylinders.to_vec();

                    for cyl in new_cylinders.iter_mut() {
                        cyl.intake_open_refl = cylinder.intake_open_refl;
                        cyl.intake_closed_refl = cylinder.intake_closed_refl;
                        cyl.exhaust_open_refl = cylinder.exhaust_open_refl;
                        cyl.exhaust_closed_refl = cylinder.exhaust_closed_refl;
                        cyl.piston_motion_factor = cylinder.piston_motion_factor;
                        cyl.ignition_factor = cylinder.ignition_factor;
                        cyl.ignition_time = cylinder.ignition_time;
                        cyl.intake_waveguide.beta = cylinder.intake_waveguide.beta;
                        cyl.extractor_waveguide.beta = cylinder.extractor_waveguide.beta;
                    }

                    // set the last cylinder's crank offset correctly

                    cylinder.crank_offset = (num_cylinders - 1) as f32 / num_cylinders as f32;

                    new_cylinders.push(cylinder);

                    new_cylinders
                };
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
                    let max: f32 = 1.0;
                    let prev_val =
                        samples_to_distance(cyl.intake_waveguide.chamber0.samples.len, sample_rate);
                    if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                        .label(
                            format!("{} / Intake-cavity length {:.2}m", i + 1, prev_val).as_str(),
                        )
                        .label_font_size(LABEL_FONT_SIZE)
                        .padded_w_of(ids.canvas, MARGIN)
                        .down(DOWN_SPACE)
                        .set(ids.cylinder_intake_pipe_length[i], ui)
                    {
                        let new = cyl.intake_waveguide.get_changed(
                            distance_to_samples(value, sample_rate),
                            cyl.intake_waveguide.alpha,
                            cyl.intake_waveguide.beta,
                            sample_rate,
                        );

                        if let Some(new) = new {
                            cyl.intake_waveguide = new;
                        }
                    }
                }
                // exhaust_pipe_length
                {
                    const MIN: f32 = 0.0;
                    let max: f32 = 1.7;
                    let prev_val = samples_to_distance(
                        cyl.exhaust_waveguide.chamber0.samples.len,
                        sample_rate,
                    );
                    if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                        .label(
                            format!("{} / Exhaust-cavity length {:.2}m", i + 1, prev_val).as_str(),
                        )
                        .label_font_size(LABEL_FONT_SIZE)
                        .padded_w_of(ids.canvas, MARGIN)
                        .down(DOWN_SPACE)
                        .set(ids.cylinder_exhaust_pipe_length[i], ui)
                    {
                        let new = cyl.exhaust_waveguide.get_changed(
                            distance_to_samples(value, sample_rate),
                            cyl.exhaust_waveguide.alpha,
                            cyl.exhaust_waveguide.beta,
                            sample_rate,
                        );

                        if let Some(new) = new {
                            cyl.exhaust_waveguide = new;
                        }
                    }
                }
                // extractor_pipe_length
                {
                    const MIN: f32 = 0.0;
                    let max: f32 = 10.0;
                    let prev_val = samples_to_distance(
                        cyl.extractor_waveguide.chamber0.samples.len,
                        sample_rate,
                    );
                    if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                        .label(
                            format!("{} / Extractor-cavity length {:.2}m", i + 1, prev_val)
                                .as_str(),
                        )
                        .label_font_size(LABEL_FONT_SIZE)
                        .padded_w_of(ids.canvas, MARGIN)
                        .down(DOWN_SPACE)
                        .set(ids.cylinder_extractor_pipe_length[i], ui)
                    {
                        let new = cyl.extractor_waveguide.get_changed(
                            distance_to_samples(value, sample_rate),
                            cyl.extractor_waveguide.alpha,
                            cyl.extractor_waveguide.beta,
                            sample_rate,
                        );

                        if let Some(new) = new {
                            cyl.extractor_waveguide = new;
                        }
                    }
                }
                // crank_offset
                {
                    const MIN: f32 = 0.0;
                    let max: f32 = 1.0;
                    let prev_val = cyl.crank_offset;
                    if let Some(value) = widget::Slider::new(prev_val, MIN, max)
                        .label(format!("{} / Crank offset {:.3} cycles", i + 1, prev_val).as_str())
                        .label_font_size(LABEL_FONT_SIZE)
                        .padded_w_of(ids.canvas, MARGIN)
                        .down(DOWN_SPACE)
                        .set(ids.cylinder_crank_offset[i], ui)
                    {
                        cyl.crank_offset = value;
                    }
                }
            }
        }
    }

    image_map
}

fn recording_name() -> String {
    let time = Local::now();

    format!(
        "enginesound_{:02}{:02}{:04}-{:02}{:02}{:02}.wav",
        time.day(),
        time.month(),
        time.year(),
        time.hour(),
        time.minute(),
        time.second()
    )
}

fn config_name() -> String {
    let time = Local::now();

    format!(
        "enginesound_{:02}{:02}{:04}-{:02}{:02}{:02}.esc",
        time.day(),
        time.month(),
        time.year(),
        time.hour(),
        time.minute(),
        time.second()
    )
}
