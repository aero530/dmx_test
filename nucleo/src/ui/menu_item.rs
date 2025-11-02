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

use crate::ui::{layout::NextPrev, MenuMovement, COLOR_DEFAULT_TEXT, COLOR_ITEM_TEXT, DEFAULT_FONT};

use super::{MenuValue, View};

use crate::ui::menu_value::ValueType;
use crate::DISPLAY_HEIGHT;

#[derive(Clone, Copy, PartialEq, Eq, Format)]
pub struct MenuItemInput {
    pub name: &'static str,
    pub value: ValueType,
    pub editable: bool,
    pub line_break: bool,
}

impl<'a> MenuItemInput {
    pub fn new(name: &'static str, value: ValueType, editable: bool, line_break: bool) -> Self {
        Self { name, value, editable, line_break }
    }
}

#[derive(Clone)]
pub struct MenuItem<'a> {
    name: &'a str,
    pub value: MenuValue,
    bounds: Rectangle,
    character_style: U8g2TextStyle<Rgb565>,
    pub editable: bool,
    line_break: bool,
}

impl<'a> MenuItem<'a> {
    pub fn new(input: MenuItemInput, position: Point, character_style: U8g2TextStyle<Rgb565>) -> Self {
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
    fn translate_impl(&mut self, by: Point) {
        // make sure you don't accidentally call `translate`!
        self.bounds.translate_mut(by);
        self.value.bounds.translate_mut(by);
    }

    #[inline]
    fn bounds(&self) -> Rectangle {
        self.bounds
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
