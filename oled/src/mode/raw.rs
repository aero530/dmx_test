//! Raw mode for coercion into richer driver types
//!
//! A display driver instance without high level functionality used as a return type from the
//! builder. Used as a source to coerce the driver into richer modes like
//! [`GraphicsMode`](../graphics/index.html).

use display_interface::AsyncWriteOnlyDataCommand;

use crate::{display, mode::displaymode::DisplayModeTrait, properties::DisplayProperties};

/// Raw display mode
pub struct RawMode<DV, DI>
where
    DI: AsyncWriteOnlyDataCommand,
{
    properties: DisplayProperties<DV, DI>,
}

impl<DV, DI> DisplayModeTrait<DV, DI> for RawMode<DV, DI>
where
    DI: AsyncWriteOnlyDataCommand,
{
    /// Create new RawMode instance
    fn new(properties: DisplayProperties<DV, DI>) -> Self {
        RawMode { properties }
    }

    /// Release all resources used by RawMode
    fn release(self) -> DisplayProperties<DV, DI> {
        self.properties
    }
}

impl<DV: display::DisplayVariant, DI: AsyncWriteOnlyDataCommand> RawMode<DV, DI> {
    /// Create a new raw display mode
    pub fn new(properties: DisplayProperties<DV, DI>) -> Self {
        RawMode { properties }
    }
}
