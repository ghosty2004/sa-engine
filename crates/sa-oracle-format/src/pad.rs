//! The raw input of one input tick, in the original memory layouts (gta_sa.exe 1.0 US).
//!
//! An input tick is one `CPad::UpdatePads` (0x541DD0) pass, which runs, in order (disassembly of
//! 0x541DD7..0x541E3C): `CPad::UpdateMouse` (0x53F3C0) → `ProcessPad` → `ControlsManager`
//! keyboard/mouse mapping → `CPad::Update(0)` (0x541C40) → `CPad::Update(1)` → `OldKeyState =
//! NewKeyState; NewKeyState = TempKeyState`.
//!
//! The recorder captures, and the replayer injects, at these points:
//! - `temp_key_state` (`CPad::TempKeyState`, 0xB72CB0, `CKeyboardState` 0x270 bytes): at the entry of
//!   `UpdateMouse`, i.e. before anything of this pass reads it;
//! - `new_mouse` (`CPad::NewMouseControllerState`, 0xB73418, 0x14 bytes): right after `UpdateMouse`
//!   returned (it is the DirectInput state the camera reads);
//! - `pc_temp_key`/`pc_temp_joy`/`pc_temp_mouse` (`CPad::Pads[0]` +0x78/+0xA8/+0xD8, `CControllerState`
//!   0x30 bytes each, offsets from `CPad::Update` 0x541C49..0x541CAB): at the entry of
//!   `CPad::Update(0)`, which reconciles them into `NewState`.

pub const KEY_STATE_LEN: usize = 0x270;
pub const MOUSE_STATE_LEN: usize = 0x14;
pub const CONTROLLER_STATE_LEN: usize = 0x30;
pub const PAD_INPUT_LEN: usize = KEY_STATE_LEN + MOUSE_STATE_LEN + 3 * CONTROLLER_STATE_LEN;

/// Game addresses of the injected blocks.
pub mod addr {
    pub const TEMP_KEY_STATE: u32 = 0xB72CB0;
    pub const NEW_MOUSE_STATE: u32 = 0xB73418;
    pub const PAD0: u32 = 0xB73458;
    pub const PC_TEMP_KEY: u32 = 0x78;
    pub const PC_TEMP_JOY: u32 = 0xA8;
    pub const PC_TEMP_MOUSE: u32 = 0xD8;
}

/// `CControllerState` field offsets (ControllerState.h, int16 each).
pub mod ctrl {
    pub const LEFT_STICK_X: usize = 0x00;
    pub const LEFT_STICK_Y: usize = 0x02;
    pub const RIGHT_STICK_X: usize = 0x04;
    pub const RIGHT_STICK_Y: usize = 0x06;
    pub const LEFT_SHOULDER1: usize = 0x08;
    pub const LEFT_SHOULDER2: usize = 0x0A;
    pub const RIGHT_SHOULDER1: usize = 0x0C;
    pub const RIGHT_SHOULDER2: usize = 0x0E;
    pub const DPAD_UP: usize = 0x10;
    pub const DPAD_DOWN: usize = 0x12;
    pub const DPAD_LEFT: usize = 0x14;
    pub const DPAD_RIGHT: usize = 0x16;
    pub const START: usize = 0x18;
    pub const SELECT: usize = 0x1A;
    pub const SQUARE: usize = 0x1C;
    pub const TRIANGLE: usize = 0x1E;
    pub const CROSS: usize = 0x20;
    pub const CIRCLE: usize = 0x22;
    pub const SHOCK_L: usize = 0x24;
    pub const SHOCK_R: usize = 0x26;
    pub const PED_WALK: usize = 0x2A;
}

/// `CKeyboardState` offsets (KeyboardState.h: FKeys[12], standardKeys[256], then named keys, int16 each).
/// `enter` 0x23C and `extenter` 0x25A are confirmed by `CPad::IsEnterJustPressed` (0x4D5980 reads
/// NewKeyState 0xB73190 + 0x23C / + 0x25A).
pub mod key {
    pub const F1: usize = 0x00;
    pub const STANDARD: usize = 0x18;
    pub const ESC: usize = 0x218;
    pub const ENTER: usize = 0x23C;
    pub const TAB: usize = 0x256;
    pub const EXTENTER: usize = 0x25A;
    pub const LALT: usize = 0x266;
}

/// `CMouseControllerState`: 7 button bools at +0..+6, wheel f32 +8, AmountMoved x/y f32 at +0xC/+0x10.
pub mod mouse {
    pub const LMB: usize = 0;
    pub const RMB: usize = 1;
    pub const MOVED_X: usize = 0x0C;
    pub const MOVED_Y: usize = 0x10;
}

#[derive(Clone, PartialEq, Eq)]
pub struct PadInput {
    pub temp_key_state: [u8; KEY_STATE_LEN],
    pub new_mouse: [u8; MOUSE_STATE_LEN],
    pub pc_temp_key: [u8; CONTROLLER_STATE_LEN],
    pub pc_temp_joy: [u8; CONTROLLER_STATE_LEN],
    pub pc_temp_mouse: [u8; CONTROLLER_STATE_LEN],
}

impl Default for PadInput {
    fn default() -> Self {
        PadInput {
            temp_key_state: [0; KEY_STATE_LEN],
            new_mouse: [0; MOUSE_STATE_LEN],
            pc_temp_key: [0; CONTROLLER_STATE_LEN],
            pc_temp_joy: [0; CONTROLLER_STATE_LEN],
            pc_temp_mouse: [0; CONTROLLER_STATE_LEN],
        }
    }
}

impl std::fmt::Debug for PadInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PadInput(fnv={:016x})", crate::fnv::hash(&self.to_bytes()))
    }
}

fn set_i16(b: &mut [u8], off: usize, v: i16) {
    b[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

impl PadInput {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(PAD_INPUT_LEN);
        v.extend_from_slice(&self.temp_key_state);
        v.extend_from_slice(&self.new_mouse);
        v.extend_from_slice(&self.pc_temp_key);
        v.extend_from_slice(&self.pc_temp_joy);
        v.extend_from_slice(&self.pc_temp_mouse);
        v
    }

    pub fn from_bytes(b: &[u8]) -> Option<PadInput> {
        if b.len() != PAD_INPUT_LEN {
            return None;
        }
        let mut p = PadInput::default();
        let (a, rest) = b.split_at(KEY_STATE_LEN);
        p.temp_key_state.copy_from_slice(a);
        let (a, rest) = rest.split_at(MOUSE_STATE_LEN);
        p.new_mouse.copy_from_slice(a);
        let (a, rest) = rest.split_at(CONTROLLER_STATE_LEN);
        p.pc_temp_key.copy_from_slice(a);
        let (a, rest) = rest.split_at(CONTROLLER_STATE_LEN);
        p.pc_temp_joy.copy_from_slice(a);
        p.pc_temp_mouse.copy_from_slice(rest);
        Some(p)
    }

    /// Sets a `CControllerState` field of `pc_temp_key` (the keyboard-mapped pad state).
    pub fn set_ctrl(&mut self, off: usize, v: i16) {
        set_i16(&mut self.pc_temp_key, off, v);
    }

    /// Presses a key in `temp_key_state` (255, the value WndProc stores for a held key).
    pub fn set_key(&mut self, off: usize, v: i16) {
        set_i16(&mut self.temp_key_state, off, v);
    }

    pub fn set_mouse_moved(&mut self, dx: f32, dy: f32) {
        self.new_mouse[mouse::MOVED_X..mouse::MOVED_X + 4].copy_from_slice(&dx.to_le_bytes());
        self.new_mouse[mouse::MOVED_Y..mouse::MOVED_Y + 4].copy_from_slice(&dy.to_le_bytes());
    }

    pub fn set_mouse_button(&mut self, off: usize, down: bool) {
        self.new_mouse[off] = down as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout() {
        assert_eq!(PAD_INPUT_LEN, 788);
        // Named keys follow standardKeys[256]: esc is the first, enter the 19th.
        assert_eq!(key::ESC, key::STANDARD + 512);
        assert_eq!(key::ENTER, key::ESC + 18 * 2);
        assert_eq!(key::EXTENTER, key::ESC + 33 * 2);
        assert_eq!(key::TAB, key::ESC + 31 * 2);
        assert_eq!(key::LALT, key::ESC + 39 * 2);
        let mut p = PadInput::default();
        p.set_ctrl(ctrl::LEFT_STICK_Y, -128);
        p.set_key(key::ENTER, 255);
        p.set_mouse_moved(3.0, -1.0);
        let q = PadInput::from_bytes(&p.to_bytes()).unwrap();
        assert_eq!(p, q);
        assert_eq!(&q.pc_temp_key[2..4], &(-128i16).to_le_bytes());
        assert!(PadInput::from_bytes(&[0; 3]).is_none());
    }
}
