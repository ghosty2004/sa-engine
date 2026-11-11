//! RenderWare 3.6 layouts of `gta_sa.exe` 1.0 US, as read from the bytes (offsets in bytes). The single
//! source of these constants for the phase A port; `sa-rw` (phase B object model) keeps Rust-native
//! types and will take its offsets from here when the two are merged (docs/arch/rw-port.md).

/// `RwMatrix` (0x40): right +0, flags +0xC, up +0x10, at +0x20, pos +0x30.
pub mod matrix {
    pub const SIZE: usize = 0x40;
    pub const RIGHT: isize = 0x0;
    pub const FLAGS: isize = 0xC;
    pub const UP: isize = 0x10;
    pub const AT: isize = 0x20;
    pub const POS: isize = 0x30;
    /// rwMATRIXTYPENORMAL | rwMATRIXTYPEORTHOGONAL
    pub const TYPE_ORTHONORMAL: u32 = 3;
    /// rwMATRIXINTERNALIDENTITY
    pub const INTERNAL_IDENTITY: u32 = 0x20000;
}

/// Engine-plugin module offset globals (`module = G + [global]`).
pub mod modules {
    /// VECTORMODULE (0x18): +0 sqrt table, +4 inverse sqrt table, +8..+0x14 transform functions.
    pub const VECTOR: usize = 0xC97934;
    /// MATRIXMODULE (0x18): +0 freelist, +4 identity mask, +8 multiply fn, +0xC tolerance.
    pub const MATRIX: usize = 0xC979BC;
    /// RESOURCESMODULE (0x28).
    pub const RESOURCES: usize = 0xC9BC58;
}

/// `RwFreeList` (0x24).
pub mod freelist {
    pub const ENTRY_SIZE: isize = 0x0;
    pub const ENTRIES_PER_BLOCK: isize = 0x4;
    pub const HEAP_SIZE: isize = 0x8;
    pub const ALIGNMENT: isize = 0xC;
    pub const BLOCK_LIST: isize = 0x10;
    pub const FLAGS: isize = 0x18;
    pub const LINK: isize = 0x1C;
    pub const SIZE: u32 = 0x24;
}
