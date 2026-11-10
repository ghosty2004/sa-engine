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
//! not listed. `SA_KEYCELLS=0` keeps the original operands (to reproduce the crash).

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
pub const READERS: [Reader; 6] = [
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
    // CStreaming::AreAnimsUsedByRequestedModels entry (0x407AD0)
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
                    if absolute {
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
