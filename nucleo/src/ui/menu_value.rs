use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::Rgb565,
    prelude::{Point, Size, Transform},
    primitives::Rectangle,
    text::{
        renderer::{CharacterStyle, TextRenderer},
        Baseline, Text,
    },
    Drawable,
};
use heapless::Vec;

use crate::ui::{
    ArtNetAddr, 
    EthernetIPMode, 
    IncDec, 
    InputMode, 
    IpAddrMenu, 
    MenuMovement, 
    NextPrev, 
    SelectionMode, 
    SmartLedColorMode, 
    SmartLedPortMode, 
    SmartLedDmxGroupSize, 
    Udigit,
};
use crate::ui::{View, COLOR_EDITING_TEXT, COLOR_SELECTED_TEXT, COLOR_VALUE_TEXT};
use arrayvec::ArrayString;
use az::SaturatingAs;
use core::{fmt::Write, net::Ipv4Addr};
use embedded_text::{alignment::HorizontalAlignment, TextBox};
use u8g2_fonts::U8g2TextStyle;
use bincode::{Encode, Decode};

use crate::DISPLAY_HEIGHT;
use defmt::{info, error, Format};

/// Menu Value are values that can be configured in the menu
#[derive(Clone)]
pub struct MenuValue {
    pub value: ValueType,
    pub value_index: usize,
    pub bounds: Rectangle,
    character_style: U8g2TextStyle<Rgb565>,
    pub selection_mode: SelectionMode,
}

impl MenuValue {
    pub fn new(value: ValueType, character_style: U8g2TextStyle<Rgb565>, position: Point) -> Self {
        Self {
            value,
            value_index: 0,
            bounds: Rectangle::new(position, Size::new(DISPLAY_HEIGHT.into(), character_style.line_height())),
            character_style,
            selection_mode: SelectionMode::Normal,
        }
    }

    fn set_text_style(&self, index: usize, text_style: &mut U8g2TextStyle<Rgb565>) {
        if self.value_index == index {
            match self.selection_mode {
                SelectionMode::Normal => text_style.set_text_color(Some(COLOR_VALUE_TEXT)),
                SelectionMode::Selected => text_style.set_text_color(Some(COLOR_SELECTED_TEXT)),
                SelectionMode::Editing => text_style.set_text_color(Some(COLOR_EDITING_TEXT)),
            }
        } else {
            text_style.set_text_color(Some(COLOR_VALUE_TEXT))
        }
    }
}

impl NextPrev for MenuValue {
    fn next(&mut self) -> MenuMovement {
        if self.value_index == self.value.size() - 1 {
            MenuMovement::None
        } else {
            self.value_index = self.value_index.saturating_add(1);
            // Some(self.value_index)
            MenuMovement::NextItem
        }
    }

    fn previous(&mut self) -> MenuMovement {
        if self.value_index == 0 {
            // None
            MenuMovement::None
        } else {
            self.value_index = self.value_index.saturating_sub(1);
            // Some(self.value_index)
            MenuMovement::PreviousItem
        }
    }

    fn size(&self) -> usize {
        self.value.size()
    }
}

impl IncDec for MenuValue {
    fn increment(&self, index: usize) -> Self {
        let mut x = self.clone();
        x.value = x.value.increment(index);
        x
    }

    fn decrement(&self, index: usize) -> Self {
        let mut x = self.clone();
        x.value = x.value.decrement(index);
        x
    }
}

/// Implementing `View` is required by the layout and alignment operations
/// `View` teaches `embedded-layout` where our object is, how big it is and how to move it.
impl View for MenuValue {
    #[inline]
    fn translate_impl(&mut self, by: Point) {
        // make sure you don't accidentally call `translate`!
        self.bounds.translate_mut(by);
    }

    #[inline]
    fn bounds(&self) -> Rectangle {
        self.bounds
    }
}

/// Need to implement `Drawable` for a _reference_ of our view
impl Drawable for MenuValue {
    type Color = Rgb565;
    type Output = ();

    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        let mut text_style = self.character_style.clone();

        match self.value {
            ValueType::Uint3(x) => {
                let v = split_digits_no_std(x, 3);

                let mut p = self.bounds().top_left;

                let box_size = self.character_style.measure_string("888", p, Baseline::Bottom).bounding_box.size;
                
                p.x += (self.bounds().size.width - box_size.width).saturating_as::<i32>();
                p.y += (self.bounds().size.height - box_size.height/2).saturating_as::<i32>();

                for (index, d) in v.iter().rev().enumerate() {
                    self.set_text_style(index, &mut text_style);

                    let mut buf = ArrayString::<1>::new();
                    write!(&mut buf, "{}", d.0).expect("Can't write");
                    p = Text::new(&buf, p, text_style.clone()).draw(display)?;
                }
            }

            ValueType::SmartLedDmxGroupSize(x) => {
                let v0 = split_digits_no_std(x.0[0].into(), 3);
                let v1 = split_digits_no_std(x.0[1].into(), 3);
                let v2 = split_digits_no_std(x.0[2].into(), 3);
                let v3 = split_digits_no_std(x.0[3].into(), 3);

                let v_all = [v3[0], v3[1], v3[2], v2[0], v2[1], v2[2], v1[0], v1[1], v1[2], v0[0], v0[1], v0[2]];

                let mut p = self.bounds().top_left;

                let box_size = self.character_style.measure_string("888 888 888 888", p, Baseline::Bottom).bounding_box.size;
                
                p.x += (self.bounds().size.width - box_size.width).saturating_as::<i32>();
                p.y += (self.bounds().size.height - box_size.height/2).saturating_as::<i32>();

                for (index, d) in v_all.iter().rev().enumerate() {
                    text_style.set_text_color(Some(COLOR_VALUE_TEXT));
                    p = match index {
                        // 0 => Text::new("", p, text_style.clone()).draw(display)?,
                        3| 6 | 9 => Text::new(" ", p, text_style.clone()).draw(display)?,
                        // 6 => Text::new(" ", p, text_style.clone()).draw(display)?,
                        // 9 => Text::new(" ", p, text_style.clone()).draw(display)?,
                        _ => p,
                    };
                    
                    self.set_text_style(index, &mut text_style);
                    let mut buf = ArrayString::<1>::new();
                    write!(&mut buf, "{}", d.0).expect("Can't write");
                    p = Text::new(&buf, p, text_style.clone()).draw(display)?;
                }
            }

            ValueType::ArtNetAddr(x) => {
                let v0 = split_digits_no_std(x.0[0].into(), 3);
                let v1 = split_digits_no_std(x.0[1].into(), 3);
                let v2 = split_digits_no_std(x.0[2].into(), 3);

                let v_all = [v2[0], v2[1], v2[2], v1[0], v1[1], v1[2], v0[0], v0[1], v0[2]];

                let mut p = self.bounds().top_left;

                let box_size = self.character_style.measure_string("N888 SN888 U888", p, Baseline::Bottom).bounding_box.size;
                
                p.x += (self.bounds().size.width - box_size.width).saturating_as::<i32>();
                p.y += (self.bounds().size.height - box_size.height/2).saturating_as::<i32>();

                for (index, d) in v_all.iter().rev().enumerate() {
                    text_style.set_text_color(Some(COLOR_VALUE_TEXT));
                    p = match index {
                        0 => Text::new("N", p, text_style.clone()).draw(display)?,
                        3 => Text::new(" SN", p, text_style.clone()).draw(display)?,
                        6 => Text::new(" U", p, text_style.clone()).draw(display)?,
                        _ => p,
                    };
                    
                    self.set_text_style(index, &mut text_style);
                    let mut buf = ArrayString::<1>::new();
                    write!(&mut buf, "{}", d.0).expect("Can't write");
                    p = Text::new(&buf, p, text_style.clone()).draw(display)?;
                }
            }
            _ => {
                self.set_text_style(self.value_index, &mut text_style);
                let mut buf = ArrayString::<20>::new();
                write!(&mut buf, "{}", self.value).expect("Can't write");
                TextBox::with_alignment(&buf, self.bounds, text_style, HorizontalAlignment::Right).draw(display)?;
            }
        }
        Ok(())
    }
}

#[allow(unused)]
/// Menu Value are values that can be configured in the menu
#[derive(Clone, Copy, PartialEq, Eq, Format, Decode, Encode)]
pub enum ValueType {
    None,
    U8(u8),
    SmartLedPortMode(SmartLedPortMode),
    SmartLedColorMode(SmartLedColorMode),
    InputMode(InputMode),
    EthernetIPMode(EthernetIPMode),
    Uint3(u16),
    Ip(IpAddrMenu),
    ArtNetAddr(ArtNetAddr),
    SmartLedDmxGroupSize(SmartLedDmxGroupSize),
}

impl ValueType {
    pub fn size(&self) -> usize {
        match self {
            ValueType::None => 0,
            ValueType::U8(_x) => 1,
            ValueType::SmartLedPortMode(_x) => 1,
            ValueType::SmartLedColorMode(_x) => 1,
            ValueType::SmartLedDmxGroupSize(_x) => 12,
            ValueType::Uint3(_x) => 3,
            ValueType::InputMode(_x) => 1,
            ValueType::EthernetIPMode(_x) => 1,
            ValueType::Ip(_x) => 1,
            ValueType::ArtNetAddr(_x) => 9,
        }
    }

    #[allow(unused)]
    pub fn extract_u8(&self) -> u8 {
        match self {
            ValueType::U8(x) => *x,
            _ => {
                error!("Unable to extract u8 - setting to default value");
                0
            },
        }
    }

    #[allow(unused)]
    pub fn extract_port_mode(&self) -> SmartLedPortMode {
        info!("Extract grouping {}", self);
        match self {
            ValueType::SmartLedPortMode(x) => *x,
            _ => {
                error!("Unable to extract port mode - setting to default value");
                SmartLedPortMode::default()
            },
        }
    }

    #[allow(unused)]
    pub fn extract_led_color_mode(&self) -> SmartLedColorMode {
        info!("Extract color mode {}", self);
        match self {
            ValueType::SmartLedColorMode(x) => *x,
            _ => {
                error!("Unable to extract LED color mode - setting to default value");
                SmartLedColorMode::default()
            },
        }
    }

    #[allow(unused)]
    pub fn extract_dmx_group_size(&self) -> SmartLedDmxGroupSize {
        info!("Extract DMX group size {}", self);
        match self {
            ValueType::SmartLedDmxGroupSize(x) => *x,
            _ => {
                error!("Unable to extract DMX group size - setting to default value");
                SmartLedDmxGroupSize::default()
            },
        }
    }

    #[allow(unused)]
    pub fn extract_uint3(&self) -> u16 {
        match self {
            ValueType::Uint3(x) => *x,
            _ => {
                error!("Unable to extract UINT3 - setting to default value");
                0
            },
        }
    }

    #[allow(unused)]
    pub fn extract_input_mode(&self) -> InputMode {
        info!("Extract input mode {}", self);
        match self {
            ValueType::InputMode(x) => *x,
            _ => {
                error!("Unable to extract input mode - setting to default value");
                InputMode::default()
            },
        }
    }

    #[allow(unused)]
    pub fn extract_ethernet_ip_mode(&self) -> EthernetIPMode {
        info!("Extract ethernet ip mode {}", self);
        match self {
            ValueType::EthernetIPMode(x) => *x,
            _ => {
                error!("Unable to extract ethernet ip mode - setting to default value");
                EthernetIPMode::default()
            },
        }
    }

    #[allow(unused)]
    pub fn extract_ip(&self) -> IpAddrMenu {
        match self {
            ValueType::Ip(x) => *x,
            _ => {
                error!("Unable to extract ip - setting to default value");
                IpAddrMenu::new(0, 0, 0, 0)
            },
        }
    }

    #[allow(unused)]
    pub fn extract_artnet(&self) -> ArtNetAddr {
        match self {
            ValueType::ArtNetAddr(x) => *x,
            _ => {
                error!("Unable to extract ArtNetAddr - setting to default value");
                ArtNetAddr::default()
            },
        }
    }


}
impl IncDec for ValueType {

    fn increment(&self, index: usize) -> Self {
        match self {
            ValueType::None => ValueType::None,
            ValueType::U8(x) => ValueType::U8(x.increment(index)),
            ValueType::SmartLedPortMode(x) => ValueType::SmartLedPortMode(x.increment(index)),
            ValueType::SmartLedColorMode(x) => ValueType::SmartLedColorMode(x.increment(index)),
            ValueType::SmartLedDmxGroupSize(x) => {
                let mut new = *x;
                let b_index = index / 3;
                let e = new.0[b_index] as u16;
                let mut v = split_digits_no_std(e, 3);
                
                let i = (v.len() - 1) - (index-b_index*3);
                v[i] = v[i].increment(i);
                let j = digits_to_u16(v);
                let out = if j > 999 {
                    new.0[b_index]
                } else {
                    j
                };

                new.0[b_index] = out;
                ValueType::SmartLedDmxGroupSize(new)
            },
            ValueType::InputMode(x) => ValueType::InputMode(x.increment(index)),
            ValueType::EthernetIPMode(x) => ValueType::EthernetIPMode(x.increment(index)),
            ValueType::Uint3(x) => {
                let new = *x;
                let mut v = split_digits_no_std(new, 3);
                let i = (v.len() - 1) - index;
                v[i] = v[i].increment(i);
                let out = digits_to_u16(v);
                ValueType::Uint3(out)
            },
            ValueType::Ip(x) => {
                //todo!();
                ValueType::Ip(*x)
            },
            ValueType::ArtNetAddr(x) => {
                let mut new = *x;
                let b_index = index / 3;
                let e = new.0[b_index] as u16;
                let mut v = split_digits_no_std(e, 3);
                
                let i = (v.len() - 1) - (index-b_index*3);
                v[i] = v[i].increment(i);
                let j = digits_to_u16(v);
                let out = if j > u8::MAX.into() {
                    new.0[b_index]
                } else {
                    j as u8
                };

                new.0[b_index] = out;
                ValueType::ArtNetAddr(new)
            },
        }
    }

    fn decrement(&self, index: usize) -> Self {
        match self {
            ValueType::None => ValueType::None,
            ValueType::U8(x) => ValueType::U8(x.decrement(index)),
            ValueType::SmartLedPortMode(x) => ValueType::SmartLedPortMode(x.decrement(index)),
            ValueType::SmartLedColorMode(x) => ValueType::SmartLedColorMode(x.decrement(index)),
            ValueType::SmartLedDmxGroupSize(x) => {
                let mut new = *x;
                let b_index = index / 3;

                // let b_index = if index <= 2 { // editing the first u8
                //     0
                // } else if index <= 5 { // editing the second u8
                //     1
                // } else if index <= 8 { // editing the third u8
                //     2
                // } else {
                //     0
                // };
                let e = new.0[b_index] as u16;
                let mut v = split_digits_no_std(e, 3);
                
                let i = (v.len() - 1) - (index-b_index*3);
                v[i] = v[i].decrement(i);
                let j = digits_to_u16(v);
                let out = if j > 999 {
                    new.0[b_index]
                } else {
                    j
                };
                new.0[b_index] = out;
                ValueType::SmartLedDmxGroupSize(new)
            },
            ValueType::InputMode(x) => ValueType::InputMode(x.decrement(index)),
            ValueType::EthernetIPMode(x) => ValueType::EthernetIPMode(x.decrement(index)),
            ValueType::Uint3(x) => {
                let new = *x;
                let mut v = split_digits_no_std(new, 3);
                let i = (v.len() - 1) - index;
                v[i] = v[i].decrement(i);
                let out = digits_to_u16(v);
                ValueType::Uint3(out)
            },
            ValueType::Ip(x) => {
                //todo!();
                ValueType::Ip(*x)
            },
            ValueType::ArtNetAddr(x) => {
                let mut new: ArtNetAddr = *x;
                let b_index = index / 3;
                let e = new.0[b_index] as u16;
                let mut v = split_digits_no_std(e, 3);
                
                let i = (v.len() - 1) - (index-b_index*3);
                v[i] = v[i].decrement(i);
                let j = digits_to_u16(v);
                let out = if j > u8::MAX.into() {
                    new.0[b_index]
                } else {
                    j as u8
                };
                new.0[b_index] = out;
                ValueType::ArtNetAddr(new)
            },
        }
    }
}

// How the value type is displayed in the UI
impl core::fmt::Display for ValueType {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            ValueType::None => write!(f, ""),
            ValueType::U8(x) => write!(f, "{}", x),
            ValueType::SmartLedPortMode(x) => write!(f, "{}", x),
            ValueType::SmartLedColorMode(x) => write!(f, "{}", x),
            ValueType::SmartLedDmxGroupSize(x) => write!(f, "{} {} {} {}", x.0[0], x.0[1], x.0[2], x.0[3]),
            ValueType::InputMode(x) => write!(f, "{}", x),
            ValueType::EthernetIPMode(x) => write!(f, "{}", x),
            ValueType::Uint3(x) => write!(f, "{}", x),
            ValueType::Ip(x) => {
                let b = x.octets();
                write!(f, "{}.{}.{}.{}", b[0], b[1], b[2], b[3])
            },
            ValueType::ArtNetAddr(x) => write!(f, "{} {} {}", x.0[0], x.0[1], x.0[2]),
        }
    }
}

fn split_digits_no_std(mut n: u16, pad_size: usize) -> heapless::Vec<Udigit, 5> {
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

fn digits_to_u16(n: heapless::Vec<Udigit, 5>) -> u16 {
    let mut out = 0_u16;
    n.iter().enumerate().for_each(|(index, val)| out += val.0 as u16 * 10_u16.pow((index as u16).into()));
    out
}


