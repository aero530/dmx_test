use embedded_graphics::{prelude::Point, primitives::Rectangle};

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
    fn next(&mut self) -> Option<usize>;
    fn previous(&mut self) -> Option<usize>;
    fn size(&self) -> usize;
}
