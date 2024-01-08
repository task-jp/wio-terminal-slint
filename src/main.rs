#![no_std]
#![no_main]

use cortex_m::interrupt::{free as disable_interrupts, CriticalSection};
use heapless::{consts::U8, spsc::Queue};
use panic_halt as _;
use wio::hal::clock::GenericClockController;
use wio::hal::delay::Delay;
use wio::pac::{interrupt, CorePeripherals, Peripherals};
use wio::prelude::*;
use wio::{button_interrupt, Button, ButtonController, ButtonEvent};
use wio::{entry, Pins};
use wio_terminal as wio;

#[cfg(feature = "gui")]
mod slint_integration;
#[cfg(feature = "gui")]
slint::include_modules!();

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut delay = Delay::new(core.SYST, &mut clocks);
    let sets = Pins::new(peripherals.PORT).split();

    // TODO: use the timer to update animations
    // let timer = wio::hal::Timer::new(pac.TIMER, &mut pac.RESETS);

    let (display, _backlight) = sets
        .display
        .init(
            &mut clocks,
            peripherals.SERCOM7,
            &mut peripherals.MCLK,
            58.MHz(),
            &mut delay,
        )
        .unwrap();

    let button_ctrlr = sets
        .buttons
        .init(peripherals.EIC, &mut clocks, &mut peripherals.MCLK);
    let nvic = &mut core.NVIC;
    disable_interrupts(|_| unsafe {
        button_ctrlr.enable(nvic);
        BUTTON_CTRLR = Some(button_ctrlr);
    });

    let mut consumer = unsafe { Q.split().1 };

    #[cfg(feature = "led")]
    let mut user_led = sets.user_led.into_push_pull_output();
 
    #[cfg(feature = "gui")]
    let mut slint_integration = slint_integration::SlintIntegration::new(display);

    #[cfg(feature = "gui")]
    let app = MainWindow::new().expect("Failed to load UI");

    loop {
        #[cfg(feature = "gui")]
        slint_integration.draw();

        if let Some(button_event) = consumer.dequeue() {
            let key = match button_event.button {
                Button::TopLeft => "F1",
                Button::TopMiddle => "F2",
                // Button::TopRight => "F3", // not supported
                Button::Up => "Up",
                Button::Left => "Left",
                Button::Right => "Right",
                Button::Down => "Down",
                Button::Click => {
                    #[cfg(feature = "led")]
                    {
                        user_led.toggle().unwrap();
                        #[cfg(feature = "gui")]
                        app.set_led_on(button_event.down);
                    }
                    "Return"
                },
            };
            #[cfg(feature = "gui")]
            slint_integration.button_event(key, button_event.down);
        }

        #[cfg(feature = "gui")]
        if slint_integration.has_active_animations() {
            continue;
        }

        // TODO: we could save battery here by going to sleep up to
        //   slint::platform::duration_until_next_timer_update()
        // or until the next touch interrupt, whatever comes first
        // cortex_m::asm::wfe();
    }
}

static mut BUTTON_CTRLR: Option<ButtonController> = None;
static mut Q: Queue<ButtonEvent, U8> = Queue(heapless::i::Queue::new());

button_interrupt!(
    BUTTON_CTRLR,
    unsafe fn on_button_event(_cs: &CriticalSection, event: ButtonEvent) {
        let mut q = Q.split().0;
        q.enqueue(event).ok();
    }
);
