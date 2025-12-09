//! Things that can be configured in the menu
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

use crate::ui::{split_digits_no_std, IncDec, MenuMovement, NextPrev, SelectionMode, ValueType, View, COLOR_EDITING_TEXT, COLOR_SELECTED_TEXT, COLOR_VALUE_TEXT};

use arrayvec::ArrayString;
use az::SaturatingAs;
use core::fmt::Write;
use embedded_text::{alignment::HorizontalAlignment, TextBox};
use u8g2_fonts::U8g2TextStyle;

use crate::DISPLAY_HEIGHT;

/// MenuValue are things that can be configured in the menu
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
    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    #[inline]
    fn translate_impl(&mut self, by: Point) {
        // make sure you don't accidentally call `translate`!
        self.bounds.translate_mut(by);
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
                p.y += (self.bounds().size.height - box_size.height / 2).saturating_as::<i32>();

                for (index, d) in v.iter().rev().enumerate() {
                    self.set_text_style(index, &mut text_style);

                    let mut buf = ArrayString::<1>::new();
                    write!(&mut buf, "{}", d.0).expect("Can't write");
                    p = Text::new(&buf, p, text_style.clone()).draw(display)?;
                }
            }

            ValueType::SmartLedDmxGroupSize(x) => {
                let v0 = split_digits_no_std(x.0[0], 3);
                let v1 = split_digits_no_std(x.0[1], 3);
                let v2 = split_digits_no_std(x.0[2], 3);
                let v3 = split_digits_no_std(x.0[3], 3);

                let v_all = [v3[0], v3[1], v3[2], v2[0], v2[1], v2[2], v1[0], v1[1], v1[2], v0[0], v0[1], v0[2]];

                let mut p = self.bounds().top_left;

                let box_size = self.character_style.measure_string("888 888 888 888", p, Baseline::Bottom).bounding_box.size;

                p.x += (self.bounds().size.width - box_size.width).saturating_as::<i32>();
                p.y += (self.bounds().size.height - box_size.height / 2).saturating_as::<i32>();

                for (index, d) in v_all.iter().rev().enumerate() {
                    text_style.set_text_color(Some(COLOR_VALUE_TEXT));
                    p = match index {
                        3 | 6 | 9 => Text::new(" ", p, text_style.clone()).draw(display)?,
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
                p.y += (self.bounds().size.height - box_size.height / 2).saturating_as::<i32>();

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
