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

use crate::ui::{IncDec, MenuMovement, NextPrev, SelectionMode, SmartLedGrouping, SmartLedColorMode, Udigit};
use crate::ui::{View, COLOR_EDITING_TEXT, COLOR_SELECTED_TEXT, COLOR_VALUE_TEXT};
use arrayvec::ArrayString;
use az::SaturatingAs;
use core::fmt::Write;
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
                    if self.value_index == index {
                        match self.selection_mode {
                            SelectionMode::Normal => text_style.set_text_color(Some(COLOR_VALUE_TEXT)),
                            SelectionMode::Selected => text_style.set_text_color(Some(COLOR_SELECTED_TEXT)),
                            SelectionMode::Editing => text_style.set_text_color(Some(COLOR_EDITING_TEXT)),
                        }
                    } else {
                        text_style.set_text_color(Some(COLOR_VALUE_TEXT))
                    }

                    let mut buf = ArrayString::<1>::new();
                    write!(&mut buf, "{}", d.0).expect("Can't write");
                    p = Text::new(&buf, p, text_style.clone()).draw(display)?;
                }
            }
            _ => {
                match self.selection_mode {
                    SelectionMode::Normal => text_style.set_text_color(Some(COLOR_VALUE_TEXT)),
                    SelectionMode::Selected => text_style.set_text_color(Some(COLOR_SELECTED_TEXT)),
                    SelectionMode::Editing => text_style.set_text_color(Some(COLOR_EDITING_TEXT)),
                }
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
    SmartLedGrouping(SmartLedGrouping),
    SmartLedColorMode(SmartLedColorMode),
    Uint3(u16),
}

impl ValueType {
    pub fn size(&self) -> usize {
        match self {
            ValueType::None => 0,
            ValueType::U8(_x) => 1,
            ValueType::SmartLedGrouping(_x) => 1,
            ValueType::SmartLedColorMode(_x) => 1,
            ValueType::Uint3(_x) => 3,
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
    pub fn extract_led_grouping(&self) -> SmartLedGrouping {
        info!("Extract grouping {}", self);
        match self {
            ValueType::SmartLedGrouping(x) => *x,
            _ => {
                error!("Unable to extract LED grouping - setting to default value");
                SmartLedGrouping::default()
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
    pub fn extract_uint3(&self) -> u16 {
        match self {
            ValueType::Uint3(x) => *x,
            _ => {
                error!("Unable to extract UINT3 - setting to default value");
                0
            },
        }
    }
}
impl IncDec for ValueType {

    fn increment(&self, index: usize) -> Self {
        match self {
            ValueType::None => ValueType::None,
            ValueType::U8(x) => ValueType::U8(x.increment(index)),
            ValueType::SmartLedGrouping(x) => ValueType::SmartLedGrouping(x.increment(index)),
            ValueType::SmartLedColorMode(x) => ValueType::SmartLedColorMode(x.increment(index)),
            ValueType::Uint3(x) => {
                let new = *x;
                let mut v = split_digits_no_std(new, 3);
                let i = (v.len() - 1) - index;
                v[i] = v[i].increment(i);
                let out = digits_to_u16(v);
                ValueType::Uint3(out)
            }
        }
    }

    fn decrement(&self, index: usize) -> Self {
        match self {
            ValueType::None => ValueType::None,
            ValueType::U8(x) => ValueType::U8(x.decrement(index)),
            ValueType::SmartLedGrouping(x) => ValueType::SmartLedGrouping(x.decrement(index)),
            ValueType::SmartLedColorMode(x) => ValueType::SmartLedColorMode(x.decrement(index)),
            ValueType::Uint3(x) => {
                let new = *x;
                let mut v = split_digits_no_std(new, 3);
                let i = (v.len() - 1) - index;
                v[i] = v[i].decrement(i);
                let out = digits_to_u16(v);
                ValueType::Uint3(out)
            }
        }
    }
}

// How the value type is displayed in the UI
impl core::fmt::Display for ValueType {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            ValueType::None => write!(f, ""),
            ValueType::U8(x) => write!(f, "{}", x),
            ValueType::SmartLedGrouping(x) => write!(f, "{}", x),
            ValueType::SmartLedColorMode(x) => write!(f, "{}", x),
            ValueType::Uint3(x) => write!(f, "{}", x),
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
