//! UI hardware interface


use defmt::Format;
use embassy_stm32::gpio::{Output, OutputOpenDrain};
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_time::Delay;

use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::interface::SpiInterface;
use mipidsi::options::{Orientation, Rotation};
use mipidsi::{models::ST7789, options::ColorInversion, Builder};

use embedded_graphics::{
    mono_font::{MonoTextStyle, iso_8859_4::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::{Point, RgbColor},
    geometry::Dimensions,
    draw_target::DrawTarget,
    Drawable,
};
use embedded_layout::{
    layout::linear::{
        spacing::FixedMargin,
        LinearLayout, 
    },
    prelude::*
};


use static_cell::StaticCell;

use crate::channels::UiChannelRx;

use crate::{DISPLAY_HEIGHT, DISPLAY_OFFSET, DISPLAY_WIDTH};

mod menu_item;
use menu_item::MenuItem;



// mod app;
// use app::App;

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

#[derive(Format)]
pub enum UiEvent {
    Up,
    Down,
    Next,
    Select,
    NextTab,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum TestEnum {
    #[default]
    _0,
    _1,
    _2,
    _3,
    _4,
    _5,
    _6,
    _7,
    _8,
    _9,
}

#[derive(Clone, Copy)]
struct PwmSettings {
    freq: u8,
}

#[derive(Default, Clone, Copy)]
struct SmartLedSettings {
    leds_per_port: (u16, u16, u16, u16),
    address_mode: SmartLedGroupingAddressing,
    grouping: SmartLedGrouping,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum SmartLedGroupingAddressing {
    #[default]
    RGB,
    RGBW,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum SmartLedGrouping {
    #[default]
    Individual,
    CombineByPort,
    CombineByModule,
}

#[derive(Clone, Copy)]
enum ModuleSettings {
    SmartLed(SmartLedSettings),
    Pwm(PwmSettings),
}

#[derive(Clone, Copy)]
struct MenuData {
    dmx_address: u16,
    ip: (u8,u8,u8,u8),
    module: ModuleSettings,
}

impl Default for MenuData {
    fn default() -> Self {
        Self{
            dmx_address:0,
            ip: (0,0,0,0),
            module: ModuleSettings::SmartLed(SmartLedSettings::default()),
        }
    }
}

#[derive(Clone, Copy)]
struct MenuTab<D> {
    name: &'static str,
    content: D,
}



// #[derive(Default, Clone, Copy)]
// enum MenuEvent {
//     SliceCheckbox(usize, bool),
//     // Select(TestEnum),
//     #[default]
//     Nothing,
//     Quit,
// }


//<T> where T: Drawable
pub struct Ui {
    disp: SpiDisplay,
    rx: UiChannelRx,
    // tabs: [MenuTab;4],
    current_tab: usize,
    menu_data: MenuData,
    menu_event: Option<UiEvent>,
}

// #[derive(Clone, Copy)]
// struct MyTheme;
// impl Theme for MyTheme {
//     type Color = Rgb565;
//     fn text_color(&self) -> Self::Color {
//         Rgb565::WHITE
//     }
//     fn selected_text_color(&self) -> Self::Color {
//         Rgb565::BLUE
//     }
//     fn selection_color(&self) -> Self::Color {
//         Rgb565::new(51, 255, 51)
//     }
// }

impl Ui {
    pub fn new(
        disp: SpiDisplay,
        rx: UiChannelRx,
    ) -> Self {
        Self {
            disp,
            rx,
            menu_data: MenuData::default(),
            menu_event: None,
            // tabs: [],
            current_tab: 0,
        }
    }

    pub async fn run(&mut self) {
        // let mut menu0 = Menu::with_style(
        //     "   Tab 0",
        //     MenuStyle::new(ExampleTheme).with_font(&FONT_10X20).with_title_font(&FONT_10X20)
        //     )
        //     // .add_item("DMX:", self.menu.dmx_address, |_| 1)
        //     .add_item("Check this 2", false, |b| 30 + b as i32)
        //     .add_item("Check this 3", false, |b| 30 + b as i32)
        //     .add_item("Check this 4", false, |b| 30 + b as i32)
        //     // .add_item("Next Tab", ">>", |_| {self.menu_event=UiEvent::NextTab; ()})
        //     .add_item("Check this 3", TestEnum::_0, |b| b as i32)
        //     .build();

        // let mut menu1 = Menu::with_style(
        //     "   Tab 1",
        //     MenuStyle::new(ExampleTheme).with_font(&FONT_10X20).with_title_font(&FONT_10X20)
        //     )
        //     .add_item("Foo", ">", |_| 1)
        //     .add_section_title("===== Section =====")
        //     .add_item("Check this 5", false, |b| 30 + b as i32)
        //     .build();

        // let mut menu2 = Menu::with_style(
        //     "   Tab 2",
        //     MenuStyle::new(ExampleTheme).with_font(&FONT_10X20).with_title_font(&FONT_10X20)
        //     )
        //     .add_item("Bar", ">", |_| 1)
        //     .add_item("More stuff", false, |b| 20 + b as i32)
        //     .build();

        // let mut menu3 = Menu::with_style(
        //     "   Tab 3",
        //     MenuStyle::new(ExampleTheme).with_font(&FONT_10X20).with_title_font(&FONT_10X20)
        //     )
        //     .add_item("Cat", ">", |_| 1)
        //     .add_section_title("===== Section =====")
        //     .build();
        

        // let tab1 = [
        //     MenuItem::new("iotme name", 7, Point::zero(), text_style),
        //     MenuItem::new("sad name", 9, Point::zero(), text_style)
        // ];

        // Create a Rectangle from the display's dimensions
        let display_area = self.disp.bounding_box();

        let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
        
        let q = LinearLayout::vertical(
            Chain::new(MenuItem::new("iotme name", 7, Point::zero(), text_style))
            .append(MenuItem::new("other thing", 9, Point::zero(), text_style))
        )
            .with_spacing(FixedMargin(4))
            .arrange()
            .align_to(&display_area, horizontal::Center, vertical::Top)
            .draw(&mut self.disp);
        

        // self.tabs[0] = menu0;

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

    async fn process_event(&mut self, event: UiEvent) {
        self.menu_event = Some(event);
        // match event {
        //     UiEvent::Up => {
        //         self.menu_event = Some(event);
        //         self.app.goto_next_tab();
        //     }
        //     UiEvent::Down => {
        //         self.app.goto_previous_tab();
        //     }
        //     UiEvent::Next => {
        //         self.app.goto_previous_tab();
        //     }
        //     UiEvent::Select => {
        //         self.app.current_tab();
        //     },
        // }
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

    // // Draw a smiley face with white eyes and a red mouth
    // draw_smiley(&mut display).unwrap();

    let mut ui = Ui::new(display, rx);
    ui.run().await
}
