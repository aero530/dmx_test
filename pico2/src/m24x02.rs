//! Driver for the ST M24C02 2-Kbit (256 x 8) I2C EEPROM.
//!
//! Byte and page writes with acknowledge polling (the device is busy and
//! NAKs its address during the internal write cycle, so [`M24x02::wait`]
//! polls until it responds again).
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

/// EEPROM page size = 16 bytes per page
#[allow(dead_code)]
pub const PAGE_SIZE: u8 = 16;
/// EEPROM number of address bytes to use (1)
#[allow(dead_code)]
const ADDR_BYTES: u8 = 1;
/// Delay (in ms) when writing to EEPROM
#[allow(dead_code)]
const WRITE_TIME_DELAY: u64 = 5;

// total size 2Kbit -> 256 x 8

/// All possible errors in this crate
#[allow(unused)]
#[derive(Debug)]
pub enum Error<E> {
    /// I²C bus error
    I2C(E),
    /// Too much data passed for a write
    TooMuchData,
    /// Memory address is out of range
    InvalidAddr,
    Timeout,
    PageBoundary,
}

impl<E> defmt::Format for Error<E>
where
    E: embedded_hal_async::i2c::Error,
{
    fn format(&self, f: defmt::Formatter) {
        // format the bitfields of the register as struct fields
        match self {
            Error::I2C(e) => match e.kind() {
                embedded_hal_async::i2c::ErrorKind::Bus => defmt::write!(f, "I2C Bus"),
                embedded_hal_async::i2c::ErrorKind::ArbitrationLoss => defmt::write!(f, "I2C ArbitrationLoss"),
                embedded_hal_async::i2c::ErrorKind::NoAcknowledge(_no_acknowledge_source) => defmt::write!(f, "I2C NoAcknowledge"),
                embedded_hal_async::i2c::ErrorKind::Overrun => defmt::write!(f, "I2C Overrun"),
                embedded_hal_async::i2c::ErrorKind::Other => defmt::write!(f, "I2C Other"),
                _ => defmt::write!(f, "I2C unknown"),
            },
            Error::TooMuchData => defmt::write!(f, "Too much data"),
            Error::InvalidAddr => defmt::write!(f, "Invalid address"),
            Error::Timeout => defmt::write!(f, "Timeout"),
            Error::PageBoundary => defmt::write!(f, "PageBoundary"),
        }
    }
}

/// Underlying EEPROM memory chip interface
#[derive(Debug)]
pub struct M24x02<I2C> {
    /// The concrete I2C device implementation.
    i2c: I2C,
    /// The I2C device address.
    address: u8,
}

/// Common methods. `write_byte` / `write_page` without the wait are kept for
/// callers that batch their own acknowledge polling.
#[allow(dead_code)]
impl<I2C, E> M24x02<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Initialize a new interface
    pub fn new(i2c: I2C, address: u8) -> Self {
        M24x02 { i2c, address }
    }

    /// Attempt a wake up: acknowledge-poll the device until it answers.
    ///
    /// (An earlier version first wrote to address 0xFE — a reserved 7-bit
    /// address nothing acknowledges — which only ever produced a NAK.)
    pub async fn refresh_bus(&mut self) -> Result<(), Error<E>> {
        Timer::after_millis(WRITE_TIME_DELAY).await;
        self.wait(0x00).await
    }

    /// Write a single byte in an address.
    ///
    /// After writing a byte, the EEPROM enters an internally-timed write cycle
    /// to the nonvolatile memory.
    /// During this time all inputs are disabled and the EEPROM will not
    /// respond until the write is complete.
    pub async fn write_byte(&mut self, memory_address: u8, data: u8) -> Result<(), Error<E>> {
        // info!("{:#X} to {:#X}", data, memory_address);
        let payload = [memory_address, data];
        self.i2c.write(self.address, &payload).await.map_err(|e| Error::I2C(e))
    }

    /// Write a byte and wait for the EEPROM to finish
    pub async fn write_byte_wait(&mut self, memory_address: u8, data: u8) -> Result<(), Error<E>> {
        self.write_byte(memory_address, data).await?;
        Timer::after_millis(WRITE_TIME_DELAY).await;
        self.wait(memory_address).await?;
        Ok(())
    }

    /// Read a single byte from an address
    pub async fn read_byte(&mut self, memory_address: u8) -> Result<u8, Error<E>> {
        let mut data = [0_u8; 1];
        self.i2c.write_read(self.address, &[memory_address], &mut data).await.map_err(Error::I2C).and(Ok(data[0]))
    }

    /// Read starting in an address as many bytes as necessary to fill the data array provided.
    pub async fn read_data(&mut self, memory_address: u8, data: &mut [u8]) -> Result<(), Error<E>> {
        self.i2c.write_read(self.address, &[memory_address], data).await.map_err(Error::I2C)
    }

    /// Write up to a page starting in an address.
    ///
    /// The maximum amount of data that can be written depends on the page
    /// size of the device and its overall capacity. If too much data is passed,
    /// the error `Error::TooMuchData` will be returned.
    ///
    /// After writing a byte, the EEPROM enters an internally-timed write cycle
    /// to the nonvolatile memory.
    /// During this time all inputs are disabled and the EEPROM will not
    /// respond until the write is complete.
    pub async fn write_page(&mut self, memory_address: u8, data: &[u8]) -> Result<(), Error<E>> {
        let data_len = data.len();
        if data_len == 0 {
            return Ok(());
        } else if data_len > (PAGE_SIZE as usize) {
            return Err(Error::TooMuchData);
        }

        // Compute in usize so addresses on the last page (0xF0..=0xFF) don't overflow u8
        let page_boundary = memory_address as usize | (PAGE_SIZE as usize - 1);
        if memory_address as usize + data_len > page_boundary + 1 {
            // This would actually be supported by the EEPROM but
            // the data in the page would be overwritten
            return Err(Error::PageBoundary);
        }

        let mut payload: [u8; (ADDR_BYTES + PAGE_SIZE) as usize] = [0; (ADDR_BYTES + PAGE_SIZE) as usize];
        payload[0] = memory_address;
        payload[(ADDR_BYTES as usize)..(ADDR_BYTES as usize + data_len)].copy_from_slice(data);

        // Only send the address byte plus `data_len` bytes. Sending the whole
        // fixed-size payload would write PAGE_SIZE bytes to the EEPROM, which
        // wraps around within the page and corrupts bytes outside `data`.
        self.i2c.write(self.address, &payload[..ADDR_BYTES as usize + data_len]).await.map_err(|e| Error::I2C(e))
    }

    /// Write a page of data and wait for the EEPROM to finish.
    pub async fn write_page_wait(&mut self, memory_address: u8, data: &[u8]) -> Result<(), Error<E>> {
        // info!("page write wait mem address {:#X}, data {:#X}", memory_address, data);
        self.write_page(memory_address, data).await?;
        Timer::after_millis(WRITE_TIME_DELAY).await;
        self.wait(memory_address).await?;
        Ok(())
    }

    /// Wait for the EEPROM to finish.  Check every millisecond until the EEPROM is ready again.
    async fn wait(&mut self, memory_address: u8) -> Result<(), Error<E>> {
        let mut read_temp = [memory_address; 1];
        let mut count = 0;

        while self.i2c.read(self.address, &mut read_temp).await.is_err() {
            count += 1;
            if count > 100 {
                return Err(Error::Timeout);
            } else {
                Timer::after_millis(1).await;
            }
        }
        Ok(())
    }
}
