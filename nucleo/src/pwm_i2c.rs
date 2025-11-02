//! LED & Button interaction
use defmt::Format;
// use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
// use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_time::{with_timeout, Duration};
use pwm_pca9685::{Channel, Pca9685};

use embedded_hal_1::i2c::I2c as I2CTRAIT;

use crate::channels::PwmChannelRx;
use crate::I2cSharedDev;

#[derive(Format)]
pub enum PwmEvent {
    // On,
    // Off,
    Value([u8; 3]),
}

// pub struct PwmI2c<I2C: embedded_hal_async::i2c::I2c> {
pub struct PwmI2c<I2C: I2CTRAIT> {
    pwm: Pca9685<I2C>,
    rx: PwmChannelRx,
}

impl<I2C: I2CTRAIT> PwmI2c<I2C> {
    #[allow(unused)]
    pub fn new(i2c: I2C, address: u8, rx: PwmChannelRx) -> Self {
        let pwm = Pca9685::new(i2c, address).unwrap();
        Self { pwm, rx }
    }

    #[allow(unused)]
    pub async fn configure(&mut self) {
        // This corresponds to a frequency of 60 Hz.
        let _ = self.pwm.set_prescale(100);

        // It is necessary to enable the device.
        let _ = self.pwm.enable();

        // turn all channels on
        let _ = self.pwm.set_channel_on(Channel::All, 0);
    }

    #[allow(unused)]
    pub async fn show(&mut self) {
        if let Ok(new_message) = with_timeout(Duration::from_millis(100), self.rx.receive()).await {
            self.process_event(new_message).await;
        }
    }

    #[allow(unused)]
    async fn process_event(&mut self, event: PwmEvent) {
        match event {
            // PwmEvent::On => {
            //     self.enable();
            // }
            // PwmEvent::Off => {
            //     self.disable();
            // }
            PwmEvent::Value(values) => {
                // channels turn on at counter=0.  turn channels off at some other value
                // range is [0..4095] but value comes in as [0..255] so multiply by 16
                self.pwm.set_channel_off(Channel::C0, values[0] as u16 * 16).unwrap();
                self.pwm.set_channel_off(Channel::C1, values[1] as u16 * 16).unwrap();
                self.pwm.set_channel_off(Channel::C2, values[2] as u16 * 16).unwrap();

                self.pwm.set_channel_off(Channel::C5, values[0] as u16 * 16).unwrap();
                self.pwm.set_channel_off(Channel::C6, values[1] as u16 * 16).unwrap();
                self.pwm.set_channel_off(Channel::C7, values[2] as u16 * 16).unwrap();

                // set pwm to value
            }
        }
    }
}

#[embassy_executor::task]
pub async fn pwm_i2c_task(i2c: I2cSharedDev, address: u8, rx: PwmChannelRx) {
    // let i2c_bus_dev = I2cDevice::new(i2c_bus_manager);

    let mut pwm = PwmI2c::new(i2c, address, rx);
    pwm.configure().await;
    loop {
        //for testing only
        pwm.configure().await;
        //

        pwm.show().await;
    }
}
