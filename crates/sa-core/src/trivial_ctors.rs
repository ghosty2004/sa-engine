//! STD-8: the constructors the C++ static initialisers reach that were not ported, as phase B natives (only the
//! initialisers call them; the three-byte `mov eax, ecx; ret` entries cannot be hooked). Ported from the 1.0 US
//! bytes.

use crate::mem::GPtr;
use crate::orig;

orig! {
    /// `CScriptsForBrains::Init()` (0x46A8C0)
    fn scripts_for_brains_init(this: GPtr<u8>) = thiscall 0x46A8C0 => unported;
}

/// `mov eax, ecx; ret`: an empty constructor returning `this`.
fn identity(this: GPtr<u8>) -> GPtr<u8> {
    this
}

/// `CIntroTextLine` element constructor (0x463750, `mov eax, ecx; ret`), for `CTheScripts::IntroTextLines`.
#[crate::native(addr = 0x463750, conv = thiscall, name = "FUN_00463750")]
pub fn intro_text_line_ctor(this: GPtr<u8>) -> GPtr<u8> {
    identity(this)
}

/// `CIntroRectangle` element constructor (0x463770, `mov eax, ecx; ret`), for `CTheScripts::IntroRectangles`.
#[crate::native(addr = 0x463770, conv = thiscall, name = "FUN_00463770")]
pub fn intro_rectangle_ctor(this: GPtr<u8>) -> GPtr<u8> {
    identity(this)
}

/// `CPlantLocTri` element constructor (0x5DB2B0, `mov eax, ecx; ret`), for `CPlantMgr::m_LocTrisTab`.
#[crate::native(addr = 0x5DB2B0, conv = thiscall, name = "FUN_005db2b0")]
pub fn plant_loc_tri_ctor(this: GPtr<u8>) -> GPtr<u8> {
    identity(this)
}

/// `CPlantColEntEntry` element constructor (0x5DB290, `mov eax, ecx; ret`), for `CPlantMgr::m_ColEntCacheTab`.
#[crate::native(addr = 0x5DB290, conv = thiscall, name = "FUN_005db290")]
pub fn plant_col_ent_entry_ctor(this: GPtr<u8>) -> GPtr<u8> {
    identity(this)
}

/// `CScriptsForBrains::CScriptsForBrains()` (0x46B260): `Init()`, returns `this`.
#[crate::native(addr = 0x46B260, conv = thiscall, name = "CScriptsForBrains::CScriptsForBrains")]
pub fn scripts_for_brains_ctor(this: GPtr<u8>) -> GPtr<u8> {
    scripts_for_brains_init(this);
    this
}

/// `COctTreeBase::COctTreeBase()` (0x5A7570): +4, +0x1C, +0x20, +0x24 = 0; the four dwords at +0xA, +0xE, +0x12,
/// +0x16 (unaligned) = -1; byte +8 = 0; vtable 0x867178. Returns `this`.
#[crate::native(addr = 0x5A7570, conv = thiscall, name = "COctTreeBase::COctTreeBase")]
pub fn oct_tree_base_ctor(this: GPtr<u8>) -> GPtr<u8> {
    // the -1 dwords sit at unaligned offsets: written byte by byte (little-endian)
    let w = |o: isize, v: u32| {
        for (i, b) in v.to_le_bytes().into_iter().enumerate() {
            this.byte_add(o + i as isize).write(b);
        }
    };
    for o in [4, 0x1C, 0x20, 0x24] {
        w(o, 0);
    }
    for o in [0xA, 0xE, 0x12, 0x16] {
        w(o, u32::MAX);
    }
    this.byte_add(8).write(0);
    w(0, 0x867178);
    this
}

/// The constructor of `g_Heaps` / 0xC87B88 (0x72E860): `*this = 0`, returns `this`.
#[crate::native(addr = 0x72E860, conv = thiscall, name = "FUN_0072e860")]
pub fn heap_list_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this.cast::<u32>().write(0);
    this
}

/// This module's natives.
pub const NATIVES: &[crate::abi::NativeReg] = &[
    NATIVE_INTRO_TEXT_LINE_CTOR,
    NATIVE_INTRO_RECTANGLE_CTOR,
    NATIVE_PLANT_LOC_TRI_CTOR,
    NATIVE_PLANT_COL_ENT_ENTRY_CTOR,
    NATIVE_SCRIPTS_FOR_BRAINS_CTOR,
    NATIVE_OCT_TREE_BASE_CTOR,
    NATIVE_HEAP_LIST_CTOR,
];
