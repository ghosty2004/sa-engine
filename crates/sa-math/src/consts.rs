//! f32 constants as stored in `gta_sa.exe` 1.0 US `.rdata`, by bit pattern. [`EXE_TABLE`] lists where
//! each one is read; `sa-oracle` checks every entry against the exe.

pub const F0: u32 = 0x0000_0000;
pub const F0_0001: u32 = 0x38D1_B717;
pub const F0_5: u32 = 0x3F00_0000;
pub const F1: u32 = 0x3F80_0000;
pub const MINUS_ONE: u32 = 0xBF80_0000;
/// Not read by the original (used only by a mutant).
pub const F2: u32 = 0x4000_0000;
pub const F4: u32 = 0x4080_0000;
pub const F25: u32 = 0x41C8_0000;
pub const FM25: u32 = 0xC1C8_0000;
pub const F180: u32 = 0x4334_0000;
pub const FM180: u32 = 0xC334_0000;
pub const F360: u32 = 0x43B4_0000;
pub const PI: u32 = 0x4049_0FDB;
pub const MINUS_PI: u32 = 0xC049_0FDB;
pub const TWO_PI: u32 = 0x40C9_0FDB;
pub const HALF_PI: u32 = 0x3FC9_0FDB;
pub const MINUS_HALF_PI: u32 = 0xBFC9_0FDB;
pub const THREE_HALF_PI: u32 = 0x4096_CBE4;

pub const RAND_SCALE_INT: u32 = 0x3800_0000; // 1/32768
pub const RAND_SCALE_FLOAT: u32 = 0x3800_0100; // 1/32767
pub const SIN_TABLE_SCALE: u32 = 0x4222_F983; // 256/(2pi)
pub const F64: u32 = 0x4280_0000;
pub const EULER_EPSILON: u32 = 0x3600_0000; // 2^-19
pub const SIN_TABLE_STEP: u32 = 0x3CC9_0FDB; // 2pi/256
pub const DECOMPRESS_SCALE: u32 = 0x3C01_0204; // 1/127
pub const F127: u32 = 0x42FE_0000;
pub const PI_OVER_8: u32 = 0x3EC9_0FDB;
pub const INV_TWO_PI: u32 = 0x3E22_F983;
pub const F8: u32 = 0x4100_0000;
pub const RAD_TO_DEG: u32 = 0x4265_2EE0;

/// `CMatrix` Euler-angle axis tables (bytes at 0x866D9C and 0x866D94).
pub const EULER_T1: [u8; 4] = [0, 1, 2, 0];
pub const EULER_T2: [u8; 8] = [1, 2, 0, 1, 0, 0, 0, 0];

/// (name, address in the exe, bytes) for the byte tables above.
pub const EXE_BYTES: &[(&str, u32, &[u8])] = &[
    ("EULER_T1", 0x866D9C, &EULER_T1),
    ("EULER_T2", 0x866D94, &EULER_T2),
];

/// (name, address in the exe, bits) for every constant above that the original reads.
pub const EXE_TABLE: &[(&str, u32, u32)] = &[
    ("F0", 0x858B50, F0),
    ("F0_0001", 0x858FC4, F0_0001),
    ("F0_5", 0x858B8C, F0_5),
    ("F1", 0x858624, F1),
    ("MINUS_ONE", 0x858C1C, MINUS_ONE),
    ("F4", 0x858B90, F4),
    ("F25", 0x858FE8, F25),
    ("FM25", 0x863AC0, FM25),
    ("F180", 0x85A994, F180),
    ("FM180", 0x863ABC, FM180),
    ("F360", 0x859E2C, F360),
    ("PI", 0x858CB8, PI),
    ("MINUS_PI", 0x858CC0, MINUS_PI),
    ("TWO_PI", 0x858CBC, TWO_PI),
    ("HALF_PI", 0x858FE4, HALF_PI),
    ("MINUS_HALF_PI", 0x859998, MINUS_HALF_PI),
    ("THREE_HALF_PI", 0x863AC4, THREE_HALF_PI),
    ("RAND_SCALE_INT", 0x858B14, RAND_SCALE_INT),
    ("RAND_SCALE_FLOAT", 0x858C7C, RAND_SCALE_FLOAT),
    ("SIN_TABLE_SCALE", 0x85A778, SIN_TABLE_SCALE),
    ("F64", 0x859A44, F64),
    ("EULER_EPSILON", 0x866D90, EULER_EPSILON),
    ("SIN_TABLE_STEP", 0x859BBC, SIN_TABLE_STEP),
    ("DECOMPRESS_SCALE", 0x859BCC, DECOMPRESS_SCALE),
    ("F127", 0x86532C, F127),
    ("PI_OVER_8", 0x859F50, PI_OVER_8),
    ("INV_TWO_PI", 0x8594F0, INV_TWO_PI),
    ("F8", 0x859000, F8),
    ("RAD_TO_DEG", 0x859878, RAD_TO_DEG),
];
