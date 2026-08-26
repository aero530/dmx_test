//! TCA9555 16-bit I²C I/O expander.
//!
//! Four register pairs, so a driver rather than a dependency. Port 0 low bits
//! are the front-panel buttons; port 1 bit 0 drives the OLED `RES`. Everything
//! else goes to test points.
//!
//! # Interrupt behaviour
//!
//! `INT` is open-drain and asserts on *any* input change. It clears when the
//! input registers are read — and only for the port actually read. [`inputs`]
//! therefore always reads **both** ports in one auto-incrementing transfer, so
//! `INT` cannot be left latched by a change on the port that was not polled.
//! That is the classic expander bug: an interrupt that never re-arms and a
//! panel that goes dead after one press.
//!
//! # Pull-ups
//!
//! Buttons are wired straight to ground on the assumption that the TCA9555
//! pulls its inputs high internally. **Confirm this on the bench** — if it does
//! not hold, the buttons need external pull-ups and that is a board change, not
//! a firmware one.

use embedded_hal_async::i2c::I2c;

/// Address with A0/A1/A2 strapped to ground. The part answers 0x20–0x27, so up
/// to eight can share the bus; the M24C02 at 0x56 does not collide.
pub const ADDR: u8 = 0x20;

// Register map. Each is a pair: port 0 then port 1.
const REG_INPUT: u8 = 0x00;
const REG_OUTPUT: u8 = 0x02;
const REG_POLARITY: u8 = 0x04;
const REG_CONFIG: u8 = 0x06;

/// Port 0, bits 0..=3 — the front-panel buttons, active low.
pub const BTN_DOWN: u8 = 1 << 0;
pub const BTN_UP: u8 = 1 << 1;
pub const BTN_SELECT: u8 = 1 << 2;
pub const BTN_ESC: u8 = 1 << 3;

/// Port 1, bit 0 — OLED reset. Asserted only at boot, which is why it can live
/// behind an I²C round trip at all.
pub const OLED_RES: u8 = 1 << 0;

/// Direction masks: a 1 bit is an input, a 0 bit is an output.
///
/// Port 0 is entirely inputs (buttons plus spares). On port 1 only `OLED_RES`
/// is an output; the remaining eleven lines stay inputs so a floating test
/// point cannot fight anything.
const CONFIG_PORT0: u8 = 0xFF;
const CONFIG_PORT1: u8 = !OLED_RES;

pub struct Tca9555<I2C> {
    i2c: I2C,
    addr: u8,
    /// Shadow of the output register — the part has no readback of what was
    /// written, only of the pin state.
    out: [u8; 2],
}

impl<I2C: I2c> Tca9555<I2C> {
    pub fn new(i2c: I2C, addr: u8) -> Self {
        Self {
            i2c,
            addr,
            // OLED held in reset until `release_oled_reset` is called.
            out: [0xFF, 0xFF & !OLED_RES],
        }
    }

    /// Set directions and drive the initial output state.
    ///
    /// Outputs are written *before* the direction register so no pin is ever
    /// briefly driven to the wrong level as it turns around.
    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.i2c
            .write(self.addr, &[REG_POLARITY, 0x00, 0x00])
            .await?;
        self.i2c
            .write(self.addr, &[REG_OUTPUT, self.out[0], self.out[1]])
            .await?;
        self.i2c
            .write(self.addr, &[REG_CONFIG, CONFIG_PORT0, CONFIG_PORT1])
            .await
    }

    /// Read both input ports, clearing `INT`.
    pub async fn inputs(&mut self) -> Result<[u8; 2], I2C::Error> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(self.addr, &[REG_INPUT], &mut buf)
            .await?;
        Ok(buf)
    }

    /// Drive one output bit on the given port.
    pub async fn set(&mut self, port: usize, mask: u8, high: bool) -> Result<(), I2C::Error> {
        if high {
            self.out[port] |= mask;
        } else {
            self.out[port] &= !mask;
        }
        self.i2c
            .write(self.addr, &[REG_OUTPUT, self.out[0], self.out[1]])
            .await
    }

    /// Take the OLED out of reset. Held low from `init` so the display sees a
    /// clean edge once its supply has settled.
    pub async fn release_oled_reset(&mut self) -> Result<(), I2C::Error> {
        self.set(1, OLED_RES, true).await
    }
}
