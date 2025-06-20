//! ANSI terminal color formatting
use core::fmt;

/// The location for applying a color
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Canvas {
    Background,
    Foreground,
}

/// Generates ANSI escape sequences using a specific color
pub trait FormatColor {
    /// Apply the color
    fn prelude(&self, f: &mut fmt::Formatter, canvas: Canvas) -> fmt::Result;

    /// Undo the color application
    fn epilogue(&self, f: &mut fmt::Formatter, canvas: Canvas) -> fmt::Result {
        f.write_str(match canvas {
            Canvas::Foreground => "\x1B[37m", // should be able to use 39 for default by uniterm does not support it
            Canvas::Background => "\x1B[40m", // should be able to use 49 for default by uniterm does not support it
        })
    }
}

/// Something that will have a foreground color applied
pub struct WithForeground<Item, Formatter: FormatColor> {
    item: Item,
    formatter: Formatter,
}

/// Something that will have a background color applied
pub struct WithBackground<Item, Formatter: FormatColor> {
    item: Item,
    formatter: Formatter,
}

/// Adds a foreground or background color
pub trait Colorable: Sized {
    /// Add a background color
    #[allow(dead_code)]
    fn bg<TFormatColor: FormatColor>(
        self,
        formatter: TFormatColor,
    ) -> WithBackground<Self, TFormatColor> {
        WithBackground {
            item: self,
            formatter,
        }
    }

    /// Add a foreground color
    fn fg<TFormatColor: FormatColor>(
        self,
        formatter: TFormatColor,
    ) -> WithForeground<Self, TFormatColor> {
        WithForeground {
            item: self,
            formatter,
        }
    }
}

impl<T> Colorable for T {}

macro_rules! impl_me {
    ($bound:path) => {
        impl<Item: $bound, TFormatColor: FormatColor> $bound
            for WithForeground<Item, TFormatColor>
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.formatter
                    .prelude(f, Canvas::Foreground)
                    .and_then(|_| self.item.fmt(f))
                    .and_then(|_| self.formatter.epilogue(f, Canvas::Foreground))
            }
        }
        impl<Item: $bound, TFormatColor: FormatColor> $bound
            for WithBackground<Item, TFormatColor>
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.formatter
                    .prelude(f, Canvas::Background)
                    .and_then(|_| self.item.fmt(f))
                    .and_then(|_| self.formatter.epilogue(f, Canvas::Background))
            }
        }
    };
}

impl_me!(fmt::Binary);
impl_me!(fmt::Debug);
impl_me!(fmt::Display);
impl_me!(fmt::LowerExp);
impl_me!(fmt::LowerHex);
impl_me!(fmt::Octal);
impl_me!(fmt::Pointer);
impl_me!(fmt::UpperExp);
impl_me!(fmt::UpperHex);

#[allow(dead_code)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Color {
    fn to_num(&self) -> u8 {
        match self {
            Color::Black => 0,
            Color::Red => 1,
            Color::Green => 2,
            Color::Yellow => 3,
            Color::Blue => 4,
            Color::Magenta => 5,
            Color::Cyan => 6,
            Color::White => 7,
        }
    }
}

impl FormatColor for Color {
    fn prelude(&self, f: &mut fmt::Formatter, canvas: Canvas) -> fmt::Result {
        let base = self.to_num();
        match canvas {
            Canvas::Foreground => write!(f, "\x1B[0;{}m", base + 30),
            Canvas::Background => write!(f, "\x1B[{}m", base + 40),
        }
    }
}

#[allow(dead_code)]
pub const fn black() -> Color {
    Color::Black
}

#[allow(dead_code)]
pub const fn red() -> Color {
    Color::Red
}

#[allow(dead_code)]
pub const fn green() -> Color {
    Color::Green
}

#[allow(dead_code)]
pub const fn yellow() -> Color {
    Color::Yellow
}

#[allow(dead_code)]
pub const fn blue() -> Color {
    Color::Blue
}

#[allow(dead_code)]
pub const fn magenta() -> Color {
    Color::Magenta
}

#[allow(dead_code)]
pub const fn cyan() -> Color {
    Color::Cyan
}

#[allow(dead_code)]
pub const fn white() -> Color {
    Color::White
}
