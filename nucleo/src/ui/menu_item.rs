
use embedded_graphics::{
    mono_font::MonoTextStyle,
    primitives::Rectangle,
    pixelcolor::Rgb565,
    text::{Text, renderer::{CharacterStyle, TextRenderer}},
    prelude::{Point, RgbColor, Size, WebColors},
    draw_target::DrawTarget,
    Drawable,
};
use embedded_layout::{
    layout::linear::{
        spacing::DistributeFill,
        LinearLayout, 
    },
    prelude::*
};

use arrayvec::ArrayString;
use core::fmt::Write;



use crate::DISPLAY_HEIGHT;

pub struct MenuItem<'a> {
    name: &'a str,
    value: usize,
    bounds: Rectangle,
    character_style: MonoTextStyle<'a, Rgb565>,
}
impl<'a> MenuItem<'a> {
    pub fn new(name: &'a str, value: usize, position: Point, character_style: MonoTextStyle<'a, Rgb565>) -> Self {
        Self {
            name,
            value,
            bounds: Rectangle::new(position, Size::new(DISPLAY_HEIGHT.into(), character_style.line_height())),
            character_style,
        }
    }
}

/// Implementing `View` is required by the layout and alignment operations
/// `View` teaches `embedded-layout` where our object is, how big it is and how to move it.
impl<'a> View for MenuItem<'a> {
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
impl<'a> Drawable for MenuItem<'a> {
    type Color = Rgb565;
    type Output = ();

    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {

        let mut text_style = self.character_style.clone();

        text_style.set_text_color(Some(Rgb565::YELLOW));
        let text_label = Text::new(self.name, Point::zero(), text_style);
        
        text_style.set_text_color(Some(Rgb565::CSS_BLUE_VIOLET));
        
        let mut buf = ArrayString::<20>::new();
        write!(&mut buf, "{}", self.value).expect("Can't write");
        
        
        let text_value = Text::new(&buf, Point::zero(), text_style);

        LinearLayout::horizontal(Chain::new(text_label).append(text_value))
            .with_spacing(DistributeFill(DISPLAY_HEIGHT.into()))
            .align_to(&self.bounds, horizontal::Left, vertical::Center)
            .arrange()
            .draw(display)?;

        Ok(())
    }
}