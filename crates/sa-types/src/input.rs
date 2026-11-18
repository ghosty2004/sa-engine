//! The game's controller state.

/// One pad state (`CControllerState`, 0x30 bytes; `CPad::Pads` 0xB73458: NewState +0x0, OldState +0x30).
/// Values are the game's raw shorts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadState {
    /// Left stick x.
    pub left_x: i16,
    /// Left stick y.
    pub left_y: i16,
    /// Right stick x.
    pub right_x: i16,
    /// Right stick y.
    pub right_y: i16,
    /// Shoulders: L1, L2, R1, R2.
    pub shoulders: [i16; 4],
    /// D-pad: up, down, left, right (`+0x10…`).
    pub dpad: [i16; 4],
    /// Start, select.
    pub start_select: [i16; 2],
    /// Square, triangle (+0x1E), cross, circle.
    pub face: [i16; 4],
    /// Left / right shock (stick press, `Pads + 0x24` 0xB7347C for L).
    pub shock: [i16; 2],
    /// `m_bChatIndicated` (+0x28).
    pub chat_indicated: i16,
    /// `m_bPedWalk` (+0x2A): the walk key, sent by samp's on-foot sync.
    pub ped_walk: i16,
}
