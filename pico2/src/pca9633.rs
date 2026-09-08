//! PCA9633DP1 4-channel I²C LED driver — TFT backlight PWM.
//!
//! LED0 drives the panel's `BL` input (J4 pin 8); LED1–3 go to the J8 header as
//! spare PWM outputs. The DP1 (TSSOP-8) package has a fixed address and no
//! `OE` pin, so there is nothing to strap and nothing to gate.
//!
//! Two register settings matter for this board and are easy to get wrong:
//!
//! * `MODE2.OUTDRV = 1` — totem-pole outputs. The default is open-drain, which
//!   can only *sink*; a logic-level backlight enable needs to be driven high.
//! * The part powers up with every output **off**, so a firmware that never
//!   touches it has a working panel behind a black screen.

use embedded_hal_async::i2c::I2c;

/// Fixed by the DP1 package.
pub const ADDR: u8 = common::BACKLIGHT_ADDRESS;

const REG_MODE1: u8 = 0x00;
const REG_MODE2: u8 = 0x01;
const REG_PWM0: u8 = 0x02;
const REG_LEDOUT: u8 = 0x08;

/// Normal mode, no sub-addresses, and no ALLCALL — the part must not answer
/// the 0x70 all-call address on a bus it shares with other devices.
const MODE1_NORMAL: u8 = 0x00;
/// OUTDRV = 1 (totem-pole); DMBLNK = 0 (group control is dimming, unused);
/// OCH = 0 (outputs change on STOP).
const MODE2_TOTEM_POLE: u8 = 0x04;
/// LDR0 = 10b: LED0 follows PWM0. LED1–3 = 00b: off.
const LEDOUT_LED0_PWM: u8 = 0b10;

pub struct Pca9633<I2C> {
    i2c: I2C,
    addr: u8,
}

impl<I2C: I2c> Pca9633<I2C> {
    pub fn new(i2c: I2C, addr: u8) -> Self {
        Self { i2c, addr }
    }

    /// Wake the part, select totem-pole drive, route LED0 to PWM0 and set the
    /// initial brightness.
    pub async fn init(&mut self, brightness: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.addr, &[REG_MODE1, MODE1_NORMAL]).await?;
        self.i2c.write(self.addr, &[REG_MODE2, MODE2_TOTEM_POLE]).await?;
        self.i2c.write(self.addr, &[REG_PWM0, brightness]).await?;
        self.i2c.write(self.addr, &[REG_LEDOUT, LEDOUT_LED0_PWM]).await
    }

    /// Backlight duty, 0 (off) to 255 (full). The menu never asks for 0.
    pub async fn set_brightness(&mut self, duty: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.addr, &[REG_PWM0, duty]).await
    }
}
