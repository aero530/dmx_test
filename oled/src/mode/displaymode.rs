//! Abstraction of different operating modes

use display_interface::AsyncWriteOnlyDataCommand;

use crate::properties::DisplayProperties;

/// Display mode abstraction
pub struct DisplayMode<MODE>(pub MODE);

/// Trait with core functionality for display mode switching
pub trait DisplayModeTrait<DV, DI> {
    /// Allocate all required data and initialise display for mode
    fn new(properties: DisplayProperties<DV, DI>) -> Self;

    /// Release resources for reuse with different mode
    fn release(self) -> DisplayProperties<DV, DI>;
}

impl<MODE> DisplayMode<MODE> {
    /// Setup display to run in requested mode
    pub fn new<DV, DI>(properties: DisplayProperties<DV, DI>) -> Self
    where
        DI: AsyncWriteOnlyDataCommand,
        MODE: DisplayModeTrait<DV, DI>,
    {
        DisplayMode(MODE::new(properties))
    }

    /// Change into any mode implementing DisplayModeTrait
    // TODO: Figure out how to stay as generic DisplayMode but act as particular mode
    pub fn into<DV, DI, NMODE: DisplayModeTrait<DV, DI>>(self) -> NMODE
    where
        DI: AsyncWriteOnlyDataCommand,
        DV: crate::display::DisplayVariant,
        MODE: DisplayModeTrait<DV, DI>,
    {
        let properties = self.0.release();
        NMODE::new(properties)
    }
}
