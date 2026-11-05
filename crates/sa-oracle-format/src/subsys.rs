//! Per-subsystem frame-state hashes (INF-6).
//!
//! The hasher reads the game's state through [`Mem`] in the original memory layout of gta_sa.exe 1.0 US,
//! so the same code hashes the original process (sa-hook, phase A) and can later hash an engine that
//! keeps the original layout. Every address and offset below was read from the 1.0 US binary
//! (disassembly noted next to it) or from gta-reversed and then checked against the binary.
//!
//! A frame's hashes are taken at the frame boundary (entry of `CTimer::Update`, 0x561B10), i.e. they
//! describe the state after the previous frame finished.
//!
//! Pointers into the entity pools are normalised to (pool, slot) so that a hash does not depend on
//! where the heap placed the pool storage. Other pointers are hashed raw; [`Subsystem::HeapAddr`] is
//! a diagnostic that hashes nothing but heap addresses, to measure heap placement determinism.

use crate::fnv::Fnv64;

/// Read access to the game's address space. Returns false when the range cannot be read.
pub trait Mem {
    fn read(&self, addr: u32, out: &mut [u8]) -> bool;

    fn u8(&self, addr: u32) -> Option<u8> {
        let mut b = [0u8; 1];
        self.read(addr, &mut b).then_some(b[0])
    }
    fn u16(&self, addr: u32) -> Option<u16> {
        let mut b = [0u8; 2];
        self.read(addr, &mut b).then(|| u16::from_le_bytes(b))
    }
    fn u32(&self, addr: u32) -> Option<u32> {
        let mut b = [0u8; 4];
        self.read(addr, &mut b).then(|| u32::from_le_bytes(b))
    }
}

macro_rules! subsystems {
    ($($v:ident = $i:expr, $name:expr, $doc:expr;)*) => {
        /// Frame-state hash subsystems. The numeric ids and names are part of the trace format.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Subsystem { $(#[doc = $doc] $v = $i,)* }
        impl Subsystem {
            pub const ALL: &'static [Subsystem] = &[$(Subsystem::$v,)*];
            pub fn name(self) -> &'static str { match self { $(Subsystem::$v => $name,)* } }
            pub fn from_name(s: &str) -> Option<Subsystem> {
                match s { $($name => Some(Subsystem::$v),)* _ => None }
            }
        }
    };
}

subsystems! {
    Timer = 0, "timer", "CTimer globals 0xB7CB2C..0xB7CB8C (time, step, frame counter, render start time).";
    Rand = 1, "rand", "Main thread CRT rand state (`_getptd()->_holdrand`, ptd+0x14, rand at 0x821B1E).";
    RandCalls = 2, "randcalls", "Sequence hash of the main thread's rand/srand calls (caller, value) since the previous boundary.";
    Peds = 3, "peds", "Ped pool: occupancy byte map + per ped model, placement, matrix, move/turn speed.";
    Vehicles = 4, "vehicles", "Vehicle pool: as peds.";
    Objects = 5, "objects", "Object pool: as peds.";
    Buildings = 6, "buildings", "Building pool: occupancy + model, placement, matrix.";
    Dummies = 7, "dummies", "Dummy pool: as buildings.";
    Pools = 8, "pools", "Occupancy byte maps of the other 12 pools (ptr nodes, entry info, col models, tasks, events, routes, intelligence, attractors).";
    World = 9, "world", "CWorld sector and repeat-sector lists, moving-entity list: per list count + members (normalised).";
    Streaming = 10, "streaming", "CStreaming::ms_aInfoForModel (load state, flags, request links) + memory/request counters.";
    Scripts = 11, "scripts", "ScriptSpace (globals + code), mission locals, active CRunningScript structs (IP, locals, timers, stack).";
    Camera = 12, "camera", "TheCamera: m_mCameraMatrix, game cam position, placeable matrix.";
    ClockWeather = 13, "clockweather", "CClock globals and CWeather globals.";
    Player = 14, "player", "CWorld::Players[0..2] (CPlayerInfo, pool pointers normalised).";
    Pad = 15, "pad", "CPad::Pads[0..2], mouse and keyboard states after input processing.";
    HeapAddr = 16, "heapaddr", "Diagnostic: raw heap addresses (pool storage, byte maps, player/camera matrices).";
}

pub const COUNT: usize = 17;

/// Addresses in gta_sa.exe 1.0 US.
pub mod addr {
    /// CTimer::m_snTimerDivider (Timer.h) .. end of the timer block (m_snTimeInMilliseconds 0xB7CB84 + 8).
    pub const TIMER_BLOCK: (u32, u32) = (0xB7CB2C, 0xB7CB8C);
    pub const FRAME_COUNTER: u32 = 0xB7CB4C;
    pub const TIME_IN_MS: u32 = 0xB7CB84;
    /// CPools::ms_p*Pool pointers 0xB74484..=0xB744C4 (CPools::Initialise 0x550F10 stores them in this
    /// order: ptr single, ptr double, entry info, ped, vehicle, building, object, dummy, col model,
    /// task, event, point route, patrol route, node route, task allocator, ped intelligence, attractor).
    pub const POOLS: u32 = 0xB74484;
    pub const POOL_COUNT: u32 = 17;
    pub const PED_POOL: u32 = 0xB74490;
    pub const VEHICLE_POOL: u32 = 0xB74494;
    pub const BUILDING_POOL: u32 = 0xB74498;
    pub const OBJECT_POOL: u32 = 0xB7449C;
    pub const DUMMY_POOL: u32 = 0xB744A0;
    /// Element strides from the pool constructors (imul in 0x5503F0, 0x5504C0, 0x550570, 0x550640,
    /// 0x5506F0) and CPools::GetPed/GetVehicle/GetObject (0x54FF90, 0x54FFF0, 0x550050).
    pub const PED_STRIDE: u32 = 0x7C4;
    pub const VEHICLE_STRIDE: u32 = 0xA18;
    pub const OBJECT_STRIDE: u32 = 0x19C;
    pub const BUILDING_STRIDE: u32 = 0x38;
    pub const DUMMY_STRIDE: u32 = 0x38;
    /// CPlaceable/CEntity/CPhysical offsets (CPhysical::ApplyMoveSpeed 0x542DD0, CPlaceable::GetHeading
    /// 0x441DB0, CCarCtrl::SlowCarDownForObjectsSectorList 0x42D4F0).
    pub const ENT_PLACEMENT: u32 = 0x04;
    pub const ENT_MATRIX: u32 = 0x14;
    pub const ENT_MODEL: u32 = 0x22;
    pub const PHYS_MOVE_SPEED: u32 = 0x44;
    pub const PHYS_TURN_SPEED: u32 = 0x50;
    /// CMatrix: right +0, forward +0x10, up +0x20, pos +0x30 (12 bytes each, ApplyMoveSpeed writes +0x30).
    pub const MAT_POS: u32 = 0x30;
    /// CWorld::ms_aSectors (120x120 CSector = 2 double-link lists), ms_aRepeatSectors (16x16 x 3 lists),
    /// ms_listMovingEntityPtrs (World.h; the arrays are contiguous: 0xB7D0B8 + 14400*8 = 0xB992B8).
    pub const SECTORS: u32 = 0xB7D0B8;
    pub const SECTOR_LISTS: u32 = 120 * 120 * 2;
    pub const REPEAT_SECTORS: u32 = 0xB992B8;
    pub const REPEAT_SECTOR_LISTS: u32 = 16 * 16 * 3;
    pub const MOVING_ENTITIES: u32 = 0xB9ACC8;
    /// CStreaming (Streaming.h): ms_memoryUsedBytes, ms_numModelsRequested, ms_aInfoForModel[26316] x 0x14.
    pub const STREAMING_COUNTERS: u32 = 0x8E4CB4;
    pub const STREAMING_INFO: u32 = 0x8E4CC0;
    pub const STREAMING_INFO_LEN: u32 = 26316 * 0x14;
    /// CTheScripts (TheScripts.h): ScriptSpace (200000 main + 69000 mission), mission locals (1024 x 4),
    /// pActiveScripts list head; CRunningScript is 0xE0 bytes, m_pNext at +0.
    pub const SCRIPT_SPACE: u32 = 0xA49960;
    pub const SCRIPT_SPACE_LEN: u32 = 269_000;
    pub const MISSION_LOCALS: u32 = 0xA48960;
    pub const MISSION_LOCALS_LEN: u32 = 1024 * 4;
    pub const ACTIVE_SCRIPTS: u32 = 0xA8B42C;
    pub const RUNNING_SCRIPT_SIZE: u32 = 0xE0;
    /// TheCamera (Camera.cpp); m_mCameraMatrix at +0x974 and m_vecGameCamPos at +0x908
    /// (CCamera::CopyCameraMatrixToRWCam 0x50AFA0 reads +0x974..+0x9AC; GetGameCamPosition 0x50AE50).
    pub const CAMERA: u32 = 0xB6F028;
    pub const CAMERA_MATRIX: u32 = 0x974;
    pub const CAMERA_GAME_POS: u32 = 0x908;
    /// CClock block (Clock.h 0xB70144..0xB70160), CWeather scalars (Weather.h 0xC812A8..0xC81344),
    /// CWeather::WindDir (0xC813E0).
    pub const CLOCK_BLOCK: (u32, u32) = (0xB70144, 0xB70160);
    pub const WEATHER_BLOCK: (u32, u32) = (0xC812A8, 0xC81344);
    pub const WIND_DIR: u32 = 0xC813E0;
    /// CWorld::Players[2], sizeof(CPlayerInfo) = 0x190, m_pPed at +0.
    pub const PLAYERS: u32 = 0xB7CD98;
    pub const PLAYER_INFO_SIZE: u32 = 0x190;
    /// CPad::Pads[2] (0x134 each), New/OldMouseControllerState (0xB73418, 0xB7342C, 0x14 each),
    /// OldKeyState/NewKeyState (0xB72F20, 0xB73190, 0x270 each).
    pub const PADS: u32 = 0xB73458;
    pub const PAD_SIZE: u32 = 0x134;
    pub const MOUSE_STATES: (u32, u32) = (0xB73418, 0xB73440);
    pub const KEY_STATES: (u32, u32) = (0xB72F20, 0xB73400);
}

/// Values the hasher cannot read from memory (taken by sa-hook on the game thread).
#[derive(Debug, Clone, Copy, Default)]
pub struct Extra {
    /// Main thread `_holdrand`.
    pub holdrand: u32,
    /// Running hash of the main thread's rand/srand calls since the previous boundary.
    pub rand_calls: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PoolHdr {
    pub objects: u32,
    pub bytemap: u32,
    pub size: u32,
}

/// Pool headers read once per frame; used to normalise entity pointers.
#[derive(Debug, Clone, Default)]
pub struct PoolMap {
    /// Indexed like `addr::POOLS` (17 pools); entity strides are known for 3..=7.
    pub pools: Vec<PoolHdr>,
}

const ENTITY_POOLS: [(usize, u32); 5] = [
    (3, addr::PED_STRIDE),
    (4, addr::VEHICLE_STRIDE),
    (5, addr::BUILDING_STRIDE),
    (6, addr::OBJECT_STRIDE),
    (7, addr::DUMMY_STRIDE),
];

impl PoolMap {
    pub fn read(m: &dyn Mem) -> PoolMap {
        let mut pools = Vec::with_capacity(addr::POOL_COUNT as usize);
        for i in 0..addr::POOL_COUNT {
            let p = m.u32(addr::POOLS + i * 4).unwrap_or(0);
            let hdr = if p == 0 {
                PoolHdr::default()
            } else {
                PoolHdr {
                    objects: m.u32(p).unwrap_or(0),
                    bytemap: m.u32(p + 4).unwrap_or(0),
                    size: m.u32(p + 8).unwrap_or(0),
                }
            };
            pools.push(hdr);
        }
        PoolMap { pools }
    }

    /// Normalises a word: a pointer into an entity pool's storage becomes
    /// `(0xE000_0000 | pool << 24, slot << 16 | offset_in_object)`; any other word is `(word, 0)`.
    pub fn normalise(&self, v: u32) -> (u32, u32) {
        for &(pi, stride) in &ENTITY_POOLS {
            let Some(h) = self.pools.get(pi) else { continue };
            if h.objects == 0 {
                continue;
            }
            let end = h.objects as u64 + h.size as u64 * stride as u64;
            if (h.objects as u64..end).contains(&(v as u64)) {
                let rel = v - h.objects;
                return (
                    0xE000_0000 | (pi as u32) << 24,
                    (rel / stride) << 16 | (rel % stride),
                );
            }
        }
        (v, 0)
    }
}

/// Hashes a raw block word by word with pool pointers normalised (block length must be a multiple of 4;
/// a trailing remainder is hashed raw).
fn hash_normalised(h: &mut Fnv64, pm: &PoolMap, bytes: &[u8]) {
    let (words, rest) = bytes.as_chunks::<4>();
    for c in words {
        let (a, b) = pm.normalise(u32::from_le_bytes(*c));
        h.u32(a);
        h.u32(b);
    }
    h.write(rest);
}

fn block(m: &dyn Mem, h: &mut Fnv64, start: u32, end: u32) {
    let mut buf = vec![0u8; (end - start) as usize];
    if m.read(start, &mut buf) {
        h.write(&buf);
    } else {
        h.u32(0xDEAD_0001);
    }
}

fn entity(m: &dyn Mem, h: &mut Fnv64, e: u32, physical: bool) {
    let mut b = [0u8; 16];
    h.u16(m.u16(e + addr::ENT_MODEL).unwrap_or(0xFFFF));
    if m.read(e + addr::ENT_PLACEMENT, &mut b) {
        h.write(&b);
    }
    let mat = m.u32(e + addr::ENT_MATRIX).unwrap_or(0);
    if mat == 0 {
        h.u8(0);
    } else {
        h.u8(1);
        for k in 0..4 {
            if m.read(mat + k * 0x10, &mut b[..12]) {
                h.write(&b[..12]);
            }
        }
    }
    if physical {
        if m.read(e + addr::PHYS_MOVE_SPEED, &mut b[..12]) {
            h.write(&b[..12]);
        }
        if m.read(e + addr::PHYS_TURN_SPEED, &mut b[..12]) {
            h.write(&b[..12]);
        }
    }
}

fn entity_pool(m: &dyn Mem, h: &mut Fnv64, pm: &PoolMap, pi: usize, stride: u32, physical: bool) {
    let p = pm.pools[pi];
    h.u32(p.size);
    if p.objects == 0 || p.size == 0 || p.size > 0x10_0000 {
        return;
    }
    let mut map = vec![0u8; p.size as usize];
    if !m.read(p.bytemap, &mut map) {
        h.u32(0xDEAD_0002);
        return;
    }
    h.write(&map);
    for (i, &flag) in map.iter().enumerate() {
        if flag & 0x80 == 0 {
            h.u32(i as u32);
            entity(m, h, p.objects + i as u32 * stride, physical);
        }
    }
}

/// Walks a CPtrList (node: item +0, next +4) and hashes count + normalised members.
fn list(m: &dyn Mem, h: &mut Fnv64, pm: &PoolMap, head_addr: u32, budget: &mut u32) {
    let mut node = m.u32(head_addr).unwrap_or(0);
    let mut n = 0u32;
    while node != 0 && *budget > 0 {
        *budget -= 1;
        let item = m.u32(node).unwrap_or(0);
        let (a, b) = pm.normalise(item);
        h.u32(a);
        h.u32(b);
        node = m.u32(node + 4).unwrap_or(0);
        n += 1;
    }
    h.u32(n);
}

/// Computes every subsystem hash for the current state.
pub fn hash_frame(m: &dyn Mem, extra: &Extra) -> [u64; COUNT] {
    let mut out = [0u64; COUNT];
    let pm = PoolMap::read(m);
    for &s in Subsystem::ALL {
        let mut h = Fnv64::new();
        match s {
            Subsystem::Timer => block(m, &mut h, addr::TIMER_BLOCK.0, addr::TIMER_BLOCK.1),
            Subsystem::Rand => h.u32(extra.holdrand),
            Subsystem::RandCalls => h.u64(extra.rand_calls),
            Subsystem::Peds => entity_pool(m, &mut h, &pm, 3, addr::PED_STRIDE, true),
            Subsystem::Vehicles => entity_pool(m, &mut h, &pm, 4, addr::VEHICLE_STRIDE, true),
            Subsystem::Buildings => entity_pool(m, &mut h, &pm, 5, addr::BUILDING_STRIDE, false),
            Subsystem::Objects => entity_pool(m, &mut h, &pm, 6, addr::OBJECT_STRIDE, true),
            Subsystem::Dummies => entity_pool(m, &mut h, &pm, 7, addr::DUMMY_STRIDE, false),
            Subsystem::Pools => {
                for (i, p) in pm.pools.iter().enumerate() {
                    if (3..=7).contains(&i) {
                        continue;
                    }
                    h.u32(p.size);
                    if p.bytemap != 0 && p.size > 0 && p.size <= 0x10_0000 {
                        block(m, &mut h, p.bytemap, p.bytemap + p.size);
                    }
                }
            }
            Subsystem::World => {
                let mut budget = 2_000_000u32;
                for i in 0..addr::SECTOR_LISTS {
                    list(m, &mut h, &pm, addr::SECTORS + i * 4, &mut budget);
                }
                for i in 0..addr::REPEAT_SECTOR_LISTS {
                    list(m, &mut h, &pm, addr::REPEAT_SECTORS + i * 4, &mut budget);
                }
                list(m, &mut h, &pm, addr::MOVING_ENTITIES, &mut budget);
                h.u32(budget);
            }
            Subsystem::Streaming => {
                block(m, &mut h, addr::STREAMING_COUNTERS, addr::STREAMING_COUNTERS + 8);
                block(
                    m,
                    &mut h,
                    addr::STREAMING_INFO,
                    addr::STREAMING_INFO + addr::STREAMING_INFO_LEN,
                );
            }
            Subsystem::Scripts => {
                block(
                    m,
                    &mut h,
                    addr::SCRIPT_SPACE,
                    addr::SCRIPT_SPACE + addr::SCRIPT_SPACE_LEN,
                );
                block(
                    m,
                    &mut h,
                    addr::MISSION_LOCALS,
                    addr::MISSION_LOCALS + addr::MISSION_LOCALS_LEN,
                );
                let mut s = m.u32(addr::ACTIVE_SCRIPTS).unwrap_or(0);
                let mut n = 0;
                let mut buf = vec![0u8; addr::RUNNING_SCRIPT_SIZE as usize];
                while s != 0 && n < 256 {
                    if !m.read(s, &mut buf) {
                        break;
                    }
                    hash_normalised(&mut h, &pm, &buf);
                    s = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
                    n += 1;
                }
                h.u32(n);
            }
            Subsystem::Camera => {
                let c = addr::CAMERA;
                let mut b = [0u8; 12];
                for k in 0..4 {
                    if m.read(c + addr::CAMERA_MATRIX + k * 0x10, &mut b) {
                        h.write(&b);
                    }
                }
                if m.read(c + addr::CAMERA_GAME_POS, &mut b) {
                    h.write(&b);
                }
                let mat = m.u32(c + addr::ENT_MATRIX).unwrap_or(0);
                if mat != 0 {
                    for k in 0..4 {
                        if m.read(mat + k * 0x10, &mut b) {
                            h.write(&b);
                        }
                    }
                }
            }
            Subsystem::ClockWeather => {
                block(m, &mut h, addr::CLOCK_BLOCK.0, addr::CLOCK_BLOCK.1);
                block(m, &mut h, addr::WEATHER_BLOCK.0, addr::WEATHER_BLOCK.1);
                block(m, &mut h, addr::WIND_DIR, addr::WIND_DIR + 12);
            }
            Subsystem::Player => {
                let mut buf = vec![0u8; 2 * addr::PLAYER_INFO_SIZE as usize];
                if m.read(addr::PLAYERS, &mut buf) {
                    hash_normalised(&mut h, &pm, &buf);
                }
            }
            Subsystem::Pad => {
                block(m, &mut h, addr::PADS, addr::PADS + 2 * addr::PAD_SIZE);
                block(m, &mut h, addr::MOUSE_STATES.0, addr::MOUSE_STATES.1);
                block(m, &mut h, addr::KEY_STATES.0, addr::KEY_STATES.1);
            }
            Subsystem::HeapAddr => {
                for p in &pm.pools {
                    h.u32(p.objects);
                    h.u32(p.bytemap);
                }
                h.u32(m.u32(addr::CAMERA + addr::ENT_MATRIX).unwrap_or(0));
                let ped = m.u32(addr::PLAYERS).unwrap_or(0);
                h.u32(ped);
                if ped != 0 {
                    h.u32(m.u32(ped + addr::ENT_MATRIX).unwrap_or(0));
                }
            }
        }
        out[s as usize] = h.finish();
    }
    out
}

/// The player ped's matrix position x address (for the 1-ulp perturbation test), if the player exists.
pub fn player_pos_x_addr(m: &dyn Mem) -> Option<u32> {
    let ped = m.u32(addr::PLAYERS).filter(|&p| p != 0)?;
    let mat = m.u32(ped + addr::ENT_MATRIX).unwrap_or(0);
    Some(if mat != 0 {
        mat + addr::MAT_POS
    } else {
        ped + addr::ENT_PLACEMENT
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Sparse fake address space: unmapped bytes read as zero inside mapped regions only.
    #[derive(Default)]
    pub struct FakeMem {
        pub regions: BTreeMap<u32, Vec<u8>>,
    }

    impl FakeMem {
        pub fn map(&mut self, start: u32, len: usize) {
            self.regions.insert(start, vec![0; len]);
        }
        pub fn put(&mut self, addr: u32, bytes: &[u8]) {
            let (&s, r) = self
                .regions
                .range_mut(..=addr)
                .next_back()
                .expect("unmapped write");
            let o = (addr - s) as usize;
            r[o..o + bytes.len()].copy_from_slice(bytes);
        }
        pub fn put_u32(&mut self, addr: u32, v: u32) {
            self.put(addr, &v.to_le_bytes());
        }
    }

    impl Mem for FakeMem {
        fn read(&self, addr: u32, out: &mut [u8]) -> bool {
            let Some((&s, r)) = self.regions.range(..=addr).next_back() else {
                return false;
            };
            let o = (addr - s) as usize;
            if o + out.len() > r.len() {
                return false;
            }
            out.copy_from_slice(&r[o..o + out.len()]);
            true
        }
    }

    /// A minimal world: pools at fixed fake heap addresses, one ped with a matrix.
    pub fn fake_world(heap: u32) -> FakeMem {
        let mut m = FakeMem::default();
        m.map(0x8E0000, 0x90000); // streaming
        m.map(0xA40000, 0x60000); // scripts
        m.map(0xB60000, 0x40000); // camera, clock, pad, timer, pools, players, sectors
        m.map(0xC81000, 0x1000); // weather
        // ped pool header at heap, storage at heap+0x100, byte map at heap+0x80 (4 slots)
        m.map(heap, 0x10000);
        m.put_u32(addr::PED_POOL, heap);
        m.put_u32(heap, heap + 0x100);
        m.put_u32(heap + 4, heap + 0x80);
        m.put_u32(heap + 8, 4);
        m.put(heap + 0x80, &[0x01, 0x80, 0x80, 0x80]);
        let ped = heap + 0x100;
        let mat = heap + 0x8000;
        m.put_u32(ped + addr::ENT_MATRIX, mat);
        m.put(mat + addr::MAT_POS, &2488.5f32.to_le_bytes());
        m.put_u32(addr::PLAYERS, ped);
        // ped in a repeat-sector list: node at heap+0x9000
        m.put_u32(addr::REPEAT_SECTORS + 4, heap + 0x9000);
        m.put_u32(heap + 0x9000, ped);
        m
    }

    #[test]
    fn hashes_are_stable_and_pointer_normalised() {
        let a = hash_frame(&fake_world(0x0100_0000), &Extra::default());
        let b = hash_frame(&fake_world(0x0100_0000), &Extra::default());
        assert_eq!(a, b);
        // Same state at a different heap address: everything but the heap diagnostic is equal.
        let c = hash_frame(&fake_world(0x0200_0000), &Extra::default());
        for &s in Subsystem::ALL {
            assert_eq!(
                a[s as usize] == c[s as usize],
                s != Subsystem::HeapAddr,
                "{}",
                s.name()
            );
        }
    }

    #[test]
    fn one_ulp_changes_only_peds() {
        let mut m = fake_world(0x0100_0000);
        let a = hash_frame(&m, &Extra::default());
        let x = player_pos_x_addr(&m).unwrap();
        let v = f32::from_le_bytes(m.regions[&0x0100_0000][0x8030..0x8034].try_into().unwrap());
        m.put(x, &f32::from_bits(v.to_bits() + 1).to_le_bytes());
        let b = hash_frame(&m, &Extra::default());
        let diff: Vec<_> = Subsystem::ALL
            .iter()
            .filter(|&&s| a[s as usize] != b[s as usize])
            .map(|s| s.name())
            .collect();
        assert_eq!(diff, vec!["peds"]);
    }

    #[test]
    fn names_round_trip() {
        assert_eq!(Subsystem::ALL.len(), COUNT);
        for (i, &s) in Subsystem::ALL.iter().enumerate() {
            assert_eq!(s as usize, i);
            assert_eq!(Subsystem::from_name(s.name()), Some(s));
        }
    }
}
