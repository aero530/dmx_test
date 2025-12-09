//! Traits used in the user interface
use embedded_graphics::{prelude::Point, primitives::Rectangle};

use crate::ui::MenuMovement;

/// View trait used for items that can be displayed
pub trait View {
    fn bounds(&self) -> Rectangle;
    fn translate_impl(&mut self, by: Point);
}

/// Values that can be incremented or decremented
pub trait IncDec {
    fn increment(&self, index: usize) -> Self;
    fn decrement(&self, index: usize) -> Self;
}

/// Objects that are navigable (have a previous / next)
pub trait NextPrev {
    /// Determine what should happen when next is applied to this object
    fn next(&mut self) -> MenuMovement;
    /// Determine what should happen when previous is applied to this object
    fn previous(&mut self) -> MenuMovement;
    /// Determine the size of the object as in how many navigable items this object contains
    fn size(&self) -> usize;
}
