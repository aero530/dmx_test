use embedded_hal_async::i2c::I2c;
use defmt::{Format, info, error};
use embassy_time::{with_timeout, Delay, Duration, Timer};

pub const PAGE_SIZE: u8 = 16; // 16 byte page size
const ADDR_BYTES: u8 = 1; // 
const WRITE_TIME_DELAY: u64 = 5; // in ms
// total size 2Kbit -> 256 x 8

/// All possible errors in this crate
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

impl<E> defmt::Format for Error<E> where E: embedded_hal_async::i2c::Error{
    fn format(&self, f: defmt::Formatter) {
        // format the bitfields of the register as struct fields
        match self {
            Error::I2C(e) => match e.kind() {
                embedded_hal_async::i2c::ErrorKind::Bus => defmt::write!(f, "I2C Bus"),
                embedded_hal_async::i2c::ErrorKind::ArbitrationLoss =>  defmt::write!(f, "I2C ArbitrationLoss"),
                embedded_hal_async::i2c::ErrorKind::NoAcknowledge(_no_acknowledge_source) =>  defmt::write!(f, "I2C NoAcknowledge"),
                embedded_hal_async::i2c::ErrorKind::Overrun =>  defmt::write!(f, "I2C Overrun"),
                embedded_hal_async::i2c::ErrorKind::Other =>  defmt::write!(f, "I2C Other"),
                _ => defmt::write!(f, "I2C unknown"),
            }
            Error::TooMuchData => defmt::write!(f, "Too much data"),
            Error::InvalidAddr =>  defmt::write!(f, "Invalid address"),
            Error::Timeout =>  defmt::write!(f, "Timeout"),
            Error::PageBoundary =>  defmt::write!(f, "PageBoundary"),
        }
    }
}

#[derive(Debug)]
pub struct M24x02<I2C> {
    /// The concrete I²C device implementation.
    i2c: I2C,
    /// The I²C device address.
    address: u8,
}

/// Common methods
impl<I2C, E> M24x02<I2C> where I2C: I2c<Error = E> {
    
    pub fn new(i2c: I2C, address: u8) -> Self {
        M24x02 { i2c, address }
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

    pub async fn write_byte_wait(&mut self, memory_address: u8, data: u8) -> Result<(), Error<E>> {
        self.write_byte(memory_address, data).await?;
        Timer::after_millis(WRITE_TIME_DELAY).await;
        self.wait(memory_address).await?;
        Ok(())
    }

    /// Read a single byte from an address.S
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

        let page_boundary = memory_address | (PAGE_SIZE - 1);
        if memory_address + data_len as u8 > page_boundary + 1 {
            // This would actually be supported by the EEPROM but
            // the data in the page would be overwritten
            return Err(Error::PageBoundary);
        }

        // info!("page write mem address {:#X}, data {:#X}", memory_address, data);

        let mut payload: [u8; (ADDR_BYTES + PAGE_SIZE) as usize] = [0; (ADDR_BYTES + PAGE_SIZE) as usize];
        payload[0] = memory_address;

        payload[(ADDR_BYTES as usize)..(ADDR_BYTES as usize + data_len)].copy_from_slice(&data);
        // info!("write page {:#X}", &payload);
        
        self.i2c.write(self.address, &payload).await.map_err(|e| Error::I2C(e))

    }

    pub async fn write_page_wait(&mut self, memory_address: u8, data: &[u8]) -> Result<(), Error<E>> {
        // info!("page write wait mem address {:#X}, data {:#X}", memory_address, data);
        self.write_page(memory_address, data).await?;
        Timer::after_millis(WRITE_TIME_DELAY).await;
        self.wait(memory_address).await?;
        Ok(())
    }

    async fn wait(&mut self,memory_address: u8) -> Result<(), Error<E>> {
        let mut read_temp = [memory_address; 1];
        let mut count = 0;
        // while self.read_byte(memory_address).await.is_err() {
        while self.i2c.read(self.address, &mut read_temp).await.is_err() {
            count += 1;
            if count > 100 {
                return Err(Error::Timeout)
            } else {
                Timer::after_millis(1).await;
            }
        };
        Ok(())
    }

    fn page_size(&self) -> u8 {
        PAGE_SIZE
    }
}

