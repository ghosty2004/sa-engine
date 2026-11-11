//! PORT-RW-4 module `d3d9light`: the D3D9 light plugin and the fixed-function light enables
//! (0x755B90..0x756700). Ported from the 1.0 US bytes; `docs/re/specs/RW/RwD3D9Lights*.md` as reading aids.
//!
//! Light extension (`light + [0xC926B0]`, 0x10): +0 D3D index (-1 = none), +4/+8/+0xC attenuation
//! (constant, linear, quadratic). State (raw cells, shared): 0xC926B4 max active lights, free-index stack
//! (count 0xC926B8, capacity 0xC926BC, array 0xC926C4), next index 0xC926C0, current lights (array
//! 0xC926C8, count 0xC926CC, capacity 0xC926D0), previous lights (0xC926D4 / 0xC926D8 / 0xC926DC),
//! `AmbientSaturated` 0x8E2418..0x8E2424, D3DLIGHT9 templates 0xC92648 (directional) and 0xC925E0 (local).
//!
//! `_rwD3D9LightsEnable` ends with a tail jump to `RwD3D9SetRenderState` after storing its arguments over
//! its own argument slots (caller stack, not reproduced: the port calls it normally).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, p8, rd, rw_free, rw_globals, rw_malloc, rw_realloc, wr, x};

/// Plugin offset of the light extension.
const LIGHT_EXT_OFFSET: usize = 0xC926B0;
const MAX_ACTIVE: usize = 0xC926B4;
const FREE_COUNT: usize = 0xC926B8;
const FREE_CAP: usize = 0xC926BC;
const NEXT_INDEX: usize = 0xC926C0;
const FREE_ARRAY: usize = 0xC926C4;
const CUR_ARRAY: usize = 0xC926C8;
const CUR_COUNT: usize = 0xC926CC;
const CUR_CAP: usize = 0xC926D0;
const PREV_ARRAY: usize = 0xC926D4;
const PREV_COUNT: usize = 0xC926D8;
const PREV_CAP: usize = 0xC926DC;
/// `AmbientSaturated` (RwRGBAReal).
const AMBIENT_SAT: u32 = 0x8E2418;
/// Directional D3DLIGHT9 template.
const DIR_TEMPLATE: u32 = 0xC92648;
/// Local D3DLIGHT9 template.
const LOCAL_TEMPLATE: u32 = 0xC925E0;
/// Allocation hint of the index arrays.
const HINT: u32 = 0x0103_050D;
const ONE_BITS: u32 = 0x3F80_0000;
/// 0.0f (0x858B50).
const ZERO_ADDR: usize = 0x858B50;
/// pi (0x858CB8).
const PI_ADDR: usize = 0x858CB8;
/// 0.001f (0x858CDC).
const MILLI_ADDR: usize = 0x858CDC;
/// Plugin callbacks.
const CB_CTOR: u32 = 0x755BC0;
const CB_DTOR: u32 = 0x755BF0;
const CB_COPY: u32 = 0x755CE0;

orig! {
    /// `RpLightRegisterPlugin(size, id, ctor, dtor, copy)` (0x751D60)
    fn rp_light_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x751D60 => crate::light::rp_light_register_plugin;
    /// `RwD3D9GetCaps()` (0x7FAD20)
    fn rw_d3d9_get_caps() -> GPtr<u8> = cdecl 0x7FAD20 => unported;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn rw_frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `RwD3D9SetLight(index, light)` (0x7FA660)
    fn rw_d3d9_set_light(index: i32, light: GPtr<u8>) -> i32 = cdecl 0x7FA660 => unported;
    /// `RwD3D9EnableLight(index, enable)` (0x7FA860)
    fn rw_d3d9_enable_light(index: i32, enable: i32) -> i32 = cdecl 0x7FA860 => unported;
    /// `RwD3D9SetRenderState(state, value)` (0x7FC2D0)
    fn rw_d3d9_set_render_state(state: u32, value: u32) -> i32 = cdecl 0x7FC2D0 => unported;
    /// `RpLightGetConeAngle(light)` (0x751AE0), result in ST0
    fn rp_light_get_cone_angle(l: GPtr<u8>) -> f64 = cdecl 0x751AE0 => unported;
}

fn ext(l: GPtr<u8>) -> GPtr<u8> {
    l.byte_add(g::<i32>(LIGHT_EXT_OFFSET).read() as isize)
}

/// Gives the light's extension a D3D index if it has none (free stack first, else the next new index).
fn alloc_index(e: GPtr<u8>) {
    if rd::<i32>(e, 0) >= 0 {
        return;
    }
    let n = g::<u32>(FREE_COUNT).read();
    if n != 0 {
        let n = n.wrapping_sub(1);
        g::<u32>(FREE_COUNT).write(n);
        let arr = p8(g::<u32>(FREE_ARRAY).read());
        wr::<u32>(e, 0, rd::<u32>(arr, n as isize * 4));
    } else {
        wr::<u32>(e, 0, g::<u32>(NEXT_INDEX).read());
        let c = g::<u32>(NEXT_INDEX);
        c.write(c.read().wrapping_add(1));
    }
}

/// Appends `index` to the current light list (capacity += 8, malloc/realloc, new slots zeroed).
fn append_current(index: u32) {
    let cap = g::<u32>(CUR_CAP).read();
    if g::<u32>(CUR_COUNT).read() >= cap {
        let arr = g::<u32>(CUR_ARRAY).read();
        let mut i = cap;
        let ncap = cap.wrapping_add(8);
        g::<u32>(CUR_CAP).write(ncap);
        let p = if arr == 0 {
            rw_malloc(ncap.wrapping_mul(4), HINT)
        } else {
            rw_realloc(p8(arr), ncap.wrapping_mul(4), HINT)
        };
        g::<u32>(CUR_ARRAY).write(p.addr() as u32);
        while i < g::<u32>(CUR_CAP).read() {
            i += 1;
            wr::<u32>(p8(g::<u32>(CUR_ARRAY).read()), (i as isize - 1) * 4, 0);
        }
    }
    let n = g::<u32>(CUR_COUNT).read();
    wr::<u32>(p8(g::<u32>(CUR_ARRAY).read()), n as isize * 4, index);
    g::<u32>(CUR_COUNT).write(g::<u32>(CUR_COUNT).read().wrapping_add(1));
}

/// Directional light body shared by `_rwD3D9LightDirectionalEnable` and `_rwD3D9LightsGlobalEnable`:
/// index, template diffuse rgb and direction (LTM at), `RwD3D9SetLight`, append to the current list.
fn directional(l: GPtr<u8>) {
    let e = ext(l);
    alloc_index(e);
    let t = p8(DIR_TEMPLATE);
    wr::<u32>(t, 0x4, rd::<u32>(l, 0x18));
    wr::<u32>(t, 0x8, rd::<u32>(l, 0x1C));
    wr::<u32>(t, 0xC, rd::<u32>(l, 0x20));
    let ltm = rw_frame_get_ltm(p8(rd::<u32>(l, 4)));
    wr::<u32>(t, 0x40, rd::<u32>(ltm, 0x20));
    wr::<u32>(t, 0x44, rd::<u32>(ltm, 0x24));
    wr::<u32>(t, 0x48, rd::<u32>(ltm, 0x28));
    rw_d3d9_set_light(rd::<i32>(e, 0), t);
    append_current(rd::<u32>(e, 0));
}

/// Frees and clears the three index arrays and their counters (open/close prologue).
fn free_lists() {
    let a = g::<u32>(FREE_ARRAY).read();
    if a != 0 {
        rw_free(p8(a));
        g::<u32>(FREE_ARRAY).write(0);
    }
    g::<u32>(FREE_COUNT).write(0);
    g::<u32>(FREE_CAP).write(0);
    g::<u32>(NEXT_INDEX).write(0);
    let a = g::<u32>(CUR_ARRAY).read();
    if a != 0 {
        rw_free(p8(a));
        g::<u32>(CUR_ARRAY).write(0);
    }
    g::<u32>(CUR_COUNT).write(0);
    g::<u32>(CUR_CAP).write(0);
    let a = g::<u32>(PREV_ARRAY).read();
    if a != 0 {
        rw_free(p8(a));
        g::<u32>(PREV_ARRAY).write(0);
    }
    g::<u32>(PREV_COUNT).write(0);
    g::<u32>(PREV_CAP).write(0);
}

/// `_rpD3D9LightRegisterPlugin()` (0x755B90): the 0x10-byte extension (id 0x505); returns `offset >= 0`.
#[hook(addr = 0x755B90, conv = cdecl, name = "_rpD3D9LightRegisterPlugin", expect = "68 E0 5C 75 00")]
pub fn rp_d3d9_light_register_plugin() -> u32 {
    let off = rp_light_register_plugin(0x10, 0x505, CB_CTOR, CB_DTOR, CB_COPY);
    g::<i32>(LIGHT_EXT_OFFSET).write(off);
    (off >= 0) as u32
}

/// `RpD3D9LightSetAttenuationParams(light, params)` (0x755D20): 3 dwords to ext+4. EAX = params[2].
#[hook(addr = 0x755D20, conv = cdecl, name = "RpD3D9LightSetAttenuationParams", expect = "8B 44 24 04 8B 0D B0 26 C9 00")]
pub fn rp_d3d9_light_set_attenuation_params(l: GPtr<u8>, params: GPtr<u8>) -> u32 {
    let d = ext(l).byte_add(4);
    wr::<u32>(d, 0, rd::<u32>(params, 0));
    wr::<u32>(d, 4, rd::<u32>(params, 4));
    let v = rd::<u32>(params, 8);
    wr::<u32>(d, 8, v);
    v
}

/// `RpD3D9LightGetAttenuationParams(light, params)` (0x755D50): 3 dwords from ext+4. EAX = params.
#[hook(addr = 0x755D50, conv = cdecl, name = "RpD3D9LightGetAttenuationParams", expect = "8B 44 24 04 8B 0D B0 26 C9 00")]
pub fn rp_d3d9_light_get_attenuation_params(l: GPtr<u8>, params: GPtr<u8>) -> GPtr<u8> {
    let s = ext(l).byte_add(4);
    wr::<u32>(params, 0, rd::<u32>(s, 0));
    wr::<u32>(params, 4, rd::<u32>(s, 4));
    wr::<u32>(params, 8, rd::<u32>(s, 8));
    params
}

/// `_rwD3D9LightsOpen()` (0x755D80): lists freed and cleared; max active lights from the caps (0 ->
/// 0xFFFFFFFF); AmbientSaturated (0, 0, 0, 1); the directional template fully initialised (type 3) and the
/// local template's colours and falloff. Returns 1.
#[hook(addr = 0x755D80, conv = cdecl, name = "_rwD3D9LightsOpen", expect = "A1 C4 26 C9 00")]
pub fn rw_d3d9_lights_open() -> u32 {
    free_lists();
    let caps = rw_d3d9_get_caps();
    let m = rd::<u32>(caps, 0xA0);
    g::<u32>(MAX_ACTIVE).write(m);
    if m == 0 {
        g::<u32>(MAX_ACTIVE).write(0xFFFF_FFFF);
    }
    let a = p8(AMBIENT_SAT);
    wr::<u32>(a, 0, 0);
    wr::<u32>(a, 4, 0);
    wr::<u32>(a, 8, 0);
    wr::<u32>(a, 0xC, ONE_BITS);
    let d = p8(DIR_TEMPLATE);
    // Store order of 0x755E50..0x755F4A (offsets from 0xC92648).
    let dir: [(isize, u32); 26] = [
        (0x0, 3),
        (0x4, 0),
        (0x8, 0),
        (0xC, 0),
        (0x10, ONE_BITS),
        (0x40, 0),
        (0x44, 0),
        (0x48, 0),
        (0x14, 0),
        (0x18, 0),
        (0x1C, 0),
        (0x20, ONE_BITS),
        (0x24, 0),
        (0x28, 0),
        (0x2C, 0),
        (0x30, ONE_BITS),
        (0x34, 0),
        (0x38, 0),
        (0x3C, 0),
        (0x4C, 0),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5C, 0),
        (0x60, 0),
        (0x64, 0),
    ];
    for (o, v) in dir {
        wr::<u32>(d, o, v);
    }
    let l = p8(LOCAL_TEMPLATE);
    for (o, v) in [
        (0x4isize, 0u32),
        (0x8, 0),
        (0xC, 0),
        (0x10, ONE_BITS),
        (0x14, 0),
        (0x18, 0),
        (0x1C, 0),
        (0x20, ONE_BITS),
        (0x24, 0),
        (0x28, 0),
        (0x2C, 0),
        (0x30, ONE_BITS),
        (0x50, ONE_BITS),
    ] {
        wr::<u32>(l, o, v);
    }
    1
}

/// `_rwD3D9LightsClose()` (0x755FE0): the three index arrays freed and every counter cleared. EAX is the
/// last array pointer read (void here: the free path leaves it undefined).
#[hook(addr = 0x755FE0, conv = cdecl, name = "_rwD3D9LightsClose", expect = "A1 C4 26 C9 00")]
pub fn rw_d3d9_lights_close() {
    free_lists();
}

/// `_rwD3D9LightsGlobalEnable(flags)` (0x756070): AmbientSaturated = (0, 0, 0, 1); for each directional-list
/// light of the current world (`world+0x3C`) with `flags & light.flags`: DIRECTIONAL -> the directional
/// enable body; any other subType -> its rgb added to AmbientSaturated (x87). Then `RwD3D9SetRenderState(
/// D3DRS_AMBIENT, lit ? 0xFFFFFFFF : 0)`. Returns lit (0/1).
#[hook(addr = 0x756070, conv = cdecl, name = "_rwD3D9LightsGlobalEnable", expect = "A1 24 7B C9 00")]
pub fn rw_d3d9_lights_global_enable(flags: u32) -> u32 {
    let a = p8(AMBIENT_SAT);
    wr::<u32>(a, 0xC, ONE_BITS);
    wr::<u32>(a, 0, 0);
    wr::<u32>(a, 4, 0);
    wr::<u32>(a, 8, 0);
    let world = p8(rd::<u32>(rw_globals(), 4));
    let head = world.byte_add(0x3C);
    let mut link = p8(rd::<u32>(world, 0x3C));
    let mut lit = 0u32;
    if link == head {
        rw_d3d9_set_render_state(0x8B, 0);
        return lit;
    }
    while link != head {
        let l = link.byte_add(-0x34);
        if !l.is_null() && (flags as u8) & rd::<u8>(l, 2) != 0 {
            if rd::<u8>(l, 1) == 1 {
                directional(l);
            } else {
                for o in [0isize, 4, 8] {
                    let v = x(rd::<f32>(a, o)).fadd_m(rd::<f32>(l, 0x18 + o));
                    wr::<f32>(a, o, f32_of(v));
                }
            }
            lit = 1;
        }
        link = p8(rd::<u32>(link, 0));
    }
    rw_d3d9_set_render_state(0x8B, if lit != 0 { 0xFFFF_FFFF } else { 0 });
    lit
}

/// `_rwD3D9LightDirectionalEnable(light)` (0x756260): index, template, `RwD3D9SetLight`, current list.
/// Returns 1.
#[hook(addr = 0x756260, conv = cdecl, name = "_rwD3D9LightDirectionalEnable", expect = "8B 44 24 04 8B 0D B0 26 C9 00")]
pub fn rw_d3d9_light_directional_enable(l: GPtr<u8>) -> u32 {
    directional(l);
    1
}

/// `_rwD3D9LightLocalEnable(light)` (0x756390): index; local template diffuse rgb, position (LTM pos),
/// range = radius; radius == 0 (x87 compare, NaN continues) returns 0 with the index kept. Attenuation
/// (att0, att1 / r, att2 / (r * r)); POINT: type 1, direction / theta / phi 0; SPOT: type 2, direction =
/// LTM at, phi = 2 * cone, theta = phi if phi < pi else phi - 0.001; SPOTSOFT: type 2, theta 0, phi = 2 *
/// cone; other subTypes return 0. `RwD3D9SetLight`, append to the current list, 1.
#[hook(addr = 0x756390, conv = cdecl, name = "_rwD3D9LightLocalEnable", expect = "A1 B0 26 C9 00")]
pub fn rw_d3d9_light_local_enable(l: GPtr<u8>) -> u32 {
    let e = ext(l);
    alloc_index(e);
    let t = p8(LOCAL_TEMPLATE);
    wr::<u32>(t, 0x4, rd::<u32>(l, 0x18));
    wr::<u32>(t, 0x8, rd::<u32>(l, 0x1C));
    wr::<u32>(t, 0xC, rd::<u32>(l, 0x20));
    let ltm = rw_frame_get_ltm(p8(rd::<u32>(l, 4)));
    wr::<u32>(t, 0x34, rd::<u32>(ltm, 0x30));
    wr::<u32>(t, 0x38, rd::<u32>(ltm, 0x34));
    wr::<u32>(t, 0x3C, rd::<u32>(ltm, 0x38));
    let r = x(rd::<f32>(l, 0x14));
    let equal = !r.fcom(x(g::<f32>(ZERO_ADDR).read())).test_parity(0x44);
    wr::<f32>(t, 0x4C, f32_of(r));
    if equal {
        return 0;
    }
    wr::<u32>(t, 0x54, rd::<u32>(e, 4));
    wr::<f32>(t, 0x58, f32_of(x(rd::<f32>(e, 8)).fdiv(r)));
    wr::<f32>(t, 0x5C, f32_of(r.fmul(r).fdivr_m(rd::<f32>(e, 0xC))));
    match rd::<u8>(l, 1) {
        0x80 => {
            wr::<u32>(t, 0x0, 1);
            wr::<u32>(t, 0x40, 0);
            wr::<u32>(t, 0x44, 0);
            wr::<u32>(t, 0x48, 0);
            wr::<u32>(t, 0x60, 0);
            wr::<u32>(t, 0x64, 0);
        }
        0x81 => {
            wr::<u32>(t, 0x0, 2);
            wr::<u32>(t, 0x40, rd::<u32>(ltm, 0x20));
            wr::<u32>(t, 0x44, rd::<u32>(ltm, 0x24));
            wr::<u32>(t, 0x48, rd::<u32>(ltm, 0x28));
            let a = X87::from_f64_exact(rp_light_get_cone_angle(l)).fdouble();
            let below = !a.fcom(x(g::<f32>(PI_ADDR).read())).test_parity(0x05);
            if below {
                wr::<f32>(t, 0x60, f32_of(a));
            } else {
                wr::<f32>(t, 0x60, f32_of(x(g::<f32>(MILLI_ADDR).read()).fsubr(a)));
            }
            wr::<f32>(t, 0x64, f32_of(a));
        }
        0x82 => {
            wr::<u32>(t, 0x0, 2);
            wr::<u32>(t, 0x40, rd::<u32>(ltm, 0x20));
            wr::<u32>(t, 0x44, rd::<u32>(ltm, 0x24));
            wr::<u32>(t, 0x48, rd::<u32>(ltm, 0x28));
            wr::<u32>(t, 0x60, 0);
            let a = X87::from_f64_exact(rp_light_get_cone_angle(l)).fdouble();
            wr::<f32>(t, 0x64, f32_of(a));
        }
        _ => return 0,
    }
    rw_d3d9_set_light(rd::<i32>(e, 0), t);
    append_current(rd::<u32>(e, 0));
    1
}

/// `_rwD3D9LightsEnable(enable, unused)` (0x756600): enable == 0 -> `RwD3D9SetRenderState(D3DRS_LIGHTING,
/// 0)` only. Else the current count is clamped to the max active lights; previous lights missing from the
/// current list are disabled; the current ones enabled (count re-read after each call); the lists are
/// swapped (prevCount = count) or prevCount = 0; count = 0; `RwD3D9SetRenderState(D3DRS_LIGHTING, 1)`.
#[hook(addr = 0x756600, conv = cdecl, name = "_rwD3D9LightsEnable", expect = "8B 44 24 04 85 C0")]
pub fn rw_d3d9_lights_enable(enable: u32, _unused: u32) -> i32 {
    if enable == 0 {
        return rw_d3d9_set_render_state(0x89, 0);
    }
    let mut count = g::<u32>(CUR_COUNT).read();
    let max = g::<u32>(MAX_ACTIVE).read();
    if count > max {
        count = max;
        g::<u32>(CUR_COUNT).write(count);
    }
    if g::<u32>(PREV_COUNT).read() != 0 {
        let mut i = 0u32;
        loop {
            let idx = rd::<u32>(p8(g::<u32>(PREV_ARRAY).read()), i as isize * 4);
            let mut k = 0u32;
            let cur = p8(g::<u32>(CUR_ARRAY).read());
            while k < count && rd::<u32>(cur, k as isize * 4) != idx {
                k += 1;
            }
            if k >= count {
                rw_d3d9_enable_light(idx as i32, 0);
                count = g::<u32>(CUR_COUNT).read();
            }
            i += 1;
            if i >= g::<u32>(PREV_COUNT).read() {
                break;
            }
        }
    }
    if count != 0 {
        let mut i = 0u32;
        while i < count {
            let idx = rd::<u32>(p8(g::<u32>(CUR_ARRAY).read()), i as isize * 4);
            rw_d3d9_enable_light(idx as i32, 1);
            count = g::<u32>(CUR_COUNT).read();
            i += 1;
        }
        let prev = g::<u32>(PREV_ARRAY).read();
        g::<u32>(PREV_ARRAY).write(g::<u32>(CUR_ARRAY).read());
        let cur_cap = g::<u32>(CUR_CAP).read();
        g::<u32>(CUR_ARRAY).write(prev);
        let prev_cap = g::<u32>(PREV_CAP).read();
        g::<u32>(PREV_CAP).write(cur_cap);
        g::<u32>(CUR_CAP).write(prev_cap);
        g::<u32>(PREV_COUNT).write(count);
    } else {
        g::<u32>(PREV_COUNT).write(0);
    }
    g::<u32>(CUR_COUNT).write(0);
    rw_d3d9_set_render_state(0x89, 1)
}

/// Every `#[hook]` of this module.
/// `cb_LIGHTMODULE_RpLight_Ctor(light, offset, size)` (0x755BC0): with a light, its extension
/// gets index -1, attenuation (1.0, 0.0, 5.0); returns the light (NULL for NULL). Written by STD-3,
/// unverified.
#[hook(addr = 0x755BC0, conv = cdecl, name = "cb_LIGHTMODULE_RpLight_Ctor", expect = "8B 44 24 04 85 C0")]
pub fn light_ext_ctor(light: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    if light.is_null() {
        return GPtr::NULL;
    }
    let e = light.byte_add(g::<i32>(LIGHT_EXT_OFFSET).read() as isize);
    wr::<i32>(e, 0, -1);
    wr::<u32>(e, 4, 0x3F80_0000);
    wr::<u32>(e, 8, 0);
    wr::<u32>(e, 0xC, 0x40A0_0000);
    light
}

/// `cb_LIGHTMODULE_RpLight_Dtor(light, offset, size)` (0x755BF0): a light holding a D3D light
/// index (>= 0) gives it back: when the free list already holds all but one of the handed-out
/// indices the whole bookkeeping is reset (list freed); otherwise the index is appended (the list
/// grows by 0x100 entries, hint 0x103050D). Returns the light. Written by STD-3, unverified.
#[hook(addr = 0x755BF0, conv = cdecl, name = "cb_LIGHTMODULE_RpLight_Dtor", expect = "53 56 8B 74 24 0C")]
pub fn light_ext_dtor(light: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    if light.is_null() {
        return light;
    }
    let e = light.byte_add(g::<i32>(LIGHT_EXT_OFFSET).read() as isize);
    let idx = rd::<i32>(e, 0);
    if idx < 0 {
        return light;
    }
    let used = g::<i32>(NEXT_INDEX).read();
    let free = g::<i32>(FREE_COUNT).read();
    if free >= used.wrapping_sub(1) {
        g::<i32>(FREE_COUNT).write(0);
        g::<i32>(FREE_CAP).write(0);
        g::<i32>(NEXT_INDEX).write(0);
        let l = g::<u32>(FREE_ARRAY).read();
        if l != 0 {
            rw_free(p8(l));
            g::<u32>(FREE_ARRAY).write(0);
        }
        return light;
    }
    if free >= g::<i32>(FREE_CAP).read() {
        let cap = g::<i32>(FREE_CAP).read().wrapping_add(0x100);
        g::<i32>(FREE_CAP).write(cap);
        let l = g::<u32>(FREE_ARRAY).read();
        let n = if l == 0 {
            rw_malloc((cap as u32).wrapping_mul(4), 0x103_050D)
        } else {
            rw_realloc(p8(l), (cap as u32) << 2, 0x103_050D)
        };
        g::<u32>(FREE_ARRAY).write(n.addr() as u32);
    }
    let l = p8(g::<u32>(FREE_ARRAY).read());
    let f = g::<i32>(FREE_COUNT).read();
    wr::<i32>(l, 4 * f as isize, idx);
    g::<i32>(FREE_COUNT).write(f.wrapping_add(1));
    light
}

/// `cb_LIGHTMODULE_RpLight_Copy(dst, src, offset, size)` (0x755CE0): with both lights, the 16
/// extension bytes are copied; the result register is `dst` (EAX keeps the first argument).
/// Written by STD-3, unverified.
#[hook(addr = 0x755CE0, conv = cdecl, name = "cb_LIGHTMODULE_RpLight_Copy", expect = "8B 54 24 08 8B 44 24 04")]
pub fn light_ext_copy(dst: GPtr<u8>, src: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    if !src.is_null() && !dst.is_null() {
        let off = g::<i32>(LIGHT_EXT_OFFSET).read() as isize;
        for i in 0..4 {
            wr::<u32>(dst.byte_add(off), 4 * i, rd::<u32>(src.byte_add(off), 4 * i));
        }
    }
    dst
}

pub const HOOKS: &[HookReg] = &[
    HOOK_LIGHT_EXT_CTOR,
    HOOK_LIGHT_EXT_DTOR,
    HOOK_LIGHT_EXT_COPY,
    HOOK_RP_D3D9_LIGHT_REGISTER_PLUGIN,
    HOOK_RP_D3D9_LIGHT_SET_ATTENUATION_PARAMS,
    HOOK_RP_D3D9_LIGHT_GET_ATTENUATION_PARAMS,
    HOOK_RW_D3D9_LIGHTS_OPEN,
    HOOK_RW_D3D9_LIGHTS_CLOSE,
    HOOK_RW_D3D9_LIGHTS_GLOBAL_ENABLE,
    HOOK_RW_D3D9_LIGHT_DIRECTIONAL_ENABLE,
    HOOK_RW_D3D9_LIGHT_LOCAL_ENABLE,
    HOOK_RW_D3D9_LIGHTS_ENABLE,
];

/// Globals owned by this module (the light tables are shared cells, accessed raw).
pub const GLOBALS: &[GlobalDesc] = &[];
