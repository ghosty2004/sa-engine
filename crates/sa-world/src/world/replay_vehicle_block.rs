//! Replay recording of a vehicle (batch `mvpm-world-1`): `tReplayVehicleBlock::MakeVehicleUpdateData`. Ported from the
//! 1.0 US bytes (gta-reversed `Replay.cpp` as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::world::common::{find_player_vehicle, g, rd, wr, x};

orig! {
    /// `CCompressedMatrixNotAligned::CompressFromFullMatrix(CMatrix&)` (0x59BAD0, `ret 4`)
    fn compressed_matrix_compress_from_full(c: GPtr<u8>, m: GPtr<u8>) = thiscall 0x59BAD0 => unported;
}

/// `CReplay` "player vehicle recorded without its siren" byte (0xB6EC20)
const PLAYER_VEHICLE_FLAG: usize = 0xB6EC20;

fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn byte_of(v: X87) -> u8 {
    v.ftol() as u8
}
fn less(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(5)
}

/// A move-speed component clamped as the original branches: `!(hi < c) && lo > c` → `lo`, else `hi < c` → `hi`, else
/// `c`; then `_ftol(v * 127)` (0x859C38) as a byte.
fn speed_byte(c: f32) -> u8 {
    let (hi, lo) = (x(kf(0x858B90)), x(kf(0x859C3C)));
    let cv = x(c);
    let v = if !less(hi, cv) && !lo.fcom(cv).test_any(0x41) {
        lo
    } else if less(hi, cv) {
        hi
    } else {
        cv
    };
    byte_of(v.fmul_m(kf(0x859C38)))
}

/// `tReplayVehicleBlock::MakeVehicleUpdateData(CVehicle*, uint8 poolRef)` (0x45B730, thiscall on the block, `ret 8`).
///
/// +1 pool ref; +4 the compressed matrix; +2 `_ftol(min-ish(gas pedal +0x4C0, k) * k2)` (k replaces the pedal when
/// `k < pedal`); +3 `_ftol(steer angle +0x49C * k3)`; +0x1C the automobile's +0x5B4 (other types 0); +0x20..+0x22 the
/// clamped move speed ([`speed_byte`]); +0x1A model; +0x2D/+0x2E colours +0x434/+0x435; flags +0x2F bits 1, 2, 3 from
/// the entity flags (+0x1C bits 7, 30, 31), bit 1 cleared for the player's vehicle while the byte at 0xB6EC20 is set;
/// +0x23 `_ftol(+0x94C * k)` for model 0x1B0 else `_ftol(+0x494 * 50)`. Automobiles (+0x590 == 0): the wheel bytes
/// +0x24..+0x27 (`+0x7D4.. * 50`) and +0x28..+0x2B (`+0x828.. * k`), interleaved in that order, +0x18 / +0x19 (`+0x5F4`
/// / `+0x60C * k`), and +0x2C bits 0..5 for the door bytes +0x5A9..+0x5AE equal to 4. Finally +0x2F bit 0 = physical
/// flag +0x40 bit 29 and +0x30 = the vehicle sub-type byte +0x594.
#[hook(addr = 0x45b730, conv = thiscall, name = "tReplayVehicleBlock::MakeVehicleUpdateData", expect = "8A 44 24 08 53 56 8B F1",
       unverified = "mvpm-world-1", route = "L2", hazards = "heap-writes", recs = "driving",
       note = "writes the replay buffer block passed in ECX")]
pub fn make_vehicle_update_data(this: GPtr<u8>, veh: GPtr<u8>, pool_ref: u32) {
    wr::<u8>(this, 1, pool_ref as u8);
    compressed_matrix_compress_from_full(this.byte_add(4), rd::<GPtr<u8>>(veh, 0x14));
    let pedal = x(rd::<f32>(veh, 0x4C0));
    let k = x(kf(0x858C4C));
    let p = if less(k, pedal) { k } else { pedal };
    wr::<u8>(this, 2, byte_of(p.fmul_m(kf(0x858C84))));
    wr::<u8>(this, 3, byte_of(x(rd::<f32>(veh, 0x49C)).fmul_m(kf(0x858628))));
    let automobile = rd::<u32>(veh, 0x590) == 0;
    wr::<u32>(this, 0x1C, if automobile { rd::<u32>(veh, 0x5B4) } else { 0 });
    wr::<u8>(this, 0x20, speed_byte(rd::<f32>(veh, 0x44)));
    wr::<u8>(this, 0x21, speed_byte(rd::<f32>(veh, 0x48)));
    wr::<u8>(this, 0x22, speed_byte(rd::<f32>(veh, 0x4C)));
    let mut b = rd::<u8>(this, 0x2F);
    wr::<u16>(this, 0x1A, rd::<u16>(veh, 0x22));
    wr::<u8>(this, 0x2D, rd::<u8>(veh, 0x434));
    wr::<u8>(this, 0x2E, rd::<u8>(veh, 0x435));
    let f = rd::<u32>(veh, 0x1C);
    b = (b & !2) | ((((f >> 7) as u8) << 1) & 2);
    wr::<u8>(this, 0x2F, b);
    b = (b & !4) | ((((rd::<u32>(veh, 0x1C) >> 30) as u8) << 2) & 4);
    wr::<u8>(this, 0x2F, b);
    b = (b & !8) | ((((rd::<u32>(veh, 0x1C) >> 31) as u8) << 3) & 8);
    wr::<u8>(this, 0x2F, b);
    if find_player_vehicle(-1, 0) == veh && g::<u8>(PLAYER_VEHICLE_FLAG).read() != 0 {
        wr::<u8>(this, 0x2F, rd::<u8>(this, 0x2F) & 0xFD);
    }
    let v23 = if rd::<u16>(veh, 0x22) == 0x1B0 {
        x(rd::<f32>(veh, 0x94C)).fmul_m(kf(0x859C34))
    } else {
        x(rd::<f32>(veh, 0x494)).fmul_m(kf(0x858B40))
    };
    wr::<u8>(this, 0x23, byte_of(v23));
    if rd::<u32>(veh, 0x590) == 0 {
        for w in 0..4isize {
            wr::<u8>(this, 0x24 + w, byte_of(x(rd::<f32>(veh, 0x7D4 + 4 * w)).fmul_m(kf(0x858B40))));
            wr::<u8>(this, 0x28 + w, byte_of(x(rd::<f32>(veh, 0x828 + 4 * w)).fmul_m(kf(0x859C34))));
        }
        wr::<u8>(this, 0x18, byte_of(x(rd::<f32>(veh, 0x5F4)).fmul_m(kf(0x859C30))));
        wr::<u8>(this, 0x19, byte_of(x(rd::<f32>(veh, 0x60C)).fmul_m(kf(0x859C30))));
        wr::<u8>(this, 0x2C, 0);
        for (k, bit) in [1u8, 2, 4, 8, 0x10, 0x20].iter().enumerate() {
            if rd::<u8>(veh, 0x5A9 + k as isize) == 4 {
                wr::<u8>(this, 0x2C, rd::<u8>(this, 0x2C) | bit);
            }
        }
    }
    let b = rd::<u8>(this, 0x2F);
    wr::<u8>(this, 0x2F, (b & !1) | ((rd::<u32>(veh, 0x40) >> 29) as u8 & 1));
    wr::<u8>(this, 0x30, rd::<u8>(veh, 0x594));
}

pub const HOOKS: &[HookReg] = &[HOOK_MAKE_VEHICLE_UPDATE_DATA];
