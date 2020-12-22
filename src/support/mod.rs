use conrod_winit::WinitWindow;
use glium::glutin::event::Event;
use glium::glutin::platform::desktop::EventLoopExtDesktop;
use winit::dpi::PhysicalSize;

pub struct GliumDisplayWinitWrapper(pub glium::Display);

impl GliumDisplayWinitWrapper {
    pub fn get(&self) -> &glium::Display {
        &self.0
    }

    pub fn scale_factor(&self) -> f64 {
        self.hidpi_factor() as f64
    }

    pub fn inner_size(&self) -> PhysicalSize<u32> {
        self.0.gl_window().window().inner_size()
    }
}

impl WinitWindow for GliumDisplayWinitWrapper {
    fn get_inner_size(&self) -> Option<(u32, u32)> {
        let s = self.0.gl_window().window().inner_size();

        Some((s.width, s.height))
    }
    fn hidpi_factor(&self) -> f32 {
        self.0
            .gl_window()
            .window()
            .current_monitor()
            .map(|m| m.scale_factor())
            .unwrap_or(1.0) as f32
    }
}

/// In most of the examples the `glutin` crate is used for providing the window context and
/// events while the `glium` crate is used for displaying `conrod_core::render::Primitives` to the
/// screen.
///
/// This `Iterator`-like type simplifies some of the boilerplate involved in setting up a
/// glutin+glium event loop that works efficiently with conrod.
pub struct EventLoop {
    ui_needs_update: bool,
    last_update: std::time::Instant,
    events: Vec<Event<'static, ()>>,
}

impl EventLoop {
    pub fn new() -> Self {
        EventLoop {
            last_update: std::time::Instant::now(),
            ui_needs_update: true,
            events: vec![],
        }
    }

    /// Produce an iterator yielding all available events.
    pub fn next(
        &mut self,
        events_loop: &mut glium::glutin::event_loop::EventLoop<()>,
    ) -> Vec<glium::glutin::event::Event<'static, ()>> {
        // We don't want to loop any faster than 60 FPS, so wait until it has been at least 16ms
        // since the last yield.
        let last_update = self.last_update;
        let sixteen_ms = std::time::Duration::from_millis(16);
        let duration_since_last_update = std::time::Instant::now().duration_since(last_update);
        if duration_since_last_update < sixteen_ms {
            std::thread::sleep(sixteen_ms - duration_since_last_update);
        }

        // Collect all pending events.
        self.events.clear();
        events_loop.run_return(|event, _target, flow| match event {
            glium::glutin::event::Event::MainEventsCleared => {
                *flow = glium::glutin::event_loop::ControlFlow::Exit;
            }
            e => {
                if let Some(se) = e.to_static() {
                    self.events.push(se);
                }
                *flow = glium::glutin::event_loop::ControlFlow::Poll;
            }
        });

        // If there are no events and the `Ui` does not need updating, wait for the next event.
        if self.events.is_empty() && !self.ui_needs_update {
            events_loop.run_return(|event, _target, flow| match event {
                glium::glutin::event::Event::MainEventsCleared => {
                    *flow = glium::glutin::event_loop::ControlFlow::Exit;
                }
                e => {
                    if let Some(se) = e.to_static() {
                        self.events.push(se);
                    }
                    *flow = glium::glutin::event_loop::ControlFlow::Poll;
                }
            });
        }

        self.ui_needs_update = false;
        self.last_update = std::time::Instant::now();

        std::mem::take(&mut self.events)
    }

    /// Notifies the event loop that the `Ui` requires another update whether or not there are any
    /// pending events.
    ///
    /// This is primarily used on the occasion that some part of the `Ui` is still animating and
    /// requires further updates to do so.
    pub fn needs_update(&mut self) {
        self.ui_needs_update = true;
    }
}
