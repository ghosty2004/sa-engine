//! `CRunningScript::ProcessCommands1300To1399` (0x48CDD0): complete port (PORT-SCRIPT-2 partial, completed
//! in PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands1300To1399.md`.
//!
//! Divergence notes: the 8-byte label cases (0517, 0518, 054C) read the label into a zeroed 16-byte
//! buffer (the original's is not cleared: an 8-character label is followed by stale stack bytes in the
//! C-string readers). Dead stack writes not ported: the flag bytes in the command-argument slot.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0515, 0x0517, 0x0518, 0x0519, 0x051A, 0x051B, 0x051C, 0x051D, 0x051E, 0x0521, 0x0522, 0x0523, 0x0524,
    0x0525, 0x0526, 0x052C, 0x053E, 0x053F, 0x0540, 0x0541, 0x0545, 0x0547, 0x0548, 0x054A, 0x054B, 0x054C, 0x054D,
    0x054E, 0x054F, 0x0550, 0x0551, 0x0552, 0x0555, 0x0556, 0x0557, 0x055B, 0x055D, 0x055E, 0x055F, 0x0560,
    0x0561, 0x0563, 0x0564, 0x0565, 0x0566, 0x0568, 0x0569, 0x056A, 0x056C, 0x056D, 0x056E, 0x0570, 0x0572,
    0x0574, 0x0575,
];

/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;
/// `.rdata` 0x858B8C: 0.5f
const K_HALF: u32 = 0x3F00_0000;
/// `TheText` (`CText`) 0xC1B340.
const THE_TEXT: usize = 0xC1B340;
/// `AudioEngine` (`CAudioEngine`) 0xB6BC90.
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `ModelIndices` slots read as int16 by the property pickups: 0x8CD5E8 (locked), 0x8CD5EC (for sale).
const MI_PROPERTY_LOCKED: usize = 0x8CD5E8;
const MI_PROPERTY_FORSALE: usize = 0x8CD5EC;
/// `CPopulation::ms_nTotalMissionPeds` 0xC0EC24.
const TOTAL_MISSION_PEDS: usize = 0xC0EC24;
/// `CCutsceneMgr::ms_wasCutsceneSkipped` (byte) 0xB5F854.
const WAS_CUTSCENE_SKIPPED: usize = 0xB5F854;
/// Byte 0x96918B (`CCheat::m_aCheatsActive[0x5B]`, the taxi nitro cheat, cleared by 0572).
const TAXI_NITRO_CHEAT: usize = 0x96918B;
/// The Hunter gun offset `CVector` FIRE_HUNTER_GUN copies (`.data` 0x8D3394).
const HUNTER_GUN_OFFSET: usize = 0x8D3394;
/// `CTimer::ms_fTimeStep` 0xB7CB5C.
const TIME_STEP: usize = 0xB7CB5C;

orig! {
    /// `CMBlur::ClearDrunkBlur()` (0x71D780)
    pub fn clear_drunk_blur() = cdecl 0x71D780 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (exact as f64)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CText::Get(const char* key)` (0x6A0050)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CText::LoadMissionText(const char*)` (0x69FBF0)
    fn load_mission_text(t: GPtr<u8>, name: GPtr<u8>) = thiscall 0x69FBF0 => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0), result discarded
    fn get_actual_pickup_index(h: i32) -> i32 = cdecl 0x4552A0 => unported;
    /// `CPickups::GenerateNewOne(CVector, uint32 model, ePickupType, uint32 ammo, uint32 moneyPerDay,
    /// bool isEmpty, const char* message)` (0x456F20)
    fn generate_new_one(x: u32, y: u32, z: u32, model: u32, ty: u32, ammo: u32, money: u32, empty: u32, msg: GPtr<u8>) -> i32 = cdecl 0x456F20 => unported;
    /// `CPhysical::SkipPhysics()` (0x5433B0)
    fn skip_physics(p: GPtr<u8>) = thiscall 0x5433B0 => unported;
    /// `CPhysical::SetVelocity(CVector)` (0x441130, a `.HOODLUM` stub), the vector as three raw dwords
    fn set_velocity(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x441130 => unported;
    /// Unnamed thiscall taking a vector by value (0x45AFB0, a `.HOODLUM` stub; FREEZE_CAR_POSITION
    /// passes (0, 0, 0) after SetVelocity)
    fn physical_45afb0(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x45AFB0 => unported;
    /// `CAudioEngine::GetCurrentRadioStationID()` (0x507040), AL
    fn get_current_radio_station_id(a: GPtr<u8>) -> u8 = thiscall 0x507040 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CVehicle::CanBeDeleted()` (0x6D1180), AL
    fn can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// `CPlaceable::IsWithinArea(float x1, float y1, float x2, float y2)` (0x54F200), AL
    fn is_within_area(e: GPtr<u8>, x1: u32, y1: u32, x2: u32, y2: u32) -> u8 = thiscall 0x54F200 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160), ECX = the vehicle pool
    fn vehicle_get_ref(pool: GPtr<u8>, v: GPtr<u8>) -> i32 = thiscall 0x424160 => unported;
    /// `CPhysical::GetHasCollidedWith(CEntity*)` (0x543540), AL
    fn get_has_collided_with(p: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x543540 => unported;
    /// `CPed::ClearWeapon(eWeaponType)` (0x5E62B0)
    fn clear_weapon(p: GPtr<u8>, ty: u32) = thiscall 0x5E62B0 => unported;
    /// `CPopulation::AddPedInCar(CVehicle*, bool driver, int32, int32 seat, bool, bool)` (0x613A00)
    fn add_ped_in_car(v: GPtr<u8>, driver: u32, a: i32, seat: u32, b: u32, c: u32) -> GPtr<u8> = cdecl 0x613A00 => unported;
    /// `CPed::SetAmmo(eWeaponType, uint32)` (0x5DF290)
    fn set_ammo(p: GPtr<u8>, ty: u32, ammo: u32) = thiscall 0x5DF290 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080), the slot returned is discarded
    fn give_weapon(p: GPtr<u8>, ty: u32, ammo: u32, flag: u32) -> i32 = thiscall 0x5E6080 => unported;
    /// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280)
    fn set_current_weapon(p: GPtr<u8>, ty: u32) = thiscall 0x5E6280 => unported;
    /// `CExplosion::AddExplosion(CEntity* victim, CPed* creator, eExplosionType, CVector, uint32 lifetime,
    /// uint8 makeSound, float camShake, uint8 visible)` (0x736A50); the vector and float as raw dwords
    fn add_explosion(victim: u32, creator: u32, ty: u32, x: u32, y: u32, z: u32, life: u32, sound: u32, shake: u32, visible: u32) = cdecl 0x736A50 => unported;
    /// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370), AL
    fn is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CCheat::ApplyCheat(eCheats)` (0x438370)
    fn apply_cheat(id: u32) = cdecl 0x438370 => unported;
    /// `CWeapon::CWeapon(eWeaponType, int32 ammo)` (0x73B430)
    fn weapon_ctor(w: GPtr<u8>, ty: u32, ammo: u32) -> GPtr<u8> = thiscall 0x73B430 => unported;
    /// Unnamed `CVector operator*(const CVector&, float)` (0x40FEC0; hidden-pointer result), the float as
    /// its raw dword
    fn vector_mul_float(out: GPtr<u32>, v: GPtr<u8>, k: u32) -> GPtr<u32> = cdecl 0x40FEC0 => unported;
    /// `CMatrix::operator*(const CMatrix&, const CVector&)` (0x59C890; hidden-pointer result)
    fn matrix_mul_vector(out: GPtr<u32>, m: GPtr<u8>, v: GPtr<u32>) -> GPtr<u32> = cdecl 0x59C890 => unported;
    /// Unnamed `CVector operator+(const CVector&, const CVector&)` (0x40FE30; hidden-pointer result)
    fn vector_add(out: GPtr<u32>, a: GPtr<u32>, b: GPtr<u32>) -> GPtr<u32> = cdecl 0x40FE30 => unported;
    /// `CWeapon::FireInstantHit(CEntity* firer, CVector* origin, CVector* muzzle, CEntity* target,
    /// CVector* targetPos, CVector* originForDriveBy, bool, bool)` (0x73FB10), AL (discarded)
    fn fire_instant_hit(w: GPtr<u8>, firer: GPtr<u8>, origin: GPtr<u32>, muzzle: GPtr<u32>, a: u32, b: u32, c: u32, d: u32, e: u32) -> u8 = thiscall 0x73FB10 => unported;
    /// `CWeapon::AddGunshell(CEntity*, const CVector&, const CVector2D&, float size)` (0x73A3E0)
    fn add_gunshell(w: GPtr<u8>, e: GPtr<u8>, pos: GPtr<u32>, dir: GPtr<u32>, size: u32) = thiscall 0x73A3E0 => unported;
    /// `CAudioEngine::ReportWeaponEvent(int32 event, eWeaponType, CPhysical*)` (0x506F40)
    fn report_weapon_event(a: GPtr<u8>, ev: u32, ty: u32, e: GPtr<u8>) = thiscall 0x506F40 => unported;
}

/// 0541 FIRE_HUNTER_GUN (0x48D3AA, gr: unregistered): no null check. Nothing when
/// `CTimer::m_snTimeInMilliseconds <= veh+0x4D4 + 150` (unsigned). Otherwise a stack `CWeapon(0x26,
/// 5000)` (0x1C bytes, no destructor call, as in the original); pos = `veh->m_matrix (+0x14) *
/// [0x8D3394..0x8D339C]` + `veh->m_vecMoveSpeed (+0x44) * CTimer::ms_fTimeStep` (the callees' x87);
/// FireInstantHit(veh, &pos, &pos, 0, 0, 0, 0, 1); AddGunshell(veh, &pos, (0, 0.1), 0.025);
/// AudioEngine.ReportWeaponEvent(0x95, 0x26, veh); veh+0x4D4 = the time.
fn fire_hunter_gun(s: Scr) {
    collect(s, 1);
    let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
    let now = sa_core::timer::TIME_MS.get();
    if now <= veh.byte_add(0x4D4).cast::<u32>().read().wrapping_add(0x96) {
        return;
    }
    let mut weapon = [0u32; 7];
    let w = GPtr::from_raw(weapon.as_mut_ptr()).cast::<u8>();
    weapon_ctor(w, 0x26, 0x1388);
    let off = GPtr::<u32>::from_addr(HUNTER_GUN_OFFSET);
    let ofs = [off.read(), off.wrapping_index(1).read(), off.wrapping_index(2).read()];
    let mat = veh.byte_add(0x14).cast::<GPtr<u8>>().read();
    let step = GPtr::<u32>::from_addr(TIME_STEP).read();
    let mut t1 = [0u32; 3];
    let a = vector_mul_float(GPtr::from_raw(t1.as_mut_ptr()), veh.byte_add(0x44), step);
    let mut t2 = [0u32; 3];
    let b = matrix_mul_vector(GPtr::from_raw(t2.as_mut_ptr()), mat, GPtr::from_raw(ofs.as_ptr().cast_mut()));
    let mut t3 = [0u32; 3];
    let r = vector_add(GPtr::from_raw(t3.as_mut_ptr()), b, a);
    let pos = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
    let pp = GPtr::from_raw(pos.as_ptr().cast_mut());
    fire_instant_hit(w, veh, pp, pp, 0, 0, 0, 0, 1);
    let dir = [0u32, 0x3DCC_CCCD];
    add_gunshell(w, veh, pp, GPtr::from_raw(dir.as_ptr().cast_mut()), 0x3CCC_CCCD);
    report_weapon_event(GPtr::from_addr(AUDIO_ENGINE), 0x95, 0x26, veh);
    veh.byte_add(0x4D4).cast::<u32>().write(sa_core::timer::TIME_MS.get());
}

/// The property pickups' z (as 02E1/04A6): `FindGroundZForCoord(x, y) + 0.5` (x87) when `z <= -100`
/// (ordered), else the `fld; fst` copy of z.
fn pickup_z(x: u32, y: u32, z: u32) -> u32 {
    if X87::from_f32(f(z)).fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        fld_fst(f(z)).to_bits()
    } else {
        X87::from_f64_exact(find_ground_z_for_coord(x, y)).fadd_m(f(K_HALF)).to_f32().to_bits()
    }
}

/// 0517 CREATE_LOCKED_PROPERTY_PICKUP (0x48CDF9) / 0518 CREATE_FORSALE_PROPERTY_PICKUP (0x48CEDB), gr:
/// unregistered: CollectParameters(3 / 4); z rule (`pickup_z`); the 8-byte label (zeroed 16-byte buffer,
/// divergence note: the original's is not cleared) -> `TheText.Get` (result discarded); discarded
/// GetActualPickupIndex of the output operand; P0 = GenerateNewOne((x,y,z), model, type, ammo, 0, 0,
/// label) with (model, type, ammo) = ((u16)[0x8CD5E8], 0x11, 0) / ((u16)[0x8CD5EC], 0x12, P3) (the
/// label buffer itself is the message); StoreParameters(1).
fn property_pickup(s: Scr, forsale: bool) {
    collect(s, if forsale { 4 } else { 3 });
    let (x, y) = (sp(0), sp(1));
    let z = pickup_z(x, y, sp(2));
    let mut buf = [0u8; 16];
    let l = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, l, 8);
    text_get(GPtr::from_addr(THE_TEXT), l);
    get_actual_pickup_index(collect_next_parameter_without_increasing_pc(s));
    let (mi, ty, ammo) = if forsale {
        (MI_PROPERTY_FORSALE, 0x12, sp(3))
    } else {
        (MI_PROPERTY_LOCKED, 0x11, 0)
    };
    let model = u32::from(GPtr::<u16>::from_addr(mi).read());
    let h = generate_new_one(x, y, z, model, ty, ammo, 0, 0, l);
    set_sp(0, h as u32);
    store(s, 1);
}

/// Sets or clears `mask` in the dword at `e + off` (`or` / `and ~mask`).
fn set_bits(e: GPtr<u8>, off: isize, mask: u32, on: bool) {
    let p = e.byte_add(off).cast::<u32>();
    let v = p.read();
    p.write(if on { v | mask } else { v & !mask });
}

/// 0560 CREATE_RANDOM_CHAR_AS_DRIVER (0x48D775) / 0561 CREATE_RANDOM_CHAR_AS_PASSENGER (0x48D820): no
/// null checks; ped = AddPedInCar(veh, driver, -1, seat, 0, 0) (driver: (1, seat 0); passenger: (0,
/// P1)); SetCharCreatedBy(2); the driver only: `m_pIntelligence->m_TaskMgr.SetTask(nullptr, 3, 0)`;
/// bit 31 of +0x46C cleared; mission ped count + 1; P0 = handle; StoreParameters(1); cleanup add (type
/// 2) with mission cleanup.
fn random_char_in_car(s: Scr, driver: bool) {
    collect(s, if driver { 1 } else { 2 });
    let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
    let ped = if driver {
        add_ped_in_car(veh, 1, -1, 0, 0, 0)
    } else {
        add_ped_in_car(veh, 0, -1, sp(1), 0, 0)
    };
    crate::cmd0::set_char_created_by(ped, 2);
    if driver {
        let intel = ped.byte_add(0x47C).cast::<GPtr<u8>>().read();
        crate::cmd0::task_manager_set_task(intel.byte_add(4), GPtr::NULL, 3, 0);
    }
    set_bits(ped, 0x46C, 0x8000_0000, false);
    let n = GPtr::<u32>::from_addr(TOTAL_MISSION_PEDS);
    n.write(n.read().wrapping_add(1));
    let h = ped_get_ref(pool(PED_POOL), ped);
    set_sp(0, h as u32);
    store(s, 1);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), spi(0), 2);
    }
}

/// 0574 FREEZE_CAR_POSITION_AND_DONT_LOAD_COLLISION (0x48DC3F) / 0575
/// FREEZE_CHAR_POSITION_AND_DONT_LOAD_COLLISION (0x48DCB7): no null check. P1 != 0: `+0x40 |= set`
/// (0x2004 / 0x2000); with mission cleanup CWorld::Remove, `+0x1C |= 0x40000`, CWorld::Add. P1 == 0:
/// `+0x40 &= ~set`; the vehicle form also clears byte +0xB8.
fn freeze_no_collision(s: Scr, e: GPtr<u8>, set: u32, vehicle: bool) {
    if sp(1) != 0 {
        set_bits(e, 0x40, set, true);
        if uses_mission_cleanup(s) {
            world_remove(e);
            set_bits(e, 0x1C, 0x40000, true);
            world_add(e);
        }
    } else {
        set_bits(e, 0x40, set, false);
        if vehicle {
            e.byte_add(0xB8).write(0);
        }
    }
}

/// 053E GET_RANDOM_CAR_OF_TYPE_IN_AREA_NO_SAVE (0x48D278, gr: unregistered): CollectParameters(5) (x1,
/// y1, x2, y2, model). Walks the vehicle pool from slot `m_nSize - 1` down to 0 until a handle is found;
/// a slot is taken when in use (`(int8)m_byteMap[i] >= 0`), its address is not 0, GetVehicleAppearance()
/// is 1 or 2 (called twice when not 1), `(int16)m_nModelIndex == P4` or `P4 < 0`, CanBeDeleted(), and
/// IsWithinArea(P0..P3); then handle = VehiclePool->GetRef(it) and byte +0x42A |= 2. P0 = handle or -1;
/// StoreParameters(1). The pool pointer is re-read per slot (as the original's loop head does).
fn random_car_no_save(s: Scr) {
    collect(s, 5);
    let size = pool(VEHICLE_POOL).byte_add(8).cast::<u32>().read();
    let mut h: i32 = -1;
    let mut i = size;
    while i != 0 {
        i = i.wrapping_sub(1);
        if h != -1 {
            break;
        }
        let p = pool(VEHICLE_POOL);
        let flags = p.byte_add(4).cast::<GPtr<u8>>().read();
        if (flags.byte_add(i as isize).read() as i8) < 0 {
            continue;
        }
        let base = p.cast::<u32>().read();
        let v = GPtr::<u8>::from_addr(base.wrapping_add(i.wrapping_mul(0xA18)) as usize);
        if v.is_null() {
            continue;
        }
        if get_vehicle_appearance(v) != 1 && get_vehicle_appearance(v) != 2 {
            continue;
        }
        let m = i32::from(v.byte_add(0x22).cast::<u16>().read() as i16);
        if (m != spi(4) && spi(4) >= 0) || can_be_deleted(v) == 0 || is_within_area(v, sp(0), sp(1), sp(2), sp(3)) == 0 {
            continue;
        }
        h = vehicle_get_ref(pool(VEHICLE_POOL), v);
        let fl = v.byte_add(0x42A);
        fl.write(fl.read() | 2);
    }
    set_sp(0, h as u32);
    store(s, 1);
}

/// `CRunningScript::ProcessCommands1300To1399(int32 command)` (0x48CDD0, thiscall, `ret 4`): AL = 0
/// continue, -1 unknown.
#[hook(addr = 0x48CDD0, conv = thiscall, name = "CRunningScript::ProcessCommands1300To1399", expect = "8B 44 24 04 83 EC 68", this = "CRunningScript")]
pub fn process_commands_1300_to_1399(s: Scr, command: u32) -> i8 {
    match command {
        // 052C SET_PLAYER_DRUNKENNESS (case 0x48D239): `Players[SP0].m_nDrunkenness (+0x44) = (uint8)SP1;
        // m_nFadeDrunkenness (+0x45) = 0`; `CMBlur::ClearDrunkBlur()` when the byte is 0.
        0x052C => {
            collect(s, 2);
            let p = player_info(sp(0));
            let d = sp(1) as u8;
            p.byte_add(0x44).write(d);
            p.byte_add(0x45).write(0u8);
            if d == 0 {
                clear_drunk_blur();
            }
            0
        }
        // 053F SET_CAN_BURST_CAR_TYRES (case 0x48D356): `veh->bTyresDontBurst` (bit 7 of +0x42B) =
        // SP1 == 0; the vehicle pointer is not null-checked (as in the original). gr: unregistered.
        0x053F => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let b = v.byte_add(0x42B);
            let x = b.read();
            b.write(if sp(1) != 0 { x & 0x7F } else { x | 0x80 });
            0
        }
        // 0570 ADD_SHORT_RANGE_SPRITE_BLIP_FOR_CONTACT_POINT (case 0x48DB3D): type 5 (contact point),
        // colour 2. gr: unregistered.
        0x0570 => {
            short_range_coord_blip(s, 5, 2);
            0
        }
        // 0515, 0521..0525, 0540, 0545, 0548, 054B, 054D, 0551, 0552, 0556, 0557, 055B, 0569 (0x48DD1B, gr:
        // nop/unimplemented): straight to the epilogue, operands not read, return 0.
        0x0515 | 0x0521..=0x0525 | 0x0540 | 0x0545 | 0x0548 | 0x054B | 0x054D => 0,
        0x0551 | 0x0552 | 0x0556 | 0x0557 | 0x055B | 0x0569 => 0,
        0x0517 => {
            property_pickup(s, false);
            0
        }
        0x0518 => {
            property_pickup(s, true);
            0
        }
        // 0519 FREEZE_CAR_POSITION (0x48CFA5, gr: unregistered): no null check. P1 != 0: `+0x40 |= 0x200C`,
        // SkipPhysics, SetVelocity(0,0,0), the `.HOODLUM` 0x45AFB0(0,0,0); else `+0x40 &= ~0x200C`.
        0x0519 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            if sp(1) != 0 {
                set_bits(v, 0x40, 0x200C, true);
                skip_physics(v);
                set_velocity(v, 0, 0, 0);
                physical_45afb0(v, 0, 0, 0);
            } else {
                set_bits(v, 0x40, 0x200C, false);
            }
            0
        }
        // 051A HAS_CHAR_BEEN_DAMAGED_BY_CHAR (0x48D017): both lookups first; a null -> 0; d =
        // `m_pLastDamEntity` (+0x764), null -> 0; flag = d == b, or (b not null-checked) b in a vehicle
        // (+0x46C bit 8) and d == b->m_pVehicle (+0x58C).
        0x051A => {
            collect(s, 2);
            let a = ped_at_ref(pool(PED_POOL), spi(0));
            let b = ped_at_ref(pool(PED_POOL), spi(1));
            let mut r = false;
            if !a.is_null() {
                let d = a.byte_add(0x764).cast::<GPtr<u8>>().read();
                if !d.is_null() {
                    r = d == b;
                    if b.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0
                        && d == b.byte_add(0x58C).cast::<GPtr<u8>>().read()
                    {
                        r = true;
                    }
                }
            }
            set_flag(s, r);
            0
        }
        // 051B HAS_CHAR_BEEN_DAMAGED_BY_CAR (0x48D08F): ped P0 (null -> 0), d = +0x764 (null -> 0); flag =
        // d == VehiclePool->GetAtRef(P1) (looked up only then).
        0x051B => {
            collect(s, 2);
            let a = ped_at_ref(pool(PED_POOL), spi(0));
            let mut r = false;
            if !a.is_null() {
                let d = a.byte_add(0x764).cast::<GPtr<u8>>().read();
                if !d.is_null() {
                    r = d == vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
                }
            }
            set_flag(s, r);
            0
        }
        // 051C HAS_CAR_BEEN_DAMAGED_BY_CHAR (0x48D0D2) / 051D HAS_CAR_BEEN_DAMAGED_BY_CAR (0x48D15B), gr:
        // unregistered: vehicle P0; the other entity looked up only when P1 != -1 (ped / vehicle pool);
        // vehicle null or `m_pLastDamageEntity` (+0x50C) null -> 0. P1 == -1: flag = d's type (`+0x36 & 7`)
        // is ped (3) / vehicle (2). Otherwise flag = d == other; 051C also when (other not null-checked)
        // other is in a vehicle and d == other->m_pVehicle.
        0x051C | 0x051D => {
            collect(s, 2);
            let a = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let t = spi(1);
            let o = if t == -1 {
                GPtr::NULL
            } else if command == 0x051C {
                ped_at_ref(pool(PED_POOL), t)
            } else {
                vehicle_at_ref(pool(VEHICLE_POOL), t)
            };
            let mut r = false;
            if !a.is_null() {
                let d = a.byte_add(0x50C).cast::<GPtr<u8>>().read();
                if !d.is_null() {
                    if t == -1 {
                        r = d.byte_add(0x36).read() & 7 == if command == 0x051C { 3 } else { 2 };
                    } else {
                        r = d == o;
                        if command == 0x051C
                            && o.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0
                            && d == o.byte_add(0x58C).cast::<GPtr<u8>>().read()
                        {
                            r = true;
                        }
                    }
                }
            }
            set_flag(s, r);
            0
        }
        // 051E GET_RADIO_CHANNEL (0x48D1C6, gr: unregistered): no operands; P0 =
        // (int8)AudioEngine.GetCurrentRadioStationID() - 1; StoreParameters(1).
        0x051E => {
            let r = i32::from(get_current_radio_station_id(GPtr::from_addr(AUDIO_ENGINE)) as i8) - 1;
            set_sp(0, r as u32);
            store(s, 1);
            0
        }
        // 0526 SET_CHAR_STAY_IN_CAR_WHEN_JACKED (0x48D1DF), 054A SET_CHAR_CAN_BE_SHOT_IN_VEHICLE (0x48D52D),
        // 0568 SET_CHAR_NEVER_TARGETTED (0x48D9E2): no null check; dword +0x470 bit 0x800000 / 0x4000000 /
        // 0x10000000 = P1 != 0.
        0x0526 | 0x054A | 0x0568 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let m = if command == 0x0526 {
                0x80_0000
            } else if command == 0x054A {
                0x400_0000
            } else {
                0x1000_0000
            };
            set_bits(ped, 0x470, m, sp(1) != 0);
            0
        }
        0x053E => {
            random_car_no_save(s);
            0
        }
        0x0541 => {
            fire_hunter_gun(s);
            0
        }
        // 0547 IS_CHAR_TOUCHING_VEHICLE (0x48D4CE): no null checks; both lookups first; flag =
        // GetHasCollidedWith(in a vehicle (+0x46C bit 8) ? m_pVehicle (not null-checked) : ped, veh) != 0.
        0x0547 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let e = if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
                ped.byte_add(0x58C).cast::<GPtr<u8>>().read()
            } else {
                ped
            };
            let b = get_has_collided_with(e, veh) != 0;
            set_flag(s, b);
            0
        }
        // 054C LOAD_MISSION_TEXT (0x48D587): TheText.LoadMissionText(label) (zeroed 16-byte buffer,
        // divergence note as 0517).
        0x054C => {
            let mut buf = [0u8; 16];
            let l = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, l, 8);
            load_mission_text(GPtr::from_addr(THE_TEXT), l);
            0
        }
        // 054E CLEAR_CHAR_LAST_DAMAGE_ENTITY (0x48D5B0) / 054F CLEAR_CAR_LAST_DAMAGE_ENTITY (0x48D5E8, gr:
        // unregistered): null -> nothing; dword +0x764 (ped) / +0x50C (vehicle) = 0.
        0x054E | 0x054F => {
            collect(s, 1);
            let (e, off) = if command == 0x054E {
                (ped_at_ref(pool(PED_POOL), spi(0)), 0x764)
            } else {
                (vehicle_at_ref(pool(VEHICLE_POOL), spi(0)), 0x50C)
            };
            if !e.is_null() {
                e.byte_add(off).cast::<u32>().write(0);
            }
            0
        }
        // 0550 FREEZE_OBJECT_POSITION (0x48D621, gr: unregistered): no null check; +0x40 bits 0x2004 = P1 != 0.
        0x0550 => {
            collect(s, 2);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            set_bits(o, 0x40, 0x2004, sp(1) != 0);
            0
        }
        // 0555 REMOVE_WEAPON_FROM_CHAR (0x48D673): no null check; ClearWeapon(P1).
        0x0555 => {
            collect(s, 2);
            clear_weapon(ped_at_ref(pool(PED_POOL), spi(0)), sp(1));
            0
        }
        // 055D MAKE_PLAYER_FIRE_PROOF (0x48D6A7): Players[P0] byte +0x14E = P1 != 0.
        0x055D => {
            collect(s, 2);
            player_info(sp(0)).byte_add(0x14E).write(u8::from(sp(1) != 0));
            0
        }
        // 055E INCREASE_PLAYER_MAX_HEALTH (0x48D6DD) / 055F INCREASE_PLAYER_MAX_ARMOUR (0x48D729):
        // Players[P0] byte +0x14F / +0x150 += (u8)P1; its m_pPed's health +0x540 / armour +0x548 = the new
        // byte (`fild`, exact).
        0x055E | 0x055F => {
            collect(s, 2);
            let (bo, fo) = if command == 0x055E { (0x14F, 0x540) } else { (0x150, 0x548) };
            let p = player_info(sp(0));
            let b = p.byte_add(bo).read().wrapping_add(sp(1) as u8);
            p.byte_add(bo).write(b);
            let ped = p.cast::<GPtr<u8>>().read();
            ped.byte_add(fo).cast::<u32>().write(f32::from(b).to_bits());
            0
        }
        0x0560 => {
            random_char_in_car(s, true);
            0
        }
        0x0561 => {
            random_char_in_car(s, false);
            0
        }
        // 0563 ENSURE_PLAYER_HAS_DRIVE_BY_WEAPON (0x48D862): ped = Players[P0].m_pPed; only in a vehicle.
        // Slot 4 type (+0x610) != 0: SetAmmo(it, P1) when its total ammo (+0x61C) < P1 (signed). Else
        // GiveWeapon(0x1C, P1, 1); when `m_nSavedWeapon` (+0x70C) == 0x37 it takes the active slot's type;
        // SetCurrentWeapon(0x1C).
        0x0563 => {
            collect(s, 2);
            let pi = player_info(sp(0));
            let ped = pi.cast::<GPtr<u8>>().read();
            if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
                let w = ped.byte_add(0x610).cast::<u32>().read();
                if w != 0 {
                    if (ped.byte_add(0x61C).cast::<u32>().read() as i32) < spi(1) {
                        set_ammo(ped, w, sp(1));
                    }
                } else {
                    give_weapon(ped, 0x1C, sp(1), 1);
                    let p = pi.cast::<GPtr<u8>>().read();
                    if p.byte_add(0x70C).cast::<u32>().read() == 0x37 {
                        let slot = i32::from(p.byte_add(0x718).read() as i8);
                        let t = p.byte_add(0x5A0 + slot.wrapping_mul(0x1C) as isize).cast::<u32>().read();
                        p.byte_add(0x70C).cast::<u32>().write(t);
                    }
                    set_current_weapon(pi.cast::<GPtr<u8>>().read(), 0x1C);
                }
            }
            0
        }
        // 0564 MAKE_HELI_COME_CRASHING_DOWN (0x48D903, gr: unregistered): no null check; mission (+0x3BA) =
        // 0x3A unless 0x39/0x3A.
        0x0564 => {
            collect(s, 1);
            let m = vehicle_at_ref(pool(VEHICLE_POOL), spi(0)).byte_add(0x3BA);
            let c = m.read();
            if c != 0x39 && c != 0x3A {
                m.write(0x3A);
            }
            0
        }
        // 0565 ADD_EXPLOSION_NO_SOUND (0x48D94A, gr: unregistered): AddExplosion(null, null, P3, (P0,P1,P2),
        // 0, 0, -1.0f, 0).
        0x0565 => {
            collect(s, 4);
            add_explosion(0, 0, sp(3), sp(0), sp(1), sp(2), 0, 0, 0xBF80_0000, 0);
            0
        }
        // 0566 SET_OBJECT_AREA_VISIBLE (0x48D9B2, gr: unregistered): no null check; byte +0x2F = (u8)P1.
        0x0566 => {
            collect(s, 2);
            object_at_ref(pool(OBJECT_POOL), spi(0)).byte_add(0x2F).write(sp(1) as u8);
            0
        }
        // 056A WAS_CUTSCENE_SKIPPED (0x48DA3C, gr: unregistered): no operands; flag = byte 0xB5F854 != 0.
        0x056A => {
            let b = GPtr::<u8>::from_addr(WAS_CUTSCENE_SKIPPED).read() != 0;
            set_flag(s, b);
            0
        }
        // 056C IS_CHAR_IN_ANY_POLICE_VEHICLE (0x48DA4A): no null check; flag = in a vehicle && it != 0 &&
        // IsLawEnforcementVehicle() && its model (word +0x22, re-read) != 0x1AE.
        0x056C => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let mut b = false;
            if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
                let v = ped.byte_add(0x58C).cast::<GPtr<u8>>().read();
                if !v.is_null() && is_law_enforcement_vehicle(v) != 0 {
                    let v = ped.byte_add(0x58C).cast::<GPtr<u8>>().read();
                    b = v.byte_add(0x22).cast::<u16>().read() != 0x1AE;
                }
            }
            set_flag(s, b);
            0
        }
        // 056D DOES_CHAR_EXIST (0x48DAB5) / 056E DOES_VEHICLE_EXIST (0x48DAF9, gr: unregistered): flag =
        // GetAtRef(P0) != null.
        0x056D => {
            collect(s, 1);
            let b = !ped_at_ref(pool(PED_POOL), spi(0)).is_null();
            set_flag(s, b);
            0
        }
        0x056E => {
            collect(s, 1);
            let b = !vehicle_at_ref(pool(VEHICLE_POOL), spi(0)).is_null();
            set_flag(s, b);
            0
        }
        // 0572 SET_ALL_TAXIS_HAVE_NITRO (0x48DC04): P0 != 0 -> CCheat::ApplyCheat(0x5B); else byte 0x96918B = 0.
        0x0572 => {
            collect(s, 1);
            if sp(0) != 0 {
                apply_cheat(0x5B);
            } else {
                GPtr::<u8>::from_addr(TAXI_NITRO_CHEAT).write(0);
            }
            0
        }
        0x0574 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            freeze_no_collision(s, v, 0x2004, true);
            0
        }
        0x0575 => {
            collect(s, 2);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            freeze_no_collision(s, p, 0x2000, false);
            0
        }
        _ => -1,
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1300_TO_1399];
