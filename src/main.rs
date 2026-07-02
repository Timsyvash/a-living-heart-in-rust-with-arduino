#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

use panic_halt as _;
use smart_leds::{SmartLedsWrite, RGB8};

const NUM_LEDS: usize = 64;

const HEARTBEAT_LUT: [u8; 40] = [
    10, 60, 150, 255, 200, 90, 20,
    10, 5,
    15, 50, 110, 80, 40, 15, 5,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0
];

const HEART_MASK: [u8; 64] = [
    0, 1, 1, 0, 0, 1, 1, 0,
    1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1,
    0, 1, 1, 1, 1, 1, 1, 0,
    0, 0, 1, 1, 1, 1, 0, 0,
    0, 0, 0, 1, 1, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
];

type NeoPixelPin = arduino_hal::port::Pin<
    arduino_hal::port::mode::Output,
    arduino_hal::hal::port::PH3
>;

#[allow(dead_code)]
pub struct NeoPixelDriver {
    pin: NeoPixelPin,
}

impl NeoPixelDriver {
    pub fn new<M>(pin: arduino_hal::port::Pin<M, arduino_hal::hal::port::PH3>) -> Self
    where
        M: arduino_hal::port::mode::Io
    {
        Self {
            pin: pin.into_output(),
        }
    }

    fn write_byte(&mut self, mut byte: u8) {
        const PORT_H_SRAM: u16 = 0x0102;
        const PIN_MASK: u8 = 1 << 3;

        unsafe {
            core::arch::asm!(
            "mov r26, {port_low}",
            "mov r27, {port_high}",

            "ld {tmp}, X",
            "mov {high}, {tmp}",
            "or {high}, {mask}",
            "mov {low}, {tmp}",
            "com {mask}",
            "and {low}, {mask}",
            "com {mask}",

            "ldi {cnt}, 8",
            "1:",

            "st X, {high}",
            "nop", "nop",

            "sbrs {byte}, 7",
            "st X, {low}",

            "nop", "nop", "nop",
            "nop",

            "st X, {low}",

            "lsl {byte}",
            "dec {cnt}",
            "brne 1b",

            port_low = in(reg) (PORT_H_SRAM & 0xFF) as u8,
            port_high = in(reg) ((PORT_H_SRAM >> 8) & 0xFF) as u8,
            byte = inout(reg) byte => _,
            mask = in(reg) PIN_MASK,
            high = out(reg) _,
            low = out(reg) _,
            tmp = out(reg) _,
            cnt = out(reg_upper) _,
            out("r26") _, out("r27") _,
            options(nomem, nostack)
            );
        }
    }
}

impl SmartLedsWrite for NeoPixelDriver {
    type Error = core::convert::Infallible;
    type Color = RGB8;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        critical_section::with(|_| {
            for item in iterator {
                let color = item.into();
                self.write_byte(color.g);
                self.write_byte(color.r);
                self.write_byte(color.b);
            }
        });

        arduino_hal::delay_us(70);
        Ok(())
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut ws2812 = NeoPixelDriver::new(pins.d6);
    let mut leds = [RGB8::default(); NUM_LEDS];

    let mut lut_index = 0;

    loop {
        let brightness = HEARTBEAT_LUT[lut_index];

        for i in 0..NUM_LEDS {
            if HEART_MASK[i] == 1 {
                leds[i] = RGB8 {
                    r: brightness,
                    g: 0,
                    b: 0,
                };
            } else {
                leds[i] = RGB8 { r: 0, g: 0, b: 0 };
            }
        }

        ws2812.write(leds.iter().cloned()).unwrap();

        lut_index = (lut_index + 1) % HEARTBEAT_LUT.len();

        arduino_hal::delay_ms(30);
    }
}
