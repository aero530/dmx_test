//! UI hardware interface
// use core::default;

use defmt::{error, info, Format};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_stm32::i2c::I2c;
use embassy_time::{with_timeout, Duration};
use embedded_graphics::mono_font::iso_8859_4::FONT_10X20;
use mousefood::{EmbeddedBackend, EmbeddedBackendConfig};
use ratatui_core::backend::Backend;

// use ratatui_core::buffer::Buffer;
// use ratatui_core::layout::{Constraint, Layout, Rect};
// use ratatui_core::text::Line;
// use ratatui_core::widgets::Widget;
// use ratatui_widgets::tabs::Tabs;
use crate::channels::UiChannelRx;
use crate::I2c1Bus;

// use mousefood::prelude::*;

use ratatui_core::terminal::Terminal;
// use ratatui_core::style::*;

// use ratatui_widgets::block::Block;
// use ratatui_widgets::paragraph::{Paragraph, Wrap};


// https://github.com/cschuhen/oled_drivers/blob/master/examples/i2c.rs

use oled_async::{prelude::*, Builder as OledBuilder};
use oled_async::displayrotation::DisplayRotation;


use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};

mod app;
use app::App;

#[derive(Format)]
pub enum UiEvent {
    // On,
    // Off,
    Value([u8;3]),
}


pub struct Ui<B> where B: Backend {
    terminal: Terminal<B>,
    rx: UiChannelRx,
    app: App,
}

impl<B> Ui<B> where B:Backend {
    pub fn new(terminal: Terminal<B>, rx: UiChannelRx) -> Self {
        Self { 
            terminal, 
            rx, 
            app: App::default() 
        }
    }

    pub async fn run(&mut self) {
        // if self.terminal.draw(draw).is_err() {
        if self.terminal.draw(|frame| frame.render_widget(&self.app, frame.area())).is_err() {
            error!("Failed to draw to screen");
        }

        if let Ok(new_message) = with_timeout(Duration::from_millis(50), self.rx.receive()).await {
            self.process_event(new_message).await;
        }
    }

    async fn process_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::Value(_) => {
                // self.channels_a.ch1.set_duty_cycle_fraction(values[0] as u16, 255);
                // self.channels_b.ch3.set_duty_cycle_fraction(values[1] as u16, 255);
                // self.channels_c.ch2.set_duty_cycle_fraction(values[2] as u16, 255);
                // set pwm to value
            }
        }
    }
    
}



#[embassy_executor::task]
// pub async fn ui_task(mut i2c: I2c<'static, embassy_stm32::mode::Async>, rx: UiChannelRx) {
pub async fn ui_task(sm_bus_manager: &'static I2c1Bus, rx: UiChannelRx) {

    let sm_bus_dev = I2cDevice::new(sm_bus_manager);

    type I2cDisplay = I2cDevice<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, I2c<'static, embassy_stm32::mode::Async>>;

    type I2cInterface = display_interface_i2c::I2CInterface<I2cDisplay>;

    let di: I2cInterface = display_interface_i2c::I2CInterface::new(
        sm_bus_dev,  // I2C
        0x3C, // I2C Address 3C or 61
        0x40, // Data byte
    );

    let raw_disp = OledBuilder::new(oled_async::displays::sh1106::Sh1106_128_64 {})
        .with_rotation(DisplayRotation::Rotate0)
        .connect(di);

    let mut disp: GraphicsMode<_, _> = raw_disp.into();

    let a = disp.display_on(true).await;
    match a {
        Ok(()) => info!("Display on"),
        Err(e) => error!("{}",e)
    }

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


    let backend = EmbeddedBackend::new(&mut disp, EmbeddedBackendConfig::default());
    if let Ok(terminal) = Terminal::new(backend) {
        let mut ui = Ui::new(terminal, rx);
        // app.run();
        loop {
            ui.run().await
        }
    } else {
        error!("Unable to create terminal using backend display");
    }
}


