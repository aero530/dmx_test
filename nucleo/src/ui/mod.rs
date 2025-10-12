//! UI hardware interface

use defmt::{info, Format};
use embassy_stm32::gpio::{Output, OutputOpenDrain};
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_time::Delay;

use embedded_graphics::prelude::WebColors;
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::interface::SpiInterface;
use mipidsi::options::{Orientation, Rotation};
use mipidsi::{models::ST7789, options::ColorInversion, Builder};

use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{iso_8859_4::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::{Point, RgbColor},
    Drawable,
};
use u8g2_fonts::{
    fonts,
    types::{FontColor, HorizontalAlignment, VerticalPosition},
    Font, FontRenderer, U8g2TextStyle,
};

use embassy_time::{with_timeout, Duration};

use static_cell::StaticCell;

use crate::channels::UiChannelRx;
use crate::ui::layout::NextPrev;
use crate::ui::menu_value::ValueType;

use crate::{DISPLAY_HEIGHT, DISPLAY_OFFSET, DISPLAY_WIDTH};

mod menu_item;
use menu_item::MenuItem;

mod menu_tab;
use menu_tab::MenuTab;

mod layout;
pub use layout::{IncDec, SelectionMode, View};

mod types;
pub use types::*;

mod menu_value;
pub use menu_value::MenuValue;

type SpiDisplay = mipidsi::Display<SpiInterface<'static, ExclusiveDevice<Spi<'static, Async>, Output<'static>, embedded_hal_bus::spi::NoDelay>, Output<'static>>, ST7789, Output<'static>>;

static SPI_DISP_BUFFER: StaticCell<[u8; 512]> = StaticCell::new();

const COLOR_DEFAULT_TEXT: Rgb565 = Rgb565::WHITE;
const COLOR_MENU_TEXT: Rgb565 = Rgb565::CSS_CORAL;

const COLOR_ITEM_TEXT: Rgb565 = Rgb565::CSS_CORNFLOWER_BLUE;
const COLOR_VALUE_TEXT: Rgb565 = Rgb565::CSS_CORNFLOWER_BLUE;
const COLOR_SELECTED_TEXT: Rgb565 = Rgb565::CSS_BLUE_VIOLET;
const COLOR_EDITING_TEXT: Rgb565 = Rgb565::CSS_DEEP_PINK;

//https://github.com/olikraus/u8g2/wiki/fntlistmono#24-pixel-height
const DEFAULT_FONT: fonts::u8g2_font_inr16_mf = fonts::u8g2_font_inr16_mf;

#[derive(Format)]
pub enum UiEvent {
    Up,
    Down,
    Select,
    Esc,
}

pub struct Ui {
    disp: SpiDisplay,
    rx: UiChannelRx,
    current_tab: usize,
}

impl Ui {
    pub fn new(disp: SpiDisplay, rx: UiChannelRx) -> Self {
        Self { disp, rx, current_tab: 0 }
    }

    fn next_tab(&mut self, menu_len: usize) {
        if self.current_tab == menu_len - 1 {
            self.current_tab = 0;
        } else {
            self.current_tab = self.current_tab.saturating_add(1);
        }
    }

    fn previous_tab(&mut self, menu_len: usize) {
        if self.current_tab == 0 {
            self.current_tab = menu_len - 1;
        } else {
            self.current_tab = self.current_tab.saturating_sub(1)
        }
    }

    pub async fn run(&mut self) {
        // Create a Rectangle from the display's dimensions
        let text_style = U8g2TextStyle::new(DEFAULT_FONT, COLOR_DEFAULT_TEXT);

        let mut menu0_items = [MenuItem::new("DMX", ValueType::Uint3(258), true, Point::zero(), text_style.clone())];

        let mut menu1_items = [
            MenuItem::new("Group Mode", ValueType::SmartLedGrouping(SmartLedGrouping::CombineByPort), true, Point::zero(), text_style.clone()),
            MenuItem::new(
                "LED Mode",
                ValueType::SmartLedGroupingAddressing(SmartLedGroupingAddressing::RGB),
                true,
                Point::zero(),
                text_style.clone(),
            ),
        ];

        let mut menu2_items = [
            MenuItem::new("LEDs on Port 1", ValueType::Uint3(0), true, Point::zero(), text_style.clone()),
            MenuItem::new("LEDs on Port 2", ValueType::Uint3(0), true, Point::zero(), text_style.clone()),
            MenuItem::new("LEDs on Port 3", ValueType::Uint3(0), true, Point::zero(), text_style.clone()),
            MenuItem::new("LEDs on Port 4", ValueType::Uint3(123), true, Point::zero(), text_style.clone()),
        ];

        let mut menu0 = MenuTab::new("Main Menu", &mut menu0_items);
        let mut menu1 = MenuTab::new("LED Settings 1", &mut menu1_items);
        let mut menu2 = MenuTab::new("LED Settings 2", &mut menu2_items);

        menu0.arrange();
        menu1.arrange();
        menu2.arrange();

        let mut menus = [menu0, menu1, menu2];

        menus[self.current_tab].update();
        let _ = menus[self.current_tab].draw(&mut self.disp);

        loop {
            if let Ok(new_message) = with_timeout(Duration::from_millis(250), self.rx.receive()).await {
                self.disp.clear(Rgb565::BLACK).unwrap();
                // self.process_event(new_message).await;
                match new_message {
                    UiEvent::Up => {
                        match menus[self.current_tab].next() {
                            Some(_x) => {}
                            None => self.next_tab(menus.len()),
                        };
                    }
                    UiEvent::Down => {
                        match menus[self.current_tab].previous() {
                            Some(_x) => {}
                            None => self.previous_tab(menus.len()),
                        };
                    }
                    UiEvent::Esc => {
                        if !menus[self.current_tab].editing() {
                            self.next_tab(menus.len());
                        }
                    }
                    UiEvent::Select => {
                        if menus[self.current_tab].editing() {
                            menus[self.current_tab].set_editing(false);
                        } else {
                            menus[self.current_tab].set_editing(true);
                        }
                    }
                };

                menus[self.current_tab].update();
                let _ = menus[self.current_tab].draw(&mut self.disp);
            };
        }

        // loop {
        //     match self.app.current_tab() {
        //         SelectedTab::Tab0 => {
        //             menu0.update(&self.disp);
        //             let _ = menu0.draw(&mut self.disp);
        //         }
        //         SelectedTab::Tab1 => {
        //             menu1.update(&self.disp);
        //             let _ = menu1.draw(&mut self.disp).unwrap();
        //         }
        //         SelectedTab::Tab2 => {
        //             menu2.update(&self.disp);
        //             let _ = menu2.draw(&mut self.disp).unwrap();
        //         }
        //         SelectedTab::Tab3 => {
        //             menu3.update(&self.disp);
        //             let _ = menu3.draw(&mut self.disp).unwrap();
        //         }
        //     }

        //     if let Ok(new_message) =
        //         with_timeout(Duration::from_millis(250), self.rx.receive()).await
        //     {
        //         self.disp.clear(Rgb565::BLACK).unwrap();
        //         // self.process_event(new_message).await;
        //         match new_message {
        //             UiEvent::Up => {
        //                 menu0.interact(Interaction::Navigation(Navigation::Next));
        //             },
        //             UiEvent::Down => {
        //                 menu0.interact(Interaction::Navigation(Navigation::Previous));
        //             },
        //             UiEvent::Next => {
        //                 menu0.interact(Interaction::Navigation(Navigation::Next));
        //             },
        //             UiEvent::Select => {
        //                 menu0.interact(Interaction::Action(Action::Select));
        //             },
        //             UiEvent::NextTab => {
        //                 self.app.goto_next_tab();
        //             },
        //         };
        //     };
        // }
    }

    // async fn process_event(&mut self, event: UiEvent) {
    //     self.menu_event = Some(event);
    //     match event {
    //         UiEvent::Up => {
    //             self.menu_event = Some(event);
    //             self.app.goto_next_tab();
    //         }
    //         UiEvent::Down => {
    //             self.app.goto_previous_tab();
    //         }
    //         UiEvent::Next => {
    //             self.app.goto_previous_tab();
    //         }
    //         UiEvent::Select => {
    //             self.app.current_tab();
    //         },
    //     }
    // }
}

#[embassy_executor::task]
pub async fn ui_task_spi(bus: Spi<'static, Async>, cs: Output<'static>, dc: Output<'static>, reset: Output<'static>, mut backlight: OutputOpenDrain<'static>, rx: UiChannelRx) {
    let spi_device = ExclusiveDevice::new_no_delay(bus, cs).unwrap();

    // let mut buffer = [0_u8; 512];
    let buffer = SPI_DISP_BUFFER.init([0_u8; 512]);

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

    let mut ui = Ui::new(display, rx);
    ui.run().await
}
