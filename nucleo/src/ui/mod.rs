//! UI hardware interface

use defmt::{error, info, Format};
use display_interface::AsyncWriteOnlyDataCommand;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_stm32::i2c::I2c;
use embassy_time::{with_timeout, Duration, Timer};

use crate::channels::UiChannelRx;
use crate::ui::app::SelectedTab;
use crate::I2c1Bus;

// https://github.com/cschuhen/oled_drivers/blob/master/examples/i2c.rs

use oled_async::display;
use oled_async::displayrotation::DisplayRotation;
use oled_async::{prelude::*, Builder as OledBuilder};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, iso_8859_4::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};

use embedded_menu::{
    interaction::{Action, Interaction, Navigation},
    Menu, SelectValue,
};

mod app;
use app::App;

#[derive(Copy, Clone, PartialEq, SelectValue)]
pub enum TestEnum {
    A,
    B,
    C,
}

#[derive(Format)]
pub enum UiEvent {
    // Value([u8;3]),
    NextTab,
    PreviousTab,
}

pub struct Ui<DV, DI>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    disp: GraphicsMode<DV, DI>,
    rx: UiChannelRx,
    app: App,
}

impl<DV, DI> Ui<DV, DI>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    pub fn new(disp: GraphicsMode<DV, DI>, rx: UiChannelRx) -> Self {
        Self {
            disp,
            rx,
            app: App::default(),
        }
    }

    pub async fn run(&mut self) {
        let mut menu0 = Menu::build("Tab 0")
            .add_item("Check this 2", false, |b| 30 + b as i32)
            .add_item("Check this 3", TestEnum::A, |b| 40 + b as i32)
            .build();

        let mut menu1 = Menu::build("Tab 1")
            .add_item("Foo", ">", |_| 1)
            .add_section_title("===== Section =====")
            .add_item("Check this 5", false, |b| 30 + b as i32)
            .build();

        let mut menu2 = Menu::build("Tab 2")
            .add_item("Bar", ">", |_| 1)
            .add_item("More stuff", false, |b| 20 + b as i32)
            .build();

        let mut menu3 = Menu::build("Tab 3")
            .add_item("Cat", ">", |_| 1)
            .add_section_title("===== Section =====")
            .build();

        loop {
            self.disp.clear();

            match self.app.current_tab() {
                SelectedTab::Tab0 => {
                    menu0.update(&self.disp);
                    let _ = menu0.draw(&mut self.disp);
                }
                SelectedTab::Tab1 => {
                    menu1.update(&self.disp);
                    let _ = menu1.draw(&mut self.disp).unwrap();
                }
                SelectedTab::Tab2 => {
                    menu2.update(&self.disp);
                    let _ = menu2.draw(&mut self.disp).unwrap();
                }
                SelectedTab::Tab3 => {
                    menu3.update(&self.disp);
                    let _ = menu3.draw(&mut self.disp).unwrap();
                }
            }

            let _ = self.disp.flush().await; // unwrap
                                             // self.app.render();

            if let Ok(new_message) =
                with_timeout(Duration::from_millis(250), self.rx.receive()).await
            {
                self.process_event(new_message).await;
            }
        }
    }

    async fn process_event(&mut self, event: UiEvent) {
        match event {
            // UiEvent::Value(_) => {
            //     // self.channels_a.ch1.set_duty_cycle_fraction(values[0] as u16, 255);
            //     // self.channels_b.ch3.set_duty_cycle_fraction(values[1] as u16, 255);
            //     // self.channels_c.ch2.set_duty_cycle_fraction(values[2] as u16, 255);
            //     // set pwm to value
            // },
            UiEvent::NextTab => {
                self.app.goto_next_tab();
            }
            UiEvent::PreviousTab => {
                self.app.goto_previous_tab();
            }
        }
    }
}

#[embassy_executor::task]
// pub async fn ui_task(mut i2c: I2c<'static, embassy_stm32::mode::Async>, rx: UiChannelRx) {
pub async fn ui_task(i2c_bus_manager: &'static I2c1Bus, rx: UiChannelRx) {
    type I2cDisplay = I2cDevice<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::mode::Master>,
    >;
    type I2cInterface = display_interface_i2c::I2CInterface<I2cDisplay>;

    let i2c_bus_dev = I2cDevice::new(i2c_bus_manager);
    let di: I2cInterface = display_interface_i2c::I2CInterface::new(
        i2c_bus_dev, // I2C
        0x3C,        // I2C Address 3C or 61
        0x40,        // Data byte
    );
    let raw_disp = OledBuilder::new(oled_async::displays::sh1106::Sh1106_128_64 {})
        .with_rotation(DisplayRotation::Rotate0)
        .connect(di);

    let mut disp: GraphicsMode<_, _> = raw_disp.into();
    Timer::after_millis(50).await;

    let a = disp.display_on(true).await;
    match a {
        Ok(()) => info!("Display on"),
        Err(e) => error!("{}", e),
    }

    // Timer::after_millis(50).await;
    //     let a = disp.set_rotation(DisplayRotation::Rotate0).await;
    //     match a {
    //         Ok(()) => info!("Display rotated"),
    //         Err(e) => error!("{}",e)
    //     }
    // Timer::after_millis(50).await;

    // let _ = disp.init().await; // unwrap
    // let _ = disp.flush().await; // unwrap
    // disp.clear();
    // let _ = disp.flush().await; // unwrap
    // info!("clear");
    // let text_style = MonoTextStyleBuilder::new()
    //     .font(&FONT_10X20)
    //     .text_color(BinaryColor::On)
    //     .build();
    // Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
    //     .draw(&mut disp)
    //     .unwrap();
    // disp.set_pixel(0, 0, 255); // top left
    // disp.set_pixel(127, 63, 255); // bottom right
    // disp.set_pixel(64, 0, 255);
    // disp.set_pixel(64, 63, 255);
    // disp.set_pixel(0, 32, 255);
    // disp.set_pixel(127, 32, 255);
    // let _ = disp.flush().await; // unwrap
    // info!("hello");

    let mut ui = Ui::new(disp, rx);
    ui.run().await
}
