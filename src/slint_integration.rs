#[cfg(feature = "gui")]
extern crate alloc;

use alloc::boxed::Box;
use alloc::rc::Rc;
use embedded_alloc::Heap;
use embedded_graphics_core::draw_target::DrawTarget;
use embedded_graphics_core::pixelcolor::Rgb565;
use slint::platform::software_renderer as renderer;
use slint::platform::WindowEvent::{KeyPressed, KeyReleased};

const HEAP_SIZE: usize = 100 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: Heap = Heap::empty();

struct WioTermialBackend {
    window: Rc<renderer::MinimalSoftwareWindow>,
}

impl slint::platform::Platform for WioTermialBackend {
    fn create_window_adapter(
        &self,
    ) -> Result<alloc::rc::Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        core::time::Duration::from_micros(0) // TODO
    }
}
pub struct SlintIntegration<DISPLAY> {
    display: DISPLAY,
    window: Rc<renderer::MinimalSoftwareWindow>,
    buffer: [slint::platform::software_renderer::Rgb565Pixel; 320],
}

impl<DISPLAY> SlintIntegration<DISPLAY> {
    pub fn new(display: DISPLAY) -> Self {
        unsafe {
            ALLOCATOR.init(
                &mut HEAP as *const u8 as usize,
                core::mem::size_of_val(&HEAP),
            )
        }

        let window =
            slint::platform::software_renderer::MinimalSoftwareWindow::new(Default::default());

        slint::platform::set_platform(Box::new(WioTermialBackend {
            window: window.clone(),
        }))
        .expect("backend already initialized");

        let buffer = [slint::platform::software_renderer::Rgb565Pixel(0); 320];
        Self {
            display,
            window,
            buffer,
        }
    }

    pub fn has_active_animations(&self) -> bool {
        self.window.has_active_animations()
    }

    pub fn button_event(&mut self, key: &str, down: bool) {
        let text: slint::platform::Key = match key {
            "F1" => slint::platform::Key::F1,
            "F2" => slint::platform::Key::F2,
            "F3" => slint::platform::Key::F3,
            "Up" => slint::platform::Key::UpArrow,
            "Left" => slint::platform::Key::LeftArrow,
            "Right" => slint::platform::Key::RightArrow,
            "Down" => slint::platform::Key::DownArrow,
            "Return" => slint::platform::Key::Return,
            _ => return,
        };

        let event = if down {
            KeyPressed { text: text.into() }
        } else {
            KeyReleased { text: text.into() }
        };
        self.window.dispatch_event(event);
    }
}

impl<DISPLAY> SlintIntegration<DISPLAY>
where
    DISPLAY: DrawTarget<Color = Rgb565>,
{
    pub fn draw(&mut self) {
        slint::platform::update_timers_and_animations();

        let display = &mut self.display;
        let mut buffer = self.buffer;

        self.window.draw_if_needed(|renderer| {
            use embedded_graphics_core::prelude::Point;
            use embedded_graphics_core::prelude::Size;
            struct DisplayWrapper<'a, DISPLAY>(
                &'a mut DISPLAY,
                &'a mut [slint::platform::software_renderer::Rgb565Pixel],
            );
            impl<DISPLAY: DrawTarget<Color = Rgb565>>
                slint::platform::software_renderer::LineBufferProvider
                for DisplayWrapper<'_, DISPLAY>
            {
                type TargetPixel = slint::platform::software_renderer::Rgb565Pixel;
                fn process_line(
                    &mut self,
                    line: usize,
                    range: core::ops::Range<usize>,
                    render_fn: impl FnOnce(&mut [Self::TargetPixel]),
                ) {
                    let rect = embedded_graphics_core::primitives::Rectangle::new(
                        Point::new(range.start as _, line as _),
                        Size::new(range.len() as _, 1),
                    );
                    render_fn(&mut self.1[range.clone()]);

                    self.0
                        .fill_contiguous(
                            &rect,
                            self.1[range.clone()].iter().map(|p| {
                                embedded_graphics_core::pixelcolor::raw::RawU16::new(p.0).into()
                            }),
                        )
                        .map_err(drop)
                        .unwrap();
                }
            }
            renderer.render_by_line(DisplayWrapper(display, &mut buffer));
        });
    }
}
