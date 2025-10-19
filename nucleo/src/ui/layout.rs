use embedded_graphics::{prelude::Point, primitives::Rectangle};

use crate::ui::MenuMovement;

pub trait View {
    fn bounds(&self) -> Rectangle;
    fn translate_impl(&mut self, by: Point);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Normal,
    Selected,
    Editing,
}

pub trait IncDec {
    fn increment(&self, index: usize) -> Self;
    fn decrement(&self, index: usize) -> Self;
    // fn size(&self) -> usize;
}

pub trait NextPrev {
    fn next(&mut self) -> MenuMovement;
    fn previous(&mut self) -> MenuMovement;
    fn size(&self) -> usize;
}
