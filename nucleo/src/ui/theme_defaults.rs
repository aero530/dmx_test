//! UI theme default values
//!
//! Colors and fonts
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::RgbColor;
use embedded_graphics::prelude::WebColors;
use u8g2_fonts::fonts;

/// Text color
pub const COLOR_DEFAULT_TEXT: Rgb565 = Rgb565::WHITE;

/// Menu color
pub const COLOR_MENU_TEXT: Rgb565 = Rgb565::CSS_CORAL;

/// MenuItem name color
pub const COLOR_ITEM_TEXT: Rgb565 = Rgb565::CSS_CORNFLOWER_BLUE;

/// MenuValue color
pub const COLOR_VALUE_TEXT: Rgb565 = Rgb565::CSS_CORNFLOWER_BLUE;

/// Selected text color
pub const COLOR_SELECTED_TEXT: Rgb565 = Rgb565::CSS_BLUE_VIOLET;

/// Editing text color
pub const COLOR_EDITING_TEXT: Rgb565 = Rgb565::CSS_DEEP_PINK;

/// Default font
/// https://github.com/olikraus/u8g2/wiki/fntlistmono#24-pixel-height
pub const DEFAULT_FONT: fonts::u8g2_font_inr16_mf = fonts::u8g2_font_inr16_mf;
