//! A single page (view) of the user interface.
use az::SaturatingAs;
use embedded_graphics::{draw_target::DrawTarget, pixelcolor::Rgb565, prelude::Point, primitives::Rectangle, text::renderer::TextRenderer, Drawable};

use embedded_text::{alignment::HorizontalAlignment, TextBox};
use u8g2_fonts::U8g2TextStyle;

use defmt::info;

use crate::ui::{traits::IncDec, traits::NextPrev, MenuItem, MenuMovement, SelectionMode, ValueType, View, COLOR_DEFAULT_TEXT, COLOR_MENU_TEXT, DEFAULT_FONT};

use crate::MENU_ITEMS_PER_TAB;

/// MenuTab is a single page (view) of the user interface.
#[derive(Default)]
pub struct MenuTab<'a> {
    /// Title for the tab / page
    name: &'a str,
    /// List of MenuItems to display
    items: [MenuItem<'a>; MENU_ITEMS_PER_TAB],
    /// Number of values that are selectable.
    /// This could be more than the number of MenuItems if some MenuItems have multiple user interaction (such as a 3 digit number).
    /// This could be less than the number of MenuItems if some MenuItems do not support user editing.
    num_selectable_items: usize,
    /// Index of current value that is selected
    item_index: usize,
    /// If this MenuTab has a value that is being edited.
    editing: bool,
}

impl<'a> MenuTab<'a> {
    /// Assign a value to an item
    ///
    /// # Args
    /// * `item_index` - Index of the MenuItem to update
    /// * `val` - Value to assign to the item
    pub fn set(&mut self, item_index: usize, val: ValueType) {
        self.items[item_index].value.value = val;
    }

    pub fn new(name: &'a str, items: [MenuItem<'a>; MENU_ITEMS_PER_TAB]) -> Self {
        let num_selectable_items = items.iter().filter(|item| item.editable).count();
        Self {
            name,
            items,
            num_selectable_items,
            item_index: 0,
            editing: false,
        }
    }

    /// Apply y translations to items so they are displayed one after another vertically.
    pub fn arrange(&mut self) {
        let mut by = Point::zero();

        let text_style = U8g2TextStyle::new(DEFAULT_FONT, COLOR_DEFAULT_TEXT);
        by.y += text_style.line_height().saturating_as::<i32>();

        self.items.iter_mut().for_each(|item| {
            item.translate_impl(by);
            by.y += item.bounds().size.height.saturating_as::<i32>();
        });
    }

    /// Update the selection mode for each item
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

    /// Set the editing mode for the MenuTab
    pub fn set_editing(&mut self, mode: bool) {
        if self.items[self.item_index].editable {
            self.editing = mode;
        }
    }

    /// Return if the MenuTab is being edited
    pub fn editing(&mut self) -> bool {
        self.editing
    }
}

impl<'a> NextPrev for MenuTab<'a> {
    fn next(&mut self) -> MenuMovement {
        if self.editing {
            // change value of item
            self.items[self.item_index].value = self.items[self.item_index].value.increment(self.items[self.item_index].value.value_index);
            MenuMovement::UpdateValue((self.item_index, self.items[self.item_index].value.value_index, self.items[self.item_index].value.value))
        } else if self.item_index == self.num_selectable_items - 1 && self.items[self.item_index].value.value_index == self.items[self.item_index].size() - 1 {
            // go to next tab
            info!("Go to next tab");
            MenuMovement::NextTab
        } else if self.items[self.item_index].value.value_index == self.items[self.item_index].value.size() - 1 {
            // loop around to first item
            self.item_index = self.item_index.saturating_add(1);
            info!("Next: selected is now {}", self.item_index);
            MenuMovement::FirstItem
        } else {
            self.items[self.item_index].value.value_index = self.items[self.item_index].value.value_index.saturating_add(1);
            MenuMovement::NextItem
        }
    }

    fn previous(&mut self) -> MenuMovement {
        if self.editing {
            // change value of item
            self.items[self.item_index].value = self.items[self.item_index].value.decrement(self.items[self.item_index].value.value_index);
            MenuMovement::UpdateValue((self.item_index, self.items[self.item_index].value.value_index, self.items[self.item_index].value.value))
        } else if self.item_index == 0 && self.items[self.item_index].value.value_index == 0 {
            info!("Go to previous tab");
            MenuMovement::PreviousTab
        } else if self.items[self.item_index].value.value_index == 0 {
            self.item_index = self.item_index.saturating_sub(1);
            info!("Previous: selected is now {}", self.item_index);
            MenuMovement::LastItem
        } else {
            self.items[self.item_index].value.value_index = self.items[self.item_index].value.value_index.saturating_sub(1);
            MenuMovement::PreviousItem
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
