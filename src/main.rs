#![no_std]
#![no_main]

use cortex_m::interrupt::{free as disable_interrupts, CriticalSection};
use cortex_m::peripheral::NVIC;
use embedded_hal::digital::v2::StatefulOutputPin;
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

struct Ctx {
    #[cfg(feature = "gui")]
    app: MainWindow,
    #[cfg(feature = "led")]
    led:
        wio::hal::gpio::Pin<wio::hal::gpio::PA15, wio::hal::gpio::Output<wio::hal::gpio::PushPull>>,
    tc3: wio::hal::timer::TimerCounter3,
}

static mut CTX: Option<Ctx> = None;

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

    let gclk5 = clocks
        .get_gclk(wio::pac::gclk::pchctrl::GEN_A::GCLK5)
        .unwrap();
    let timer_clock = clocks.tc2_tc3(&gclk5).unwrap();
    let mut tc3 =
        wio::hal::timer::TimerCounter::tc3_(&timer_clock, peripherals.TC3, &mut peripherals.MCLK);

    unsafe {
        NVIC::unmask(interrupt::TC3);
    }
    tc3.start(1.secs());
    tc3.enable_interrupt();

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
    let led = sets.user_led.into_push_pull_output();

    #[cfg(feature = "gui")]
    let mut slint_integration = slint_integration::SlintIntegration::new(display);

    #[cfg(feature = "gui")]
    let app = MainWindow::new().expect("Failed to load UI");

    unsafe {
        CTX = Some(Ctx {
            #[cfg(feature = "gui")]
            app,
            #[cfg(feature = "led")]
            led,
            tc3,
        });
    }

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
                Button::Click => "Return",
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

#[interrupt]
fn TC3() {
    unsafe {
        let ctx = CTX.as_mut().unwrap();
        ctx.tc3.wait().unwrap();
        #[cfg(feature = "led")]
        {
            ctx.led.toggle().unwrap();
            #[cfg(feature = "gui")]
            ctx.app.set_led_on(ctx.led.is_set_high().unwrap());
        }
    }
}
