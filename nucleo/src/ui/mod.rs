//! User interface
//!
//! The user interface is made up of multiple MenuTabs. Each MenuTab contains rows of MenuItems.
//! A MenuItem has a name and value. The type of the value defined how the data is displayed and
//! interacted with in the menu system (ie if it is an enum dropdown, number select, etc).
use cfg_if::cfg_if;
use defmt::Format;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{error, info};
    } else {
        use defmt::{error, info};
    }
}
use embassy_stm32::gpio::{Output, OutputOpenDrain};
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_time::Delay;

use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::Rgb565,
    prelude::{Point, RgbColor},
    Drawable,
};
use embedded_hal_bus::spi::ExclusiveDevice;

use mipidsi::interface::SpiInterface;
use mipidsi::options::{Orientation, Rotation};
use mipidsi::{models::ST7789, options::ColorInversion, Builder};
use u8g2_fonts::U8g2TextStyle;

use embassy_time::{with_timeout, Duration};

use static_cell::StaticCell;

use crate::channels::{RouterChannelTx, UiChannelRx};
use crate::event_router::RouterEvent;
use crate::ui::traits::NextPrev;

use crate::ui::ValueType;

use crate::{DISPLAY_HEIGHT, DISPLAY_OFFSET, DISPLAY_WIDTH, MENU_ITEMS_PER_TAB, MENU_NUM_TABS};

mod menu_item;
use menu_item::{MenuItem, MenuItemInputs};

mod menu_tab;
use menu_tab::MenuTab;

mod traits;
pub use traits::{IncDec, View};

mod types;
pub use types::*;

mod menu_value;
pub use menu_value::MenuValue;

mod theme_defaults;

use theme_defaults::*;

mod value_type;
use value_type::*;

type SpiDisplay = mipidsi::Display<SpiInterface<'static, ExclusiveDevice<Spi<'static, Async>, Output<'static>, embedded_hal_bus::spi::NoDelay>, Output<'static>>, ST7789, Output<'static>>;

/// Buffer to write data to SPI based display
static SPI_DISP_BUFFER: StaticCell<[u8; 512]> = StaticCell::new();

pub fn split_digits_no_std(mut n: u16, pad_size: usize) -> heapless::Vec<Udigit, 5> {
    let mut digits = heapless::Vec::new(); // Max 5 digits

    while n > 0 {
        let digit = Udigit((n % 10) as u8);
        let _ = digits.push(digit);
        n /= 10;
    }

    while digits.len() < pad_size {
        let _ = digits.push(Udigit(0));
    }
    digits
}

pub fn digits_to_u16(n: heapless::Vec<Udigit, 5>) -> u16 {
    let mut out = 0_u16;
    n.iter().enumerate().for_each(|(index, val)| out += val.0 as u16 * 10_u16.pow((index as u16).into()));
    out
}

/// Menu data structured for use with UI tabs
#[derive(Clone, Copy, PartialEq, Eq, Format)]
pub struct MenuTabData([[MenuItemInputs; MENU_ITEMS_PER_TAB]; MENU_NUM_TABS]);

impl From<MenuData> for MenuTabData {
    fn from(source: MenuData) -> Self {
        // info!("Convert menu data to menu tab data.");
        let (tab1, tab2) = match source.module {
            ModuleSettings::Pwm(_pwm_settings) => (
                [
                    MenuItemInputs::new("", ValueType::None, false, false),
                    MenuItemInputs::new("", ValueType::None, false, false),
                    MenuItemInputs::new("", ValueType::None, false, false),
                    MenuItemInputs::new("", ValueType::None, false, false),
                    MenuItemInputs::new("", ValueType::None, false, false),
                ],
                [
                    MenuItemInputs::new("", ValueType::None, false, false),
                    MenuItemInputs::new("", ValueType::None, false, false),
                    MenuItemInputs::new("", ValueType::None, false, false),
                    MenuItemInputs::new("", ValueType::None, false, false),
                    MenuItemInputs::new("", ValueType::None, false, false),
                ],
            ),
            ModuleSettings::SmartLed(smart_led_settings) => (
                [
                    MenuItemInputs::new("Port Mode", ValueType::SmartLedPortMode(smart_led_settings.port_mode), true, false),
                    MenuItemInputs::new("LED Group Size", ValueType::SmartLedDmxGroupSize(smart_led_settings.dmx_group_size), true, true),
                    MenuItemInputs::new("Color Mode", ValueType::SmartLedColorMode(smart_led_settings.color_mode), true, false),
                    MenuItemInputs::new("", ValueType::None, false, false),
                    MenuItemInputs::new("", ValueType::None, false, false),
                ],
                [
                    MenuItemInputs::new("LEDs on Port 1", ValueType::Uint3(smart_led_settings.leds_per_port[0]), true, false),
                    MenuItemInputs::new("LEDs on Port 2", ValueType::Uint3(smart_led_settings.leds_per_port[1]), true, false),
                    MenuItemInputs::new("LEDs on Port 3", ValueType::Uint3(smart_led_settings.leds_per_port[2]), true, false),
                    MenuItemInputs::new("LEDs on Port 4", ValueType::Uint3(smart_led_settings.leds_per_port[3]), true, false),
                    MenuItemInputs::new("Univrs Offset", ValueType::PortUniverseOffsets(smart_led_settings.universe_offset()), false, false),
                ],
            ),
        };

        MenuTabData([
            [
                MenuItemInputs::new("DMX", ValueType::Uint3(source.dmx_address), true, false),
                MenuItemInputs::new("Input Mode", ValueType::InputMode(source.input_mode), true, false),
                MenuItemInputs::new("Ethernet IP", ValueType::EthernetIPMode(source.ethernet_ip_mode), true, false),
                MenuItemInputs::new("IP", ValueType::Ip(source.ip_addr), true, false),
                MenuItemInputs::new("ArtNet", ValueType::ArtNetAddr(source.artnet_address), true, false),
            ],
            tab1,
            tab2,
        ])
    }
}

impl MenuTabData {
    /// Convert MenuTabData to MenuData
    #[allow(unused)]
    fn to_menu_data(self, module_type: ModuleType) -> MenuData {
        // info!("Convert menu tab data to menu data.");
        let module_settings = match module_type {
            ModuleType::Pwm => ModuleSettings::Pwm(PwmSettings { freq: todo!() }),
            ModuleType::SmartLed => {
                let dmx_group_size = self.0[1][1].value.extract_dmx_group_size();
                let color_mode = self.0[1][2].value.extract_led_color_mode();
                let leds_per_port = [
                    self.0[2][0].value.extract_uint3(),
                    self.0[2][1].value.extract_uint3(),
                    self.0[2][2].value.extract_uint3(),
                    self.0[2][3].value.extract_uint3(),
                ];

                ModuleSettings::SmartLed(SmartLedSettings {
                    port_mode: self.0[1][0].value.extract_port_mode(),
                    dmx_group_size,
                    color_mode,
                    leds_per_port,
                })
            }
            ModuleType::Unknown => {
                todo!()
            }
        };

        MenuData {
            dmx_address: self.0[0][0].value.extract_uint3(),
            input_mode: self.0[0][1].value.extract_input_mode(),
            ethernet_ip_mode: self.0[0][2].value.extract_ethernet_ip_mode(),
            ip_addr: self.0[0][3].value.extract_ip(),
            artnet_address: self.0[0][4].value.extract_artnet(),
            module: module_settings,
        }
    }
}

/// Action to be processed by the UI (often user interaction)
#[derive(Format)]
pub enum UiEvent {
    Up,
    Down,
    Select,
    Esc,
    Load(MenuData),
}

/// Interface to UI that maintains reference to hardware display and manages menu data
#[allow(unused)]
pub struct Ui<'a> {
    /// Hardware display
    disp: SpiDisplay,
    rx: UiChannelRx,
    tx: RouterChannelTx,
    /// Current tab shown in the UI
    current_tab: usize,
    /// Menu data structured for use with UI tabs
    menu_tab_data: MenuTabData,
    /// Menu data formatted for EEPROM storage
    menu_data: MenuData,
    /// Currently installed module type
    module_type: ModuleType,
    /// Menus displayed in the UI
    menus: [MenuTab<'a>; 3],
}

impl<'a> Ui<'a> {
    pub fn new(disp: SpiDisplay, rx: UiChannelRx, tx: RouterChannelTx) -> Self {
        let md = MenuData::default(); // should be coming from the inputs to new fn
        let menu_tab_data = md.into();
        let module_type = ModuleType::SmartLed;

        Self {
            disp,
            rx,
            tx,
            current_tab: 0,
            menu_data: md,
            menu_tab_data,
            module_type,
            menus: [MenuTab::default(), MenuTab::default(), MenuTab::default()],
        }
    }

    /// Go to next tab
    fn next_tab(&mut self) {
        if self.current_tab == self.menus.len() - 1 {
            self.current_tab = 0;
        } else {
            self.current_tab = self.current_tab.saturating_add(1);
        }
    }

    /// Go to previous tab
    fn previous_tab(&mut self) {
        if self.current_tab == 0 {
            self.current_tab = self.menus.len() - 1;
        } else {
            self.current_tab = self.current_tab.saturating_sub(1)
        }
    }

    /// Main function to manage the UI
    pub async fn run(&mut self) {
        // Create a Rectangle from the display's dimensions
        let text_style = U8g2TextStyle::new(DEFAULT_FONT, COLOR_DEFAULT_TEXT);

        let mtd: MenuTabData = self.menu_data.into();
        // Create menu items
        let menu0_items = [
            MenuItem::new(mtd.0[0][0], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[0][1], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[0][2], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[0][3], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[0][4], Point::zero(), text_style.clone()),
        ];

        let menu1_items = [
            MenuItem::new(mtd.0[1][0], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[1][1], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[1][2], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[1][3], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[1][4], Point::zero(), text_style.clone()),
        ];

        let menu2_items = [
            MenuItem::new(mtd.0[2][0], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[2][1], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[2][2], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[2][3], Point::zero(), text_style.clone()),
            MenuItem::new(mtd.0[2][4], Point::zero(), text_style.clone()),
        ];

        // Create menu tabs
        let mut menu0 = MenuTab::new("Main Menu", menu0_items);
        let mut menu1 = MenuTab::new("LED Settings 1", menu1_items);
        let mut menu2 = MenuTab::new("LED Settings 2", menu2_items);

        menu0.arrange();
        menu1.arrange();
        menu2.arrange();

        // Create menu & draw initial tab
        let menu_structure = [menu0, menu1, menu2];
        self.menus = menu_structure;

        self.menus[self.current_tab].update();
        let _ = self.menus[self.current_tab].draw(&mut self.disp);

        loop {
            // Process message to UI
            if let Ok(new_message) = with_timeout(Duration::from_millis(200), self.rx.receive()).await {
                match new_message {
                    UiEvent::Up => {
                        match self.menus[self.current_tab].next() {
                            MenuMovement::NextTab => self.next_tab(),
                            MenuMovement::UpdateValue((item_index, _value_index, value)) => {
                                self.update_menu_tab_data(item_index, value);
                                self.menus[self.current_tab].set(item_index, value);
                            }
                            _ => {}
                        };
                    }
                    UiEvent::Down => {
                        match self.menus[self.current_tab].previous() {
                            MenuMovement::PreviousTab => self.previous_tab(),
                            MenuMovement::UpdateValue((item_index, _value_index, value)) => {
                                self.update_menu_tab_data(item_index, value);
                                self.menus[self.current_tab].set(item_index, value);
                            }
                            _ => {}
                        };
                    }
                    UiEvent::Esc => {
                        if !self.menus[self.current_tab].editing() {
                            self.next_tab();
                        }
                    }
                    UiEvent::Select => {
                        if self.menus[self.current_tab].editing() {
                            self.menus[self.current_tab].set_editing(false);
                            self.menu_data = self.menu_tab_data.to_menu_data(self.module_type);
                            info!("UI Event Select - menu data {:?}", self.menu_data);
                            match self.tx.try_send(RouterEvent::WriteSettingsToEeprom(self.menu_data)) {
                                Ok(_) => {}
                                Err(e) => error!("Message dropped. Channel full. {:?}", e),
                            };
                        } else {
                            self.menus[self.current_tab].set_editing(true);
                        }
                    }
                    UiEvent::Load(menu_data) => {
                        self.menu_data = menu_data;
                        self.menu_tab_data = self.menu_data.into();

                        for (item_index, item) in self.menu_tab_data.0[0].iter().enumerate() {
                            self.menus[0].set(item_index, item.value);
                        }

                        for (item_index, item) in self.menu_tab_data.0[1].iter().enumerate() {
                            self.menus[1].set(item_index, item.value);
                        }

                        for (item_index, item) in self.menu_tab_data.0[2].iter().enumerate() {
                            self.menus[2].set(item_index, item.value);
                        }
                    }
                };

                self.menus[self.current_tab].update();

                let _ = self.disp.clear(Rgb565::BLACK);
                let _ = self.menus[self.current_tab].draw(&mut self.disp);
            };
        }
    }

    fn update_menu_tab_data(&mut self, item_index: usize, value: ValueType) {
        match self.current_tab {
            0 => {
                self.menu_tab_data.0[0][item_index].value = value;
            }
            1 => {
                self.menu_tab_data.0[1][item_index].value = value;
            }
            2 => {
                self.menu_tab_data.0[2][item_index].value = value;
            }
            _ => {}
        }
    }
}

/// Task to manage and display the UI
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
    let _ = display.clear(Rgb565::BLACK);

    // Turn on backlight
    backlight.set_high();

    let mut ui = Ui::new(display, rx, tx);
    ui.run().await
}
