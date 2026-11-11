//! Game pads through gilrs (evdev on Linux, XInput/WGI on Windows), as [`GamepadState`]s in
//! connection order (the order DirectInput's `EnumDevices(DI8DEVCLASS_GAMECTRL)` would list them,
//! which `Input::EnumDevicesCallback` turns into pads 1 and 2).

use gilrs::{Axis, Button, Gilrs};

use crate::input::{GamepadState, PadButton};

pub struct Pads {
    gilrs: Gilrs,
}

impl Pads {
    pub fn new() -> Result<Pads, String> {
        Gilrs::new()
            .map(|gilrs| Pads { gilrs })
            .map_err(|e| e.to_string())
    }

    /// Drains pending events and returns the state of every connected pad.
    pub fn poll(&mut self) -> Vec<GamepadState> {
        while self.gilrs.next_event().is_some() {}
        let mut ids: Vec<_> = self.gilrs.gamepads().map(|(id, _)| id).collect();
        ids.sort_by_key(|id| usize::from(*id));
        ids.into_iter()
            .map(|id| {
                let g = self.gilrs.gamepad(id);
                let ax = |a: Axis| g.axis_data(a).map_or(0.0, |d| d.value());
                let bt = |b: Button| g.button_data(b).map_or(0.0, |d| d.value());
                let mut s = GamepadState {
                    name: g.name().to_owned(),
                    // gilrs: y up positive; DirectInput: y down positive
                    left: (ax(Axis::LeftStickX), -ax(Axis::LeftStickY)),
                    right: (ax(Axis::RightStickX), -ax(Axis::RightStickY)),
                    left_trigger: bt(Button::LeftTrigger2),
                    right_trigger: bt(Button::RightTrigger2),
                    buttons: [false; PadButton::COUNT],
                };
                let map = [
                    (Button::South, PadButton::South),
                    (Button::East, PadButton::East),
                    (Button::West, PadButton::West),
                    (Button::North, PadButton::North),
                    (Button::LeftTrigger, PadButton::LeftShoulder),
                    (Button::RightTrigger, PadButton::RightShoulder),
                    (Button::Select, PadButton::Select),
                    (Button::Start, PadButton::Start),
                    (Button::LeftThumb, PadButton::LeftStick),
                    (Button::RightThumb, PadButton::RightStick),
                    (Button::DPadUp, PadButton::DPadUp),
                    (Button::DPadDown, PadButton::DPadDown),
                    (Button::DPadLeft, PadButton::DPadLeft),
                    (Button::DPadRight, PadButton::DPadRight),
                    (Button::Mode, PadButton::Guide),
                ];
                for (b, p) in map {
                    s.buttons[p as usize] = g.is_pressed(b);
                }
                s
            })
            .collect()
    }
}
