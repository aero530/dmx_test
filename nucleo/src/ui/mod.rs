//! UI hardware interface

use defmt::{info, error, Format};
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

use crate::channels::{RouterChannelTx, UiChannelRx};
use crate::ui::layout::NextPrev;
use crate::ui::menu_value::ValueType;
use crate::event_router::RouterEvent;

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

#[derive(Clone, Copy, PartialEq, Eq, Format)]
pub struct MenuTabData {
    tab0: [(&'static str, ValueType); 5],
    tab1: [(&'static str, ValueType); 5],
    tab2: [(&'static str, ValueType); 5],
}

impl From<MenuData> for MenuTabData {
    fn from(source: MenuData) -> Self {
        let (tab1, tab2) = match source.module {
            ModuleSettings::Pwm(pwm_settings) => {
                (
                    [
                        ("", ValueType::None),
                        ("", ValueType::None),
                        ("", ValueType::None),
                        ("", ValueType::None),
                        ("", ValueType::None),
                    ],
                    [
                        ("", ValueType::None),
                        ("", ValueType::None),
                        ("", ValueType::None),
                        ("", ValueType::None),
                        ("", ValueType::None),
                    ]
                )
            },
            ModuleSettings::SmartLed(smart_led_settings) => {
                (
                    [
                        ("Group Mode", ValueType::SmartLedGrouping(SmartLedGrouping::CombineByPort)),
                        ("LED Mode", ValueType::SmartLedColorMode(SmartLedColorMode::RGB)),
                        ("", ValueType::None),
                        ("", ValueType::None),
                        ("", ValueType::None),
                    ],
                    [
                        ("LEDs on Port 1", ValueType::Uint3(0)),
                        ("LEDs on Port 2", ValueType::Uint3(0)),
                        ("LEDs on Port 3", ValueType::Uint3(0)),
                        ("LEDs on Port 4", ValueType::Uint3(0)),
                        ("", ValueType::None),
                    ]
                )
            },
        };

        MenuTabData {
            tab0: [
                ("DMX", ValueType::Uint3(source.dmx_address)),
                ("", ValueType::None),
                ("", ValueType::None),
                ("", ValueType::None),
                ("", ValueType::None),
            ],
            tab1,
            tab2,
        }

    }
}

impl MenuTabData {
    fn to_menu_data(&self, module_type: ModuleType) -> MenuData {
        let module_settings = match module_type {
            ModuleType::Pwm => {
                ModuleSettings::Pwm(
                    PwmSettings {
                        freq: todo!(),
                    }
                )
            },
            ModuleType::SmartLed => {
                ModuleSettings::SmartLed(
                    SmartLedSettings {
                        color_mode: self.tab1[0].1.extract_led_addressing().unwrap_or_default(),
                        grouping: self.tab1[1].1.extract_led_grouping().unwrap_or_default(),
                        leds_per_port: [
                            self.tab2[0].1.extract_uint3().unwrap_or_default(),
                            self.tab2[1].1.extract_uint3().unwrap_or_default(),
                            self.tab2[2].1.extract_uint3().unwrap_or_default(),
                            self.tab2[3].1.extract_uint3().unwrap_or_default(),
                        ],
                    }
                )
            },
            ModuleType::Unknown => {
                todo!()
            }
        };
        
        MenuData {
            dmx_address: self.tab0[0].1.extract_uint3().unwrap_or_default(),
            module: module_settings,
        }
    }
}



#[derive(Format)]
pub enum UiEvent {
    Up,
    Down,
    Select,
    Esc,
    Load(MenuData),
}

pub struct Ui {
    disp: SpiDisplay,
    rx: UiChannelRx,
    tx: RouterChannelTx,
    current_tab: usize,
    menu_tab_data: MenuTabData,
    module_type: ModuleType,
}

impl Ui {
    pub fn new(disp: SpiDisplay, rx: UiChannelRx, tx: RouterChannelTx) -> Self {
        let md = MenuData::default(); // should be coming from the inputs to new fn
        let menu_tab_data = md.into();
        let module_type = ModuleType::SmartLed;

        Self { disp, rx, tx, current_tab: 0, menu_tab_data, module_type}
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

        // Create menu items
        let mut menu0_items = [
            MenuItem::new(self.menu_tab_data.tab0[0], true, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab0[1], false, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab0[2], false, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab0[3], false, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab0[4], false, Point::zero(), text_style.clone()),
        ];

        let mut menu1_items = [
            MenuItem::new(self.menu_tab_data.tab1[0], true, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab1[1], true, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab1[2], false, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab1[3], false, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab1[4], false, Point::zero(), text_style.clone()),
        ];

        let mut menu2_items = [
            MenuItem::new(self.menu_tab_data.tab2[0], true, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab2[1], true, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab2[2], true, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab2[3], true, Point::zero(), text_style.clone()),
            MenuItem::new(self.menu_tab_data.tab2[4], false, Point::zero(), text_style.clone()),
        ];

        // Create menu tabs
        let mut menu0 = MenuTab::new("Main Menu", &mut menu0_items);
        let mut menu1 = MenuTab::new("LED Settings 1", &mut menu1_items);
        let mut menu2 = MenuTab::new("LED Settings 2", &mut menu2_items);

        menu0.arrange();
        menu1.arrange();
        menu2.arrange();

        // Create menu & draw initial tab
        let mut menus = [menu0, menu1, menu2];
        menus[self.current_tab].update();
        let _ = menus[self.current_tab].draw(&mut self.disp);


        loop {
            if let Ok(new_message) = with_timeout(Duration::from_millis(250), self.rx.receive()).await {
                match new_message {
                    UiEvent::Up => {
                        match menus[self.current_tab].next() {
                            MenuMovement::NextTab => self.next_tab(menus.len()),
                            MenuMovement::UpdateValue((item_index, value_index, value)) => {
                                self.update_value(item_index, value);
                            },
                            _ => {},
                        };
                    }
                    UiEvent::Down => {
                        match menus[self.current_tab].previous() {
                            MenuMovement::PreviousTab => self.previous_tab(menus.len()),
                            MenuMovement::UpdateValue((item_index, value_index, value)) => {
                                self.update_value(item_index, value);
                            },
                            _ => {}
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
                            match self.tx.try_send(RouterEvent::MenuDataUpdate(self.menu_tab_data.to_menu_data(self.module_type))) {
                                Ok(_) => {}
                                Err(e) => error!("Message dropped. Channel full. {}", e),
                            };
                        } else {
                            menus[self.current_tab].set_editing(true);
                        }
                    }
                    UiEvent::Load(input) => {
                        let new_data: MenuTabData = input.into();

                        self.current_tab = 0;
                        for (item_index, item) in new_data.tab0.iter().enumerate() {
                            menus[self.current_tab].set(item_index, item.1);
                            info!("Update {} to {}", item_index, item);
                        }

                        self.current_tab = 1;
                        for (item_index, item) in new_data.tab1.iter().enumerate() {
                            menus[self.current_tab].set(item_index, item.1);
                        }

                        self.current_tab = 2;
                        for (item_index, item) in new_data.tab2.iter().enumerate() {
                            menus[self.current_tab].set(item_index, item.1);
                        }

                        self.current_tab = 0;
                    }
                };

                menus[self.current_tab].update();

                self.disp.clear(Rgb565::BLACK).unwrap();
                let _ = menus[self.current_tab].draw(&mut self.disp);
            };
        }
    }

    fn update_value(&mut self, item_index: usize, value: ValueType) {
        match self.current_tab {
            0 => {self.menu_tab_data.tab0[item_index].1 = value;},
            1 => {self.menu_tab_data.tab1[item_index].1 = value;},
            2 => {self.menu_tab_data.tab2[item_index].1 = value;},
            _ => {}
        }
    }
}

#[embassy_executor::task]
pub async fn ui_task_spi(bus: Spi<'static, Async>, cs: Output<'static>, dc: Output<'static>, reset: Output<'static>, mut backlight: OutputOpenDrain<'static>, rx: UiChannelRx, tx: RouterChannelTx) {
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

    let mut ui = Ui::new(display, rx, tx);
    ui.run().await
}
