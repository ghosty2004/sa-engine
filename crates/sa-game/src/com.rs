//! Native DirectInput 8 COM objects for the ported input code.
//!
//! The ports call COM methods through `orig!` `vslot` declarations: the method address is read
//! from the object's vtable at call time. sa-game maps one page at [`PAGE`] holding the objects
//! (the `IDirectInput8A` interface and the mouse and two pad devices, at the pseudo addresses
//! [`crate::bind`] already uses) and their vtables, whose slots are trap addresses registered in
//! `orig!`'s address registry: an implemented method dispatches to the native function below, any
//! other fails loudly as `IDirectInputDevice8A::<method>`.
//!
//! Semantics are DirectInput 8's as the game observes them. Mouse capabilities: 3 axes and
//! 5 buttons (a wheel mouse with two side buttons), see [`MOUSE_BUTTONS`].

#![allow(unsafe_code)]

use sa_core::mem::GPtr;

use crate::bind::{self, DI_MOUSE, DI_PAD1, DI_PAD2, with};

/// The page holding the objects and vtables.
pub const PAGE: usize = 0x0003_0000;
/// The `IDirectInput8A` object `DirectInput8Create` returns.
pub const DI8: usize = 0x0003_0030;
const VT_DI8: usize = 0x0003_0400;
const VT_DEVICE: usize = 0x0003_0600;
/// Method trap addresses: `TRAP + 0x100 * class + slot offset`.
const TRAP: u32 = 0x0004_0000;
/// Buttons the mouse reports (`DIDEVCAPS.dwButtons`). Not measured on the phase A setup: phase A's
/// Wine dinput reports its own count; recorded as a decision in docs/arch/platform.md.
pub const MOUSE_BUTTONS: u32 = 5;

const DI8_METHODS: [&str; 11] = [
    "QueryInterface",
    "AddRef",
    "Release",
    "CreateDevice",
    "EnumDevices",
    "GetDeviceStatus",
    "RunControlPanel",
    "Initialize",
    "FindDevice",
    "EnumDevicesBySemantics",
    "ConfigureDevices",
];
const DEVICE_METHODS: [&str; 32] = [
    "QueryInterface",
    "AddRef",
    "Release",
    "GetCapabilities",
    "EnumObjects",
    "GetProperty",
    "SetProperty",
    "Acquire",
    "Unacquire",
    "GetDeviceState",
    "GetDeviceData",
    "SetDataFormat",
    "SetEventNotification",
    "SetCooperativeLevel",
    "GetObjectInfo",
    "GetDeviceInfo",
    "RunControlPanel",
    "Initialize",
    "CreateEffect",
    "EnumEffects",
    "GetEffectInfo",
    "GetForceFeedbackState",
    "SendForceFeedbackCommand",
    "EnumCreatedEffectObjects",
    "Escape",
    "Poll",
    "SendDeviceData",
    "EnumEffectsInFile",
    "WriteEffectToFile",
    "BuildActionMap",
    "SetActionMap",
    "GetImageInfo",
];

fn trap(class: u32, slot: usize) -> u32 {
    TRAP + 0x100 * class + 4 * slot as u32
}

/// Maps the page and writes the objects' vtable pointers and the vtables.
pub fn map() -> Result<(), String> {
    crate::arena::map_fixed(PAGE, 0x1000).map_err(|e| format!("cannot map the DirectInput object page at {PAGE:#x}: {e}"))?;
    let w = |a: usize, v: u32| GPtr::<u32>::from_addr(a).write(v);
    w(DI8, VT_DI8 as u32);
    for obj in [DI_MOUSE, DI_PAD1, DI_PAD2] {
        w(obj, VT_DEVICE as u32);
    }
    for i in 0..DI8_METHODS.len() {
        w(VT_DI8 + 4 * i, trap(0, i));
    }
    for i in 0..DEVICE_METHODS.len() {
        w(VT_DEVICE + 4 * i, trap(1, i));
    }
    Ok(())
}

/// Every method trap's name (`IDirectInput8A::CreateDevice`, …).
pub fn names() -> Vec<(u32, &'static str)> {
    let mut v = Vec::new();
    for (i, m) in DI8_METHODS.iter().enumerate() {
        v.push((trap(0, i), &*Box::leak(format!("IDirectInput8A::{m}").into_boxed_str())));
    }
    for (i, m) in DEVICE_METHODS.iter().enumerate() {
        v.push((trap(1, i), &*Box::leak(format!("IDirectInputDevice8A::{m}").into_boxed_str())));
    }
    v
}

macro_rules! methods {
    ($( ($class:literal, $slot:literal) => $f:path : fn($($a:ident : $t:ty),*) -> $r:ty ;)*) => {
        /// `(trap, name, native entry)` of every implemented method.
        pub fn entries() -> Vec<(u32, &'static str, usize)> {
            let mut v: Vec<(u32, &'static str, usize)> = Vec::new();
            $({
                #[cfg(target_arch = "x86")]
                extern "stdcall-unwind" fn w($($a: $t),*) -> $r {
                    $f($($a),*)
                }
                #[cfg(not(target_arch = "x86"))]
                extern "C-unwind" fn w($($a: $t),*) -> $r {
                    $f($($a),*)
                }
                let name = if $class == 0 { DI8_METHODS.get($slot) } else { DEVICE_METHODS.get($slot) }.copied().unwrap_or("?");
                v.push((trap($class, $slot), name, {
                    #[cfg(target_arch = "wasm32")]
                    let e = sa_core::wabi_wrap!(w; $($a)*);
                    #[cfg(not(target_arch = "wasm32"))]
                    let e = w as *const () as usize;
                    e
                }));
            })*
            v
        }
    };
}

methods! {
    (0, 1) => add_ref: fn(this: u32) -> u32;
    (0, 2) => release: fn(this: u32) -> u32;
    (0, 3) => create_device: fn(this: u32, guid: GPtr<[u32; 4]>, out: GPtr<u32>, outer: u32) -> i32;
    (0, 4) => enum_devices: fn(this: u32, class: u32, cb: u32, rf: u32, flags: u32) -> i32;
    (1, 1) => add_ref: fn(this: u32) -> u32;
    (1, 2) => release: fn(this: u32) -> u32;
    (1, 3) => get_capabilities: fn(this: u32, caps: GPtr<u32>) -> i32;
    (1, 5) => get_property: fn(this: u32, prop: u32, hdr: GPtr<u32>) -> i32;
    (1, 6) => set_property: fn(this: u32, prop: u32, hdr: GPtr<u32>) -> i32;
    (1, 7) => ok1: fn(this: u32) -> i32;
    (1, 8) => ok1: fn(this: u32) -> i32;
    (1, 9) => get_device_state: fn(this: u32, size: u32, data: GPtr<u8>) -> i32;
    (1, 11) => ok2: fn(this: u32, format: u32) -> i32;
    (1, 13) => ok3: fn(this: u32, hwnd: u32, flags: u32) -> i32;
    (1, 25) => ok1: fn(this: u32) -> i32;
}

fn add_ref(_this: u32) -> u32 {
    1
}
fn release(_this: u32) -> u32 {
    0
}
fn ok1(_this: u32) -> i32 {
    0
}
fn ok2(_this: u32, _a: u32) -> i32 {
    0
}
fn ok3(_this: u32, _a: u32, _b: u32) -> i32 {
    0
}

/// `GUID_SysMouse` {6F1D2B60-D5A0-11CF-BFC7-444553540000}.
const GUID_SYS_MOUSE: [u32; 4] = [0x6F1D_2B60, 0x11CF_D5A0, 0x4544_C7BF, 0x0000_5453];
/// Instance GUIDs handed to `EnumDevices` callbacks for pad 1 / pad 2 (made up, unique).
const GUID_PAD: [[u32; 4]; 2] = [
    [0x5341_0001, 0x0001_0000, 0, 0x5341_4750],
    [0x5341_0002, 0x0001_0000, 0, 0x5341_4750],
];

fn create_device(_this: u32, guid: GPtr<[u32; 4]>, out: GPtr<u32>, _outer: u32) -> i32 {
    let g = guid.read();
    let obj = if g == GUID_SYS_MOUSE {
        DI_MOUSE
    } else if g == GUID_PAD[0] {
        DI_PAD1
    } else if g == GUID_PAD[1] {
        DI_PAD2
    } else {
        return 0x8007_0002u32 as i32; // DIERR_DEVICENOTREG
    };
    with("IDirectInput8A::CreateDevice", |_| ());
    out.write(obj as u32);
    0
}

sa_core::orig! {
    /// `LPDIENUMDEVICESCALLBACKA` at a run-time address (`BOOL CALLBACK (const DIDEVICEINSTANCEA*, void*)`)
    fn enum_callback(f: u32, lpddi: GPtr<u8>, rf: u32) -> i32 = stdcall dyn => unported;
}

/// `EnumDevices(DI8DEVCLASS_GAMECTRL, cb, ref, DIEDFL_ATTACHEDONLY)`: one `DIDEVICEINSTANCEA`
/// (0x244 bytes, `guidInstance` at +4) per connected pad, in connection order, until the
/// callback returns `DIENUM_STOP` (0).
fn enum_devices(_this: u32, class: u32, cb: u32, rf: u32, _flags: u32) -> i32 {
    let pads = with("IDirectInput8A::EnumDevices", |p| p.input.pads.len());
    if class != 4 {
        return 0;
    }
    for g in GUID_PAD.iter().take(pads) {
        let mut inst = vec![0u8; 0x244];
        inst[..4].copy_from_slice(&0x244u32.to_le_bytes());
        for (i, w) in g.iter().enumerate() {
            inst[4 + 4 * i..8 + 4 * i].copy_from_slice(&w.to_le_bytes());
        }
        // dwDevType: DI8DEVTYPE_GAMEPAD (0x15) | standard subtype (0x02 << 8)
        inst[0x24..0x28].copy_from_slice(&0x0215u32.to_le_bytes());
        if enum_callback(cb, GPtr::from_raw(inst.as_mut_ptr()), rf) == 0 {
            break;
        }
    }
    0
}

/// `GetCapabilities(DIDEVCAPS*)`: +4 dwFlags (`DIDC_ATTACHED`), +8 dwDevType, +0xC dwAxes,
/// +0x10 dwButtons, +0x14 dwPOVs.
fn get_capabilities(this: u32, caps: GPtr<u32>) -> i32 {
    let (ty, axes, buttons, povs) = if this as usize == DI_MOUSE {
        (0x0112, 3, MOUSE_BUTTONS, 0)
    } else {
        (0x0215, 6, 10, 1)
    };
    for (i, v) in [1u32, ty, axes, buttons, povs].iter().enumerate() {
        caps.wrapping_index(1 + i as i32).write(*v);
    }
    0
}
/// `GetProperty`: `DIPROP_VIDPID` (0x18) of a pad: an Xbox 360 controller (VID 0x045E, PID 0x028E),
/// what gilrs pads present as through XInput-compatible drivers; anything else `DIERR_UNSUPPORTED`.
fn get_property(this: u32, prop: u32, hdr: GPtr<u32>) -> i32 {
    if prop == 0x18 && this as usize != DI_MOUSE {
        hdr.wrapping_index(4).write(0x028E_045E);
        return 0;
    }
    0x8000_4001u32 as i32
}
fn set_property(_this: u32, _prop: u32, _hdr: GPtr<u32>) -> i32 {
    0
}
fn get_device_state(this: u32, size: u32, data: GPtr<u8>) -> i32 {
    bind::did_get_device_state(GPtr::from_addr(this as usize), size, data)
}

/// `DirectInput8Create(hinst, version, riid, out, outer)` as WinMain reaches it (the thunk
/// 0x817523 `jmp [0x8E3EC4]` into dinput8.dll): the [`DI8`] object, `DI_OK`.
#[cfg(target_arch = "x86")]
extern "stdcall-unwind" fn direct_input8_create(inst: u32, version: u32, iid: u32, out: GPtr<u32>, outer: u32) -> i32 {
    di8_create(inst, version, iid, out, outer)
}
#[cfg(not(target_arch = "x86"))]
extern "C-unwind" fn direct_input8_create(inst: u32, version: u32, iid: u32, out: GPtr<u32>, outer: u32) -> i32 {
    di8_create(inst, version, iid, out, outer)
}
fn di8_create(_inst: u32, _version: u32, _iid: u32, out: GPtr<u32>, _outer: u32) -> i32 {
    with("DirectInput8Create", |_| ());
    out.write(DI8 as u32);
    0
}

/// `(address, name, native entry)` of the DirectInput entry points reached by address.
pub fn thunks() -> Vec<(u32, &'static str, usize)> {
    vec![(
        0x0081_7523,
        "DirectInput8Create (dinput8 thunk)",
        {
            #[cfg(target_arch = "wasm32")]
            let e = sa_core::wabi_wrap!(direct_input8_create; inst version iid out outer);
            #[cfg(not(target_arch = "wasm32"))]
            let e = direct_input8_create as *const () as usize;
            e
        },
    )]
}
