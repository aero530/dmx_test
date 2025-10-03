use core::marker::PhantomData;

use defmt::debug;
use embedded_hal_async::spi::{ErrorType, SpiBus};
use smart_leds_trait::{SmartLedsWriteAsync, RGB8};

const PATTERNS: [u8; 4] = [0b1000_1000, 0b1000_1110, 0b1110_1000, 0b1110_1110];
pub const NUM_LEDS_MAX: usize = 1024; // in theory could be floor(2^16 / 12) = 5461, [2^16 = MAX DMA size, 12 bytes per LED needed]
pub const BYTES_PER_LED: usize = 12; // number of bytes needed to process each LED (4*3)

/// Trait for color order reordering
pub trait OrderedColors {
    fn order(color: RGB8) -> [u8; 3];
}

/// Marker struct for RGB order
pub struct Rgb;

/// Marker struct for GRB order
pub struct Grb;

impl OrderedColors for Rgb {
    fn order(color: RGB8) -> [u8; 3] {
        [color.r, color.g, color.b]
    }
}

impl OrderedColors for Grb {
    fn order(color: RGB8) -> [u8; 3] {
        [color.g, color.r, color.b]
    }
}

/// N = 12 * NUM_LEDS + 1
/// 1 byte added to pad the front of the data package with 0.
/// Some microcontrollers pull MOSI high too early at the beginning of a
/// data transmission if the first bit is high.  This causes color errors
/// in the LEDs. Including a 0 byte at the beginning of the data prevents
/// this errant high signal from impacting the LED data.
pub struct Ws2812<SPI: SpiBus<u8>, C: OrderedColors> {
    spi: SPI,
    data: [u8; NUM_LEDS_MAX * BYTES_PER_LED + 1],
    _color_order: PhantomData<C>,
}

impl<SPI: SpiBus<u8>, C: OrderedColors> Ws2812<SPI, C> {
    /// Create a new WS2812 driver, with the given SPI bus
    /// Colors default to RGB order
    pub fn new(spi: SPI) -> Self {
        Self {
            spi,
            data: [0; NUM_LEDS_MAX * BYTES_PER_LED + 1],
            _color_order: PhantomData,
        }
    }
}

impl<SPI, E, C: OrderedColors> SmartLedsWriteAsync for Ws2812<SPI, C>
where
    SPI: SpiBus<u8, Error = E>,
{
    type Error = E;
    type Color = RGB8;

    async fn write<T, I>(&mut self, iter: T) -> Result<(), <SPI as ErrorType>::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        // STM32H563 pulls MOSI high prior to sending SPI data which messes up the first LED.
        // Here we force an additional 0 byte to hold MOSI low at the start of the data being sent.
        // skip processing the first byte of self.data to ensure it remains 0x00
        for (led_bytes, rgb8) in self.data[1..(NUM_LEDS_MAX * BYTES_PER_LED + 1)]
            .chunks_mut(BYTES_PER_LED)
            .zip(iter)
        {
            let colors = C::order(rgb8.into());
            for (i, mut color) in colors.into_iter().enumerate() {
                for ii in 0..4 {
                    led_bytes[i * 4 + ii] = PATTERNS[((color & 0b1100_0000) >> 6) as usize];
                    color <<= 2;
                }
            }
        }

        // // STM32H563 pulls MOSI high prior to sending SPI data which messes up the first LED.
        // // Here we force an additional 0 byte to hold MOSI low at the start of the data being sent.
        // let mut d: [u8; NUM_LEDS_MAX * BYTES_PER_LED+1] = [0; NUM_LEDS_MAX * BYTES_PER_LED+1];
        // for (i,x) in self.data.iter().enumerate() {
        //     d[i+1]=*x;
        // }

        // self.spi.write(&d).await?;
        self.spi.write(&self.data).await?;
        let blank = [0_u8; 140];
        self.spi.write(&blank).await
    }
}
