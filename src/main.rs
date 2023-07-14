#![no_std]
#![no_main]

mod display;
mod hid;
mod keypad;

use embedded_hal::spi::MODE_0;
use fugit::RateExtU32;
use panic_halt as _;

use rp_pico::hal::gpio::FunctionSpi as SPI;
use rp_pico::hal::timer::{Instant, Timer};
use rp_pico::hal::{self, pac, Clock, Spi, I2C};

use keypad::Keypad;

static mut TIMER: Option<Timer> = None;

type Duration64 = fugit::Duration<u64, 1, 100000>;

#[rp_pico::entry]
fn main() -> ! {
    let mut keypad = hardware_init();

    loop {
        for (id, event) in keypad.update() {
            if id == 0 && matches!(event, keypad::KeyEvent::Held) {
                for i in 0..16 {
                    keypad.keys[i].pressed_color = keypad::Color::new(255, 0, 0);
                }
            }
            if id == 1 && matches!(event, keypad::KeyEvent::Held) {
                for i in 0..16 {
                    keypad.keys[i].pressed_color = keypad::Color::new(0, 255, 0);
                }
            }
            if id == 2 && matches!(event, keypad::KeyEvent::Held) {
                for i in 0..16 {
                    keypad.keys[i].pressed_color = keypad::Color::new(0, 0, 255);
                }
            }
            if id == 3 && matches!(event, keypad::KeyEvent::Held) {
                rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
            }
            if id == 15 && matches!(event, keypad::KeyEvent::Pressed) {
                keypad.set_brightness(0.1);
            }
            if id == 14 && matches!(event, keypad::KeyEvent::Pressed) {
                keypad.set_brightness(1.0);
            }
        }
    }
}

fn hardware_init() -> Keypad {
    let mut pac = pac::Peripherals::take().unwrap();

    let sio = hal::Sio::new(pac.SIO);

    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    // Safety: we're still in the initialization stage,
    // so there's no race risk.
    unsafe {
        TIMER = Some(timer);
    }

    // I2C for keypad keys
    let i2c = I2C::i2c0(
        pac.I2C0,
        pins.gpio4.into_mode(), // SDA
        pins.gpio5.into_mode(), // SCL
        400.kHz(),
        &mut pac.RESETS,
        125_000_000.Hz(),
    );

    // SPI for keypad LEDs
    let cs = pins.gpio17.into_push_pull_output();
    let _ = pins.gpio18.into_mode::<SPI>();
    let _ = pins.gpio19.into_mode::<SPI>();

    let spi = Spi::<_, _, 8>::new(pac.SPI0).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        4_000_000.Hz(),
        &MODE_0,
    );

    Keypad::new(i2c, spi, cs)
}

/// Get an Instant representing "now."
fn now() -> Instant {
    // Safety: get_counter merely reads from the timer -
    // nothing is mutated.
    unsafe {
        TIMER.as_ref().unwrap().get_counter()
    }
}
