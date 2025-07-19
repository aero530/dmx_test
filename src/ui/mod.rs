//! UI hardware interface
// use core::default;

use defmt::{Format, error};
use embassy_stm32::i2c::I2c;
use embassy_time::{with_timeout, Duration};
use mousefood::{EmbeddedBackend, EmbeddedBackendConfig};
use ratatui_core::backend::Backend;
// use ratatui_core::buffer::Buffer;
// use ratatui_core::layout::{Constraint, Layout, Rect};
// use ratatui_core::text::Line;
// use ratatui_core::widgets::Widget;
// use ratatui_widgets::tabs::Tabs;
use crate::channels::UiChannelRx;

// use mousefood::prelude::*;

use ratatui_core::terminal::Terminal;
// use ratatui_core::style::*;

// use ratatui_widgets::block::Block;
// use ratatui_widgets::paragraph::{Paragraph, Wrap};


// https://github.com/cschuhen/oled_drivers/blob/master/examples/i2c.rs

use oled_async::{prelude::*, Builder as OledBuilder};
use oled_async::displayrotation::DisplayRotation;

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
        Self { terminal, rx, app: App::default() }
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
pub async fn ui_task(i2c: I2c<'static, embassy_stm32::mode::Async>, rx: UiChannelRx) {
    type I2cDisplay = embassy_stm32::i2c::I2c<
        'static,
        embassy_stm32::mode::Async,
    >;

    type I2cInterface = display_interface_i2c::I2CInterface<I2cDisplay>;
    let di: I2cInterface = display_interface_i2c::I2CInterface::new(
        i2c,  // I2C
        0x3C, // I2C Address
        0x40, // Data byte
    );

    let raw_disp = OledBuilder::new(oled_async::displays::sh1107::Sh1107_64_128 {})
        .with_rotation(DisplayRotation::Rotate180)
        .connect(di);

    let mut disp: GraphicsMode<_, _> = raw_disp.into();

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


