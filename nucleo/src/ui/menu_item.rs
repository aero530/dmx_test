use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::MonoTextStyle,
    pixelcolor::Rgb565,
    prelude::{Point, RgbColor, Size, Transform, WebColors},
    primitives::Rectangle,
    text::renderer::{CharacterStyle, TextRenderer},
    Drawable,
};

use embedded_text::{alignment::HorizontalAlignment, TextBox};
use u8g2_fonts::U8g2TextStyle;

use crate::ui::{layout::NextPrev, COLOR_ITEM_TEXT, COLOR_MENU_TEXT};

use super::{MenuValue, View};

use crate::ui::menu_value::ValueType;
use crate::DISPLAY_HEIGHT;

#[derive(Clone)]
pub struct MenuItem<'a> {
    name: &'a str,
    pub value: MenuValue,
    // pub value_index: usize,
    // pub value_count: usize,
    bounds: Rectangle,
    character_style: U8g2TextStyle<Rgb565>,
    // pub selection_mode: [SelectionMode; 4],
    pub editable: bool,
}
impl<'a> MenuItem<'a> {
    pub fn new(name: &'a str, value: ValueType, editable: bool, position: Point, character_style: U8g2TextStyle<Rgb565>) -> Self {
        Self {
            name,
            value: MenuValue::new(value, character_style.clone(), position),
            bounds: Rectangle::new(position, Size::new(DISPLAY_HEIGHT.into(), character_style.line_height())),
            character_style,
            editable,
        }
    }
}

impl<'a> NextPrev for MenuItem<'a> {
    fn next(&mut self) -> Option<usize> {
        self.value.next()
    }

    fn previous(&mut self) -> Option<usize> {
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
