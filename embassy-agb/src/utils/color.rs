//! Color conversion utilities for GBA RGB15 format

/// Macro to convert hex color codes (#RRGGBB) to GBA RGB15 format
#[macro_export]
macro_rules! rgb15 {
    ($hex:expr) => {{
        const HEX: u32 = $hex;
        const R: u32 = (HEX >> 16) & 0xFF;
        const G: u32 = (HEX >> 8) & 0xFF;
        const B: u32 = HEX & 0xFF;

        const R5: u32 = (R >> 3) & 0x1F;
        const G5: u32 = (G >> 3) & 0x1F;
        const B5: u32 = (B >> 3) & 0x1F;

        ((B5 << 10) | (G5 << 5) | R5) as u16
    }};
}
