use az::SaturatingAs;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{iso_8859_4::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::{Point, RgbColor},
    primitives::Rectangle,
    text::renderer::TextRenderer,
    Drawable,
};

use embedded_text::{alignment::HorizontalAlignment, TextBox};
use u8g2_fonts::{fonts, U8g2TextStyle};

use defmt::info;

use crate::ui::{layout::IncDec, COLOR_DEFAULT_TEXT, COLOR_MENU_TEXT, DEFAULT_FONT}; // DEFAULT_FONT};
use crate::ui::{layout::NextPrev, MenuItem, SelectionMode, View};

pub struct MenuTab<'a> {
    name: &'a str,
    items: &'a mut [MenuItem<'a>],
    num_selectable_items: usize,
    item_index: usize,
    editing: bool,
}

impl<'a> MenuTab<'a> {
    pub fn new(name: &'a str, items: &'a mut [MenuItem<'a>]) -> Self {
        let num_selectable_items = items.iter().filter(|item| item.editable).count();
        Self {
            name,
            items,
            num_selectable_items,
            item_index: 0,
            editing: false,
        }
    }

    pub fn arrange(&mut self) {
        let mut by = Point::zero();

        let text_style = U8g2TextStyle::new(DEFAULT_FONT, COLOR_DEFAULT_TEXT);
        by.y += text_style.line_height().saturating_as::<i32>();

        self.items.iter_mut().for_each(|item| {
            item.translate_impl(by);
            by.y += item.bounds().size.height.saturating_as::<i32>();
        });
    }

    pub fn update(&mut self) {
        self.items.iter_mut().enumerate().for_each(|(index, item)| {
            if index == self.item_index {
                if self.editing {
                    item.value.selection_mode = SelectionMode::Editing;
                } else {
                    item.value.selection_mode = SelectionMode::Selected;
                }
            } else {
                item.value.selection_mode = SelectionMode::Normal;
            }
        });
    }

    pub fn set_editing(&mut self, mode: bool) {
        self.editing = mode;
    }

    pub fn editing(&mut self) -> bool {
        self.editing
    }
}

impl<'a> NextPrev for MenuTab<'a> {
    fn next(&mut self) -> Option<usize> {
        if self.editing {
            // change value of item
            self.items[self.item_index].value = self.items[self.item_index].value.increment(self.items[self.item_index].value.value_index);
            Some(self.item_index)
        } else if self.item_index == self.num_selectable_items - 1 && self.items[self.item_index].value.value_index == self.items[self.item_index].size() - 1 {
            info!("Go to next tab");
            None
        } else if self.items[self.item_index].value.value_index == self.items[self.item_index].value.size() - 1 {
            self.item_index = self.item_index.saturating_add(1);
            info!("Next: selected is now {}", self.item_index);
            Some(self.item_index)
        } else {
            self.items[self.item_index].value.value_index = self.items[self.item_index].value.value_index.saturating_add(1);
            Some(self.item_index)
        }
    }

    fn previous(&mut self) -> Option<usize> {
        if self.editing {
            // change value of item
            self.items[self.item_index].value = self.items[self.item_index].value.decrement(self.items[self.item_index].value.value_index);
            Some(self.item_index)
        } else if self.item_index == 0 && self.items[self.item_index].value.value_index == 0 {
            info!("Go to previous tab");
            None // need to update this
        } else if self.items[self.item_index].value.value_index == 0 {
            self.item_index = self.item_index.saturating_sub(1);
            info!("Previous: selected is now {}", self.item_index);
            Some(self.item_index)
        } else {
            self.items[self.item_index].value.value_index = self.items[self.item_index].value.value_index.saturating_sub(1);
            Some(self.item_index)
        }
    }

    fn size(&self) -> usize {
        self.num_selectable_items
    }
}

/// Need to implement `Drawable` for a _reference_ of our view
impl<'a> Drawable for MenuTab<'a> {
    type Color = Rgb565;
    type Output = ();

    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        let mut by = Point::zero();
        let display_area = display.bounding_box();

        let text_style = U8g2TextStyle::new(DEFAULT_FONT, COLOR_MENU_TEXT);
        TextBox::with_alignment(
            self.name,
            Rectangle::with_corners(
                Point::zero(),
                Point::new(display_area.size.width.saturating_as::<i32>(), text_style.line_height().saturating_as::<i32>()),
            ),
            text_style.clone(),
            HorizontalAlignment::Center,
        )
        .draw(display)?;

        by.x += 0;
        by.y += text_style.line_height().saturating_as::<i32>();

        self.items.iter().for_each(|item| {
            let _ = item.draw(display);
        });

        Ok(())
    }
}
