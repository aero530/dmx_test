//! A single row of information in the menu
use defmt::Format;
use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::Rgb565,
    prelude::{Point, Size, Transform},
    primitives::Rectangle,
    text::renderer::{CharacterStyle, TextRenderer},
    Drawable,
};

use embedded_text::{alignment::HorizontalAlignment, TextBox};
use u8g2_fonts::U8g2TextStyle;

use crate::ui::{traits::NextPrev, MenuMovement, COLOR_DEFAULT_TEXT, COLOR_ITEM_TEXT, DEFAULT_FONT};

use super::{MenuValue, View};

use crate::ui::ValueType;
use crate::DISPLAY_HEIGHT;

/// Values needed to create a MenuItem
///
/// This mirrors MenuItem without the internal data as a unique type to allow easier updates of
/// underlying data and the displayed user interface.
#[derive(Clone, Copy, PartialEq, Eq, Format)]
pub struct MenuItemInputs {
    /// Item display name
    pub name: &'static str,
    /// Item value
    pub value: ValueType,
    /// If the item is editable
    pub editable: bool,
    /// If display name and value should be split across two lines
    pub line_break: bool,
}

impl MenuItemInputs {
    pub fn new(name: &'static str, value: ValueType, editable: bool, line_break: bool) -> Self {
        Self { name, value, editable, line_break }
    }
}

#[derive(Clone)]
pub struct MenuItem<'a> {
    /// Item display name
    name: &'a str,
    /// Item value
    pub value: MenuValue,
    /// Item display bounds
    bounds: Rectangle,
    /// Item character style
    character_style: U8g2TextStyle<Rgb565>,
    /// If the item is editable
    pub editable: bool,
    /// If display name and value should be split across two lines
    #[allow(unused)]
    line_break: bool,
}

impl<'a> MenuItem<'a> {
    pub fn new(input: MenuItemInputs, position: Point, character_style: U8g2TextStyle<Rgb565>) -> Self {
        let (height, value_position) = match input.line_break {
            true => (2 * character_style.line_height(), Point::new(position.x, position.y + character_style.line_height() as i32)),
            false => (character_style.line_height(), position),
        };

        Self {
            name: input.name,
            value: MenuValue::new(input.value, character_style.clone(), value_position),
            bounds: Rectangle::new(position, Size::new(DISPLAY_HEIGHT.into(), height)),
            character_style,
            editable: input.editable,
            line_break: input.line_break,
        }
    }
}

impl Default for MenuItem<'_> {
    fn default() -> Self {
        let text_style = U8g2TextStyle::new(DEFAULT_FONT, COLOR_DEFAULT_TEXT);
        Self {
            name: "",
            value: MenuValue::new(ValueType::None, text_style.clone(), Point::zero()),
            bounds: Rectangle::new(Point::zero(), Size::new(DISPLAY_HEIGHT.into(), text_style.line_height())),
            character_style: text_style,
            editable: false,
            line_break: false,
        }
    }
}

impl<'a> NextPrev for MenuItem<'a> {
    fn next(&mut self) -> MenuMovement {
        self.value.next()
    }

    fn previous(&mut self) -> MenuMovement {
        self.value.previous()
    }

    fn size(&self) -> usize {
        self.value.size()
    }
}

/// Implementing `View` is required by the layout and alignment operations
/// `View` teaches `embedded-layout` where our object is, how big it is and how to move it.
impl<'a> View for MenuItem<'a> {
    #[inline]
    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    #[inline]
    fn translate_impl(&mut self, by: Point) {
        // make sure you don't accidentally call `translate`!
        self.bounds.translate_mut(by);
        self.value.bounds.translate_mut(by);
    }
}

/// Need to implement `Drawable` for a _reference_ of our view
impl<'a> Drawable for MenuItem<'a> {
    type Color = Rgb565;
    type Output = ();

    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        let mut text_style = self.character_style.clone();

        text_style.set_text_color(Some(COLOR_ITEM_TEXT));
        let _ = TextBox::with_alignment(self.name, self.bounds, text_style, HorizontalAlignment::Left).draw(display);

        self.value.draw(display)?;

        Ok(())
    }
}
