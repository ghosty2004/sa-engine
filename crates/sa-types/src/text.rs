//! Text in the game's own font (`CFont`).

/// The `CFont` state of one string, set in the game's order (scale, colour, justify off, orientation, wrap, centre
/// size, background, background colour, proportional, drop colour, edge or drop shadow, font style), then
/// `PrintString` at (`x`, `y`). Colours are `0xRRGGBBAA`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FontText {
    /// Left (or centre / right edge, by `orientation`) in screen pixels.
    pub x: f32,
    /// Top in screen pixels.
    pub y: f32,
    /// `SetScale`.
    pub scale: (f32, f32),
    /// `SetColor`.
    pub colour: u32,
    /// `SetOrientation`: 0 centre, 1 left, 2 right.
    pub orientation: u8,
    /// `SetWrapx`.
    pub wrap_x: f32,
    /// `SetCentreSize`.
    pub centre_size: f32,
    /// `SetBackground`.
    pub background: bool,
    /// `SetBackgroundColor`.
    pub background_colour: u32,
    /// `SetProportional`.
    pub proportional: bool,
    /// `SetDropColor`.
    pub drop_colour: u32,
    /// `SetEdge` when non-zero, else `SetDropShadowPosition(shadow)`.
    pub edge: u8,
    /// The drop shadow's offset.
    pub shadow: u8,
    /// `SetFontStyle` (0 gothic, 1 subtitles, 2 menu, 3 pricedown).
    pub style: u8,
}
