//! Code bytes that the no-CD protection reads as data ("key cells").
//!
//! Some protection chunks compute a global's address from two constants, one of which is a dword of
//! *game code*, for example (gta_sa.exe 1.0 US):
//!
//! ```text
//! 0x401E3E  pushfd; stc
//! 0x401E40  mov ecx, [0x15606F0]          ; 0x01A0D9D2
//! 0x401E46  adc ecx, [0x410860]           ; 0xFF166AE9 = the entry bytes of CColStore::LoadCollision
//! 0x401E4C  mov ecx, [ecx]                ; [0xB744BC] = CTaskAllocator pool (CTaskAllocator::operator new)
//! ```
//!
//! Hooking such a function rewrites the bytes the decoder reads: with the LoadCollision hook installed,
//! `0x401E46` reads `E9 3B 96 15` (the hook's `jmp`) and the chunk dereferences 0x173715BC, an access
//! violation the first time the game reaches `CTaskAllocator::operator new` (0x69D980). Measured
//! (STREAM-L2 sR300 batch, grove walk, ~frame 4,100): Wine's "Unhandled page fault on read access to
//! 173715BC at address 00401E4C"; that run had left the plain trace at frame 202, so its peds took a
//! path the plain run never takes.
//!
//! The fix: before any hook is installed, every reader in [`READERS`] gets its own cell holding the
//! original dword, and its `disp32` is pointed at that cell. The decoded addresses are then the
//! original ones whatever the hooks write; nothing else changes. [`READERS`] is complete for every
//! hook of the table: the test `every_code_key_read_of_a_hooked_range_is_redirected` scans the
//! exe's code sections for absolute memory reads of a hooked entry's bytes and fails on a reader that is
//! not listed. Decodes from the middle of another instruction (rows of `catalog/code-data-reads.csv`
//! flagged neither `chunk` nor `sweep`) are not readers, and redirecting one would corrupt the
//! instruction it lies in. `SA_KEYCELLS=0` keeps the original operands (to reproduce the crash).

/// One reader: an instruction whose absolute memory operand reads code bytes as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reader {
    /// Instruction address.
    pub at: u32,
    /// The instruction bytes (verified before patching).
    pub insn: [u8; 6],
    /// Offset of the `disp32` within the instruction.
    pub disp_off: u8,
    /// The code address read (the old `disp32`).
    pub key: u32,
    /// The original dword at `key` (from the exe, before any patch).
    pub value: u32,
}

/// Every absolute read of a hooked entry's bytes in gta_sa.exe 1.0 US (`.text`, the no-CD `.text` and
/// `.HOODLUM`), found by the test below.
pub const READERS: [Reader; 35] = [
    // CStreaming::AreAnimsUsedByRequestedModels entry (0x407AD0)
    Reader {
        at: 0x401BCE,
        insn: [0x8B, 0x0D, 0xD0, 0x7A, 0x40, 0x00],
        disp_off: 2,
        key: 0x407AD0,
        value: 0xFF9E_E4E9,
    },
    // CColStore::LoadCollision entry (0x410860); decodes CTaskAllocator's pool 0xB744BC
    Reader {
        at: 0x401E46,
        insn: [0x13, 0x0D, 0x60, 0x08, 0x41, 0x00],
        disp_off: 2,
        key: 0x410860,
        value: 0xFF16_6AE9,
    },
    Reader {
        at: 0x40203A,
        insn: [0x8B, 0x0D, 0xD0, 0x7A, 0x40, 0x00],
        disp_off: 2,
        key: 0x407AD0,
        value: 0xFF9E_E4E9,
    },
    // CAtomicModelInfo::SetupVehicleUpgradeFlags entry (0x4C4570: nop; jmp rel32)
    Reader {
        at: 0x1564881,
        insn: [0x8B, 0x0D, 0x72, 0x45, 0x4C, 0x00],
        disp_off: 2,
        key: 0x4C4572,
        value: 0xFFF3_DCB8,
    },
    // CPhysical::AttachEntityToEntity (quaternion) entry 0x54D690 (PORT-PHYS), found by the test below
    // after the PHYS merge: `xor ebx, [0x54D691]` at 0x40A4C3.
    Reader {
        at: 0x40A4C3,
        insn: [0x33, 0x1D, 0x91, 0xD6, 0x54, 0x00],
        disp_off: 2,
        key: 0x54D691,
        value: 0xEB4C_66E9,
    },
    Reader {
        at: 0x156B570,
        insn: [0x33, 0x35, 0x72, 0x45, 0x4C, 0x00],
        disp_off: 2,
        key: 0x4C4572,
        value: 0xFFF3_DCB8,
    },
    // CBouncingPanel::SetPanel entry 0x6F4920 (PORT-VEH), found by the test below: `adc ebp, [0x6F4924]`
    // at 0x401635.
    Reader {
        at: 0x401635,
        insn: [0x13, 0x2D, 0x24, 0x49, 0x6F, 0x00],
        disp_off: 2,
        key: 0x6F4924,
        value: 0xFFD0_D17D,
    },
    // CCarAI::UpdateCarAI entry 0x41DA30 (PORT-POP-3: nop; jmp rel32): `xor ecx, [0x41DA32]` at 0x407284.
    Reader {
        at: 0x407284,
        insn: [0x33, 0x0D, 0x32, 0xDA, 0x41, 0x00],
        disp_off: 2,
        key: 0x41DA32,
        value: 0xFFFE_6212,
    },
    // CGangWars::CreateAttackWave entry 0x444810 (WRITER-POP-4: push -1; jmp rel32): `adc edi, [0x444813]`
    // at 0x403A92.
    Reader {
        at: 0x403A92,
        insn: [0x13, 0x3D, 0x13, 0x48, 0x44, 0x00],
        disp_off: 2,
        key: 0x444813,
        value: 0xFFFC_22C2,
    },
    // MVP-C batches: CDraw::SetFOV entry 0x6FF410 (`sbb edx, [0x6FF415]` at 0x401375),
    // CTaskComplexWalkRoundCar::Constructor entry 0x6541B0 (`xor ecx, [0x6541B2]` at 0x40153B,
    // `add edx, [0x6541B2]` at 0x401591), CTaskSimpleIKChain::~CTaskSimpleIKChain entry 0x633A90
    // (`add ebp, [0x633A93]` at 0x156BA30), found by the test below.
    Reader {
        at: 0x401375,
        insn: [0x1B, 0x15, 0x15, 0xF4, 0x6F, 0x00],
        disp_off: 2,
        key: 0x6FF415,
        value: 0xFFD0_1C0C,
    },
    Reader {
        at: 0x40153B,
        insn: [0x33, 0x0D, 0xB2, 0x41, 0x65, 0x00],
        disp_off: 2,
        key: 0x6541B2,
        value: 0xDACE_F1E9,
    },
    Reader {
        at: 0x401591,
        insn: [0x03, 0x15, 0xB2, 0x41, 0x65, 0x00],
        disp_off: 2,
        key: 0x6541B2,
        value: 0xDACE_F1E9,
    },
    Reader {
        at: 0x156BA30,
        insn: [0x03, 0x2D, 0x93, 0x3A, 0x63, 0x00],
        disp_off: 2,
        key: 0x633A93,
        value: 0xFFDC_E7C5,
    },
    // CCarEnterExit::SetPedInCarDirect entry 0x650280 (WRITER-MVP-B), found by the test below.
    Reader {
        at: 0x40106E,
        insn: [0x13, 0x1D, 0x81, 0x02, 0x65, 0x00],
        disp_off: 2,
        key: 0x650281,
        value: 0xDB65_6CE9,
    },
    // CTaskSimpleCarSetPedInAsPassenger entry 0x646FE0 (WRITER-MVP-B), found by the test below.
    Reader {
        at: 0x4019C0,
        insn: [0x1B, 0x05, 0xE2, 0x6F, 0x64, 0x00],
        disp_off: 2,
        key: 0x646FE2,
        value: 0xDBAC_13E9,
    },
    // CTaskSimpleHoldEntity::Constructor-1 entry 0x6913A0 (WRITER-MVP-B), found by the test below.
    Reader {
        at: 0x40686C,
        insn: [0x1B, 0x05, 0xA2, 0x13, 0x69, 0x00],
        disp_off: 2,
        key: 0x6913A2,
        value: 0xD728_ACE9,
    },
    // CEntity::Destructor entry 0x535E90 (WRITER-MVP-B), found by the test below.
    Reader {
        at: 0x40744B,
        insn: [0x33, 0x0D, 0x93, 0x5E, 0x53, 0x00],
        disp_off: 2,
        key: 0x535E93,
        value: 0xFFED_10BE,
    },
    // CTaskComplexWalkAlongsidePed::CreateSubTask entry 0x685000 (WRITER-MVP-B), found by the test below.
    Reader {
        at: 0x40A173,
        insn: [0x13, 0x15, 0x02, 0x50, 0x68, 0x00],
        disp_off: 2,
        key: 0x685002,
        value: 0xFFD7_C79C,
    },
    // CPlaneTrail::RegisterPoint entry 0x7172D0 (WRITER-MVP-B), found by the test below.
    Reader {
        at: 0x1562826,
        insn: [0x13, 0x15, 0xD2, 0x72, 0x71, 0x00],
        disp_off: 2,
        key: 0x7172D2,
        value: 0xCF01_AEE9,
    },
    Reader {
        at: 0x156710A,
        insn: [0x1B, 0x35, 0xE2, 0x6F, 0x64, 0x00],
        disp_off: 2,
        key: 0x646FE2,
        value: 0xDBAC_13E9,
    },
    // MVP-F batch mvpf-coll-1: CCollision::SphereCastVsEntity entry 0x419F00 (`push -1; jmp rel32`):
    // `xor ecx, [0x419F03]` at 0x40102D.
    Reader {
        at: 0x40102D,
        insn: [0x33, 0x0D, 0x03, 0x9F, 0x41, 0x00],
        disp_off: 2,
        key: 0x419F03,
        value: 0xFFFF_024E,
    },
    // MVP-F batch mvpf-phys-1: CPhysical::PositionAttachedEntity entry 0x546FF0 (`nop; jmp rel32`):
    // `xor eax, [0x546FF1]` at 0x4015ED.
    Reader {
        at: 0x4015ED,
        insn: [0x33, 0x05, 0xF1, 0x6F, 0x54, 0x00],
        disp_off: 2,
        key: 0x546FF1,
        value: 0xEBF7_E2E9,
    },
    // CAutomobile::UpdateWheelMatrix entry 0x6AA290 (WRITER-MVP-D), found by the test below.
    Reader {
        at: 0x406020,
        insn: [0x1B, 0x0D, 0x91, 0xA2, 0x6A, 0x00],
        disp_off: 2,
        key: 0x6AA291,
        value: 0xD577_EEE9,
    },
    // CWaterLevel::SetUpWaterFog entry 0x6EA9F0 (WRITER-MVP-D), found by the test below.
    Reader {
        at: 0x4014C9,
        insn: [0x2B, 0x0D, 0xF1, 0xA9, 0x6E, 0x00],
        disp_off: 2,
        key: 0x6EA9F1,
        value: 0xFFD1_9104,
    },
    // 0x6FBA20 (CCoronas::RenderSunReflection's fade helper) entry (WRITER-MVP-D), found by the test below.
    Reader {
        at: 0x4038F3,
        insn: [0x03, 0x2D, 0x21, 0xBA, 0x6F, 0x00],
        disp_off: 2,
        key: 0x6FBA21,
        value: 0xFFD0_AA82,
    },
    // CPostEffects::ImmediateModeRenderStatesStore entry 0x700CC0 (WRITER-MVP-D), found by the test below.
    Reader {
        at: 0x409B7F,
        insn: [0x33, 0x0D, 0xC1, 0x0C, 0x70, 0x00],
        disp_off: 2,
        key: 0x700CC1,
        value: 0xFFD0_8DE5,
    },
    // CFont::GetStringWidth entry 0x71A0E0 (`nop; jmp 0x4016B9`, PORT-HUD-3): `mov ecx, [0x71A0E2]` at
    // 0x40641C (the rotate count of the protection chunk's `rol ebp, cl`).
    Reader {
        at: 0x40641C,
        insn: [0x8B, 0x0D, 0xE2, 0xA0, 0x71, 0x00],
        disp_off: 2,
        key: 0x71A0E2,
        value: 0xFFCE_75D3,
    },
    // Batch mvph-phys-1: CPhysical::ApplySoftCollision-2 entry 0x54A2C0 (`nop; jmp rel32`):
    // `mov eax, [0x54A2C1]` at 0x406B2B (5 bytes; `insn` includes the next byte, 0x33).
    Reader {
        at: 0x406B2B,
        insn: [0xA1, 0xC1, 0xA2, 0x54, 0x00, 0x33],
        disp_off: 1,
        key: 0x54A2C1,
        value: 0xEBF4_89E9,
    },
    // Batch mvpi-ped-1: CPedClothesDesc::GetIsWearingBalaclava entry 0x5A7950, found by the test below.
    Reader {
        at: 0x407E2C,
        insn: [0x1B, 0x05, 0x55, 0x79, 0x5A, 0x00],
        disp_off: 2,
        key: 0x5A7955,
        value: 0xFFE5_BF2E,
    },
    // Batch mvpi-ped-1: CPed::RemoveGogglesModel entry 0x5DF170, found by the test below.
    Reader {
        at: 0x409AC8,
        insn: [0x13, 0x1D, 0x74, 0xF1, 0x5D, 0x00],
        disp_off: 2,
        key: 0x5DF174,
        value: 0xE223_A3E9,
    },
    // Batch mvpl-ped-1: CWanted::ClearQdCrimes entry 0x561FE0 (`lea eax, [ecx+0x34]; jmp 0x404CD2`):
    // `adc edi, [0x561FE3]` at 0x401409 (the jmp's rel32 and its opcode byte).
    Reader {
        at: 0x401409,
        insn: [0x13, 0x3D, 0xE3, 0x1F, 0x56, 0x00],
        disp_off: 2,
        key: 0x561FE3,
        value: 0xEA2C_EAE9,
    },
    // Batch mvpl-audio-1: CAEPedAudioEntity::~CAEPedAudioEntity entry 0x5DE920 (`push -1; jmp 0x4084CA`):
    // `adc ebx, [0x5DE923]` at 0x402195.
    Reader {
        at: 0x402195,
        insn: [0x13, 0x1D, 0x23, 0xE9, 0x5D, 0x00],
        disp_off: 2,
        key: 0x5DE923,
        value: 0xFFE2_9BA3,
    },
    // Batch writero-audio-1: CAEVehicleAudioEntity::AddAudioEvent entry 0x4F6420 (`push -1; jmp 0x40727C`):
    // `adc ecx, [0x4F6423]` at 0x401931 (the jmp's rel32 bytes).
    Reader {
        at: 0x401931,
        insn: [0x13, 0x0D, 0x23, 0x64, 0x4F, 0x00],
        disp_off: 2,
        key: 0x4F6423,
        value: 0xFFF1_0E55,
    },
    // Batch std12-veh-1: CDoor::Process entry 0x6F4040 (`push -1; jmp 0x407DD5`): `sbb edi, [0x6F4043]` at
    // 0x4072EE (the jmp's rel32 bytes).
    Reader {
        at: 0x4072EE,
        insn: [0x1B, 0x3D, 0x43, 0x40, 0x6F, 0x00],
        disp_off: 2,
        key: 0x6F4043,
        value: 0xFFD1_3D8E,
    },
    // Batch std13-ai-1: CPedAttractor::CPedAttractor entry 0x5EDFB0 (`push -1; jmp 0x404CB7`): `adc edx, [0x5EDFB2]`
    // at 0x40206A (the jmp opcode and three rel32 bytes).
    Reader {
        at: 0x40206A,
        insn: [0x13, 0x15, 0xB2, 0xDF, 0x5E, 0x00],
        disp_off: 2,
        key: 0x5EDFB2,
        value: 0xE16D_00E9,
    },
];

#[cfg(windows)]
mod install {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::READERS;
    use crate::win::{self, log};

    static CELLS: [AtomicU32; READERS.len()] = [const { AtomicU32::new(0) }; READERS.len()];

    /// Redirects every reader to a private copy of the original dword. Called at attach, before the
    /// hooks are installed (game threads suspended).
    pub(crate) fn install() {
        if std::env::var("SA_KEYCELLS").is_ok_and(|v| v == "0") {
            log("keycells: SA_KEYCELLS=0, protection readers keep reading the (hooked) code bytes");
            return;
        }
        let mut n = 0;
        for (r, cell) in READERS.iter().zip(CELLS.iter()) {
            let cur = win::read(r.at, r.insn.len());
            let key = win::read(r.key, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            if cur.as_deref() != Some(&r.insn[..]) || key != Some(r.value) {
                log(&format!(
                    "keycells: {:#x} unexpected bytes {cur:02x?} / key {key:x?}, not redirected",
                    r.at
                ));
                continue;
            }
            cell.store(r.value, Ordering::Release);
            let addr = cell as *const AtomicU32 as usize as u32;
            match win::patch(r.at + r.disp_off as u32, &addr.to_le_bytes()) {
                Ok(()) => n += 1,
                Err(e) => log(&format!("keycells: {:#x}: {e}", r.at)),
            }
        }
        log(&format!(
            "keycells: {n}/{} protection readers of code bytes redirected to copies of the original dwords",
            READERS.len()
        ));
    }
}
#[cfg(windows)]
pub(crate) use install::install;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use iced_x86::{Decoder, DecoderOptions, OpKind, Register};

    use super::READERS;
    use crate::table::{HOOKS, Kind};

    /// The bytes a hook rewrites: the rel32 of a `.HOODLUM` stub, or the relocated entry (whole
    /// instructions covering 5 bytes, as `install_one` computes it).
    fn patched(exe: &crate::testexe::Exe, addr: u32, kind: Kind) -> (u32, u32) {
        match kind {
            Kind::HoodlumStub => (addr + 1, addr + 5),
            Kind::Inline => {
                let consumed = crate::x86::relocate_keep_return(exe.bytes(addr, 32), addr, 0x1000_0000, 5)
                    .map(|r| r.consumed as u32)
                    .unwrap_or(16);
                (addr, addr + consumed)
            }
        }
    }

    #[test]
    fn readers_match_the_exe() {
        let Some(exe) = crate::testexe::load() else { return };
        for r in READERS {
            assert_eq!(exe.bytes(r.at, 6), &r.insn[..], "{:#x}", r.at);
            let d = r.disp_off as usize;
            assert_eq!(u32::from_le_bytes(r.insn[d..d + 4].try_into().unwrap()), r.key);
            assert_eq!(
                u32::from_le_bytes(exe.bytes(r.key, 4).try_into().unwrap()),
                r.value
            );
        }
    }

    #[test]
    fn every_code_key_read_of_a_hooked_range_is_redirected() {
        let Some(exe) = crate::testexe::load() else { return };
        // dword address → hook, for every dword that overlaps a patched range
        let mut keys: BTreeMap<u32, u32> = BTreeMap::new();
        for h in HOOKS.iter() {
            let (lo, hi) = patched(&exe, h.addr, h.kind);
            for k in lo.saturating_sub(3)..hi {
                keys.insert(k, h.addr);
            }
        }
        // Decodes that start inside another instruction are not readers: the rows of
        // catalog/code-data-reads.csv that lie neither in a protection chunk nor on a linear sweep
        // (the same criterion as `data_reads::unredirected`). E.g. `mov edx, [0x6A004D]` "at"
        // 0xD723B1 / `sub edx, [0x6A004D]` "at" 0xD72411 (CText::Get's first byte) are the rel32 of
        // `call 0x1243940` at 0xD723B0 / 0xD72410 and the following `push 0`.
        let off_stream: std::collections::BTreeSet<(u32, u32)> = crate::data_reads::rows()
            .into_iter()
            .filter(|r| !r.chunk && !r.sweep)
            .map(|r| (r.reader, r.target))
            .collect();
        let mut found: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        for (va, data) in exe.code_sections() {
            for p in 0..data.len().saturating_sub(3) {
                let v = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
                let Some(&hook) = keys.get(&v) else { continue };
                for back in 1..=6usize {
                    let Some(st) = p.checked_sub(back) else { continue };
                    let ip = va + st as u32;
                    let mut dec = Decoder::with_ip(32, exe.bytes(ip, 16), ip as u64, DecoderOptions::NONE);
                    let i = dec.decode();
                    if i.is_invalid() || i.len() < back + 4 {
                        continue;
                    }
                    let absolute = (0..i.op_count()).any(|o| i.op_kind(o) == OpKind::Memory)
                        && i.memory_base() == Register::None
                        && i.memory_index() == Register::None
                        && i.memory_displacement32() == v;
                    if absolute && !off_stream.contains(&(ip, v)) {
                        found.insert(ip, (v, hook));
                    }
                }
            }
        }
        let listed: Vec<u32> = READERS.iter().map(|r| r.at).collect();
        let missing: Vec<String> = found
            .iter()
            .filter(|(at, _)| !listed.contains(at))
            .map(|(at, (k, h))| format!("{at:#x} reads {k:#x} (hook {h:#x})"))
            .collect();
        assert!(missing.is_empty(), "unlisted code-key readers: {missing:?}");
        // and no stale entry: every listed reader reads a hooked range
        for at in listed {
            assert!(
                found.contains_key(&at),
                "{at:#x} is listed but reads no hooked bytes"
            );
        }
    }
}
