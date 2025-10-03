//! UI hardware interface

use defmt::{error, info, Format};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_stm32::gpio::{Input, Level, Output, OutputOpenDrain, OutputType, Pull, Speed};
use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_time::{with_timeout, Delay, Duration, Timer};

use embedded_hal_async::delay::DelayNs;
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::interface::SpiInterface;
use mipidsi::options::{Orientation, Rotation};
use mipidsi::{models::ST7789, options::ColorInversion, Builder};

use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Primitive, PrimitiveStyle, Triangle},
    text::Text,
};

use static_cell::StaticCell;

use crate::channels::UiChannelRx;
use crate::ui::app::SelectedTab;
use crate::I2c1Bus;

use crate::{DISPLAY_HEIGHT, DISPLAY_OFFSET, DISPLAY_WIDTH};

use embedded_graphics::{
    mono_font::{iso_8859_4::FONT_10X20,  MonoTextStyleBuilder},
    // pixelcolor::BinaryColor,
    text::Baseline,
};

use embedded_menu::{
    interaction::{Action, Interaction, Navigation},
    Menu, MenuStyle, SelectValue,
    theme::Theme,
};

mod app;
use app::App;

type SpiDisplay = mipidsi::Display<
    SpiInterface<
        'static,
        ExclusiveDevice<Spi<'static, Async>, Output<'static>, embedded_hal_bus::spi::NoDelay>,
        Output<'static>,
    >,
    ST7789,
    Output<'static>,
>;

static SPI_DISP_BUFFER: StaticCell<[u8; 512]> = StaticCell::new();

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

pub struct Ui {
    disp: SpiDisplay,
    rx: UiChannelRx,
    app: App,
}

#[derive(Clone, Copy)]
struct ExampleTheme;

impl Theme for ExampleTheme {
    type Color = Rgb565;

    fn text_color(&self) -> Self::Color {
        Rgb565::WHITE
    }

    fn selected_text_color(&self) -> Self::Color {
        Rgb565::BLUE
    }

    fn selection_color(&self) -> Self::Color {
        Rgb565::new(51, 255, 51)
    }
}


impl Ui {
    pub fn new(
        disp: SpiDisplay,
        rx: UiChannelRx,
    ) -> Self {
        Self {
            disp,
            rx,
            app: App::default(),
        }
    }

    pub async fn run(&mut self) {
        let mut menu0 = Menu::with_style(
            "   Tab 0",
            MenuStyle::new(ExampleTheme).with_font(&FONT_10X20).with_title_font(&FONT_10X20)
            )
            .add_item("Check this 2", false, |b| 30 + b as i32)
            .add_item("Check this 3", TestEnum::A, |b| 40 + b as i32)
            .build();

        let mut menu1 = Menu::with_style(
            "   Tab 1",
            MenuStyle::new(ExampleTheme).with_font(&FONT_10X20).with_title_font(&FONT_10X20)
            )
            .add_item("Foo", ">", |_| 1)
            .add_section_title("===== Section =====")
            .add_item("Check this 5", false, |b| 30 + b as i32)
            .build();

        let mut menu2 = Menu::with_style(
            "   Tab 2",
            MenuStyle::new(ExampleTheme).with_font(&FONT_10X20).with_title_font(&FONT_10X20)
            )
            .add_item("Bar", ">", |_| 1)
            .add_item("More stuff", false, |b| 20 + b as i32)
            .build();

        let mut menu3 = Menu::with_style(
            "   Tab 3",
            MenuStyle::new(ExampleTheme).with_font(&FONT_10X20).with_title_font(&FONT_10X20)
            )
            .add_item("Cat", ">", |_| 1)
            .add_section_title("===== Section =====")
            .build();


        loop {
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

            if let Ok(new_message) =
                with_timeout(Duration::from_millis(250), self.rx.receive()).await
            {
                self.disp.clear(Rgb565::BLACK).unwrap();
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
pub async fn ui_task_spi(
    bus: Spi<'static, Async>,
    cs: Output<'static>,
    dc: Output<'static>,
    reset: Output<'static>,
    mut backlight: OutputOpenDrain<'static>,
    rx: UiChannelRx,
) {
    let spi_device = ExclusiveDevice::new_no_delay(bus, cs).unwrap();

    // let mut buffer = [0_u8; 512];
    let mut buffer = SPI_DISP_BUFFER.init([0_u8; 512]);

    // Define the display interface with no chip select
    let di = SpiInterface::new(spi_device, dc, buffer);

    let mut delay = Delay;

    // Define the display from the display interface and initialize it
    let mut display = Builder::new(ST7789, di)
        .reset_pin(reset)
        .display_size(DISPLAY_WIDTH, DISPLAY_HEIGHT)
        .display_offset(DISPLAY_OFFSET, 0)
        .orientation(Orientation::new().rotate(Rotation::Deg90))
        .invert_colors(ColorInversion::Inverted)
        .init(&mut delay)
        .unwrap();

    // Make the display all black
    display.clear(Rgb565::BLACK).unwrap();

    // Turn on backlight
    backlight.set_high();

    // // Draw a smiley face with white eyes and a red mouth
    // draw_smiley(&mut display).unwrap();

    let mut ui = Ui::new(display, rx);
    ui.run().await
}
