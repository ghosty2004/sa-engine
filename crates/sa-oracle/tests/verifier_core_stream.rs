#![cfg(not(windows))]
//! Verifier probes for the PORT-CORE / PORT-STREAM L1 claims (`catalog/state.csv` rows in state L1
//! with evidence PORT-CORE / PORT-STREAM). Written independently of the ports' own tests; findings in
//! `docs/re/verify-l1-core-stream.md`.
//!
//! Run (Unicorn only; the native backend is never used here):
//! `cargo test --release -p sa-oracle --test verifier_core_stream -- --test-threads=1 --nocapture`
//!
//! Knobs: `VERIFY_AUDIT_CASES` (generated cases per spec for the instrumented audit, default 100k),
//! `VERIFY_MUTANT_CASES` (generated cases per mutant, default 100k), `VERIFY_ONLY` (comma-separated
//! spec keys).

use sa_oracle::call::*;
use sa_oracle::generate::Rng;
use sa_oracle::runner::{self, Backend, Cfg};
use sa_oracle::specs::{self, ExpRet, Expect, Inputs, RetKind, Spec};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

fn img() -> Option<Arc<sa_oracle::pe::Image>> {
    let i = sa_oracle::image_cached();
    if i.is_none() {
        eprintln!("note: gta_sa.exe 1.0 US not found, test skipped");
    }
    i
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.replace('_', "").parse().ok())
        .unwrap_or(default)
}

fn only(key: &str) -> bool {
    match std::env::var("VERIFY_ONLY") {
        Ok(v) if !v.is_empty() => v.split(',').any(|k| k.eq_ignore_ascii_case(key)),
        _ => true,
    }
}

/// The claimed functions (state L1, evidence PORT-CORE / PORT-STREAM) from `catalog/state.csv`.
fn claimed() -> BTreeMap<u32, String> {
    let text = std::fs::read_to_string(sa_oracle::workspace_root().join("catalog/state.csv")).unwrap();
    let mut out = BTreeMap::new();
    for l in text.lines().skip(1) {
        let f: Vec<&str> = l.splitn(7, ',').collect();
        if f.len() < 6 || f[3] != "L1" {
            continue;
        }
        let ev = f[5..].join(",");
        if !(ev.contains("PORT-CORE") || ev.contains("PORT-STREAM")) {
            continue;
        }
        let a = u32::from_str_radix(f[0].trim_start_matches("0x"), 16).unwrap();
        out.insert(a, f[1].to_string());
    }
    out
}

fn claimed_specs() -> Vec<Spec> {
    let c = claimed();
    specs::all()
        .into_iter()
        .filter(|s| c.contains_key(&s.addr))
        .collect()
}

/// Unicorn only: native backend never used (`snan_native: false`), SNaN cases count as unverified.
fn cfg(cases: u64) -> Cfg {
    Cfg {
        cases,
        backend: Backend::Unicorn,
        threads: 8,
        fixtures: false,
        keep: 2,
        shrink: false,
        snan_native: false,
        ..Cfg::default()
    }
}

// ======================================================= 1. every claim has a spec ================

#[test]
fn verifier_every_claim_has_a_spec() {
    let c = claimed();
    let keys: BTreeSet<u32> = specs::all().iter().map(|s| s.addr).collect();
    let missing: Vec<_> = c.iter().filter(|(a, _)| !keys.contains(a)).collect();
    println!(
        "{} claimed functions, {} without a spec: {missing:?}",
        c.len(),
        missing.len()
    );
    assert!(missing.is_empty());
}

// =========================================== 2. instrumented audit of the original under the specs ====

mod audit {
    use super::*;
    use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction, Mnemonic};
    use unicorn_engine::{Arch, HookType, MemType, Mode, Prot, RegisterX86 as R, Unicorn};

    const EXIT: u32 = 0x10000;
    const ESP0: u32 = 0x1F_0000;
    const STACK_LO: u32 = 0x10_0000;
    const STACK_HI: u32 = 0x20_0000;
    const TIB: u32 = SCRATCH + 0x8000;

    #[derive(Default)]
    pub struct St {
        prev_block: u32,
        pub blocks: BTreeSet<(u32, u32)>,
        pub bedges: BTreeSet<(u32, u32)>,
        /// esp at entry (points at the return address)
        esp_in: u32,
        nargs: u32,
        generation: u32,
        stack_gen: Vec<u32>,
        pub w_scratch_hi: BTreeMap<u32, u64>,
        pub w_args: BTreeMap<u32, u64>,
        pub w_other: BTreeMap<u32, u64>,
        pub r_scratch_hi: BTreeMap<u32, u64>,
        pub r_uninit_stack: BTreeMap<i32, u64>,
    }

    pub struct Report {
        pub instructions: usize,
        pub uncovered: Vec<(u32, String)>,
        pub jcc: Vec<(u32, String, bool, bool)>,
        pub st: St,
        pub errors: u64,
        pub edx_changed: u64,
        pub ecx_changed: u64,
        pub eax_hi_nonzero_al: u64,
        pub calls: u64,
    }

    fn bump(m: &mut BTreeMap<u32, u64>, k: u32) {
        *m.entry(k).or_default() += 1;
    }

    /// Runs the spec's calls on the spec's patched image under a bare Unicorn with block, write and
    /// read hooks. Coverage is reported for the function's own catalogued ranges; memory findings for
    /// the whole call (callees included).
    pub fn run(img: &Arc<sa_oracle::pe::Image>, spec: &Spec, inputs: &[Inputs]) -> Report {
        let img = runner::patched(img, spec);
        let cat = sa_oracle::catalog_cached().unwrap();
        let own = cat.ranges(spec.addr).unwrap();
        let mut uc = Unicorn::new_with_data(Arch::X86, Mode::MODE_32, St::default()).unwrap();
        uc.get_data_mut().stack_gen = vec![0; (STACK_HI - STACK_LO) as usize];
        uc.ctl_set_cpu_model(unicorn_engine::X86CpuModel::PENTIUM3 as i32)
            .unwrap();
        let cr4 = uc.reg_read(R::CR4).unwrap();
        uc.reg_write(R::CR4, cr4 | 0x600).unwrap();
        let cr0 = uc.reg_read(R::CR0).unwrap();
        uc.reg_write(R::CR0, (cr0 & !0x4) | 0x22).unwrap();
        let page = |x: u32| (x + 0xFFF) & !0xFFF;
        uc.mem_map(EXIT as u64, 0x1000, Prot::ALL).unwrap();
        uc.mem_write(EXIT as u64, &[0xF4]).unwrap();
        uc.mem_map(
            STACK_LO as u64,
            (STACK_HI - STACK_LO) as u64,
            Prot::READ | Prot::WRITE,
        )
        .unwrap();
        uc.mem_map(img.base as u64, page(img.flat.len() as u32) as u64, Prot::ALL)
            .unwrap();
        uc.mem_write(img.base as u64, &img.flat).unwrap();
        uc.mem_map(SCRATCH as u64, 0x1_0000, Prot::READ | Prot::WRITE)
            .unwrap();
        let mut msr = [0u8; 16];
        msr[..4].copy_from_slice(&0xC000_0100u32.to_le_bytes());
        msr[8..].copy_from_slice(&(TIB as u64).to_le_bytes());
        uc.reg_write_long(R::MSR, &msr).unwrap();
        uc.add_block_hook(1, 0, |uc, a, size| {
            let d = uc.get_data_mut();
            let a = a as u32;
            d.blocks.insert((a, size));
            if d.prev_block != 0 {
                d.bedges.insert((d.prev_block, a));
            }
            d.prev_block = a;
        })
        .unwrap();
        uc.add_mem_hook(HookType::MEM_WRITE, 1, 0, |uc, _t, a, size, _v| {
            let d = uc.get_data_mut();
            let a = a as u32;
            let n = size as u32;
            if (STACK_LO..STACK_HI).contains(&a) {
                if a >= d.esp_in + 4 && a < d.esp_in + 4 + 4 * d.nargs {
                    bump(&mut d.w_args, a - d.esp_in);
                } else if a >= d.esp_in + 4 {
                    bump(&mut d.w_other, a);
                }
                for k in 0..n {
                    let o = (a + k - STACK_LO) as usize;
                    if o < d.stack_gen.len() {
                        d.stack_gen[o] = d.generation;
                    }
                }
            } else if (SCRATCH..SCRATCH + SCRATCH_LEN as u32).contains(&a) || (TIB..TIB + 4).contains(&a) {
                // compared scratch, SEH head
            } else if (SCRATCH..SCRATCH + 0x1_0000).contains(&a) {
                bump(&mut d.w_scratch_hi, a - SCRATCH);
            } else if !(EXIT..EXIT + 0x1000).contains(&a) {
                bump(&mut d.w_other, a);
            }
            true
        })
        .unwrap();
        uc.add_mem_hook(
            HookType::MEM_READ,
            SCRATCH as u64 + SCRATCH_LEN as u64,
            SCRATCH as u64 + 0xFFFF,
            |uc, _t, a, _s, _v| {
                let a = a as u32;
                if !(TIB..TIB + 4).contains(&a) {
                    bump(&mut uc.get_data_mut().r_scratch_hi, a - SCRATCH);
                }
                true
            },
        )
        .unwrap();
        uc.add_mem_hook(
            HookType::MEM_READ,
            STACK_LO as u64,
            ESP0 as u64 - 1,
            |uc, _t, a, size, _v| {
                let d = uc.get_data_mut();
                let a = a as u32;
                if a < d.esp_in {
                    let all_written =
                        (0..size as u32).all(|k| d.stack_gen[(a + k - STACK_LO) as usize] == d.generation);
                    if !all_written {
                        let off = a as i32 - d.esp_in as i32;
                        *d.r_uninit_stack.entry(off).or_default() += 1;
                    }
                }
                true
            },
        )
        .unwrap();
        uc.add_mem_hook(HookType::MEM_INVALID, 1, 0, |_uc, _t: MemType, _a, _s, _v| false)
            .unwrap();

        let mut rep = Report {
            instructions: 0,
            uncovered: vec![],
            jcc: vec![],
            st: St::default(),
            errors: 0,
            edx_changed: 0,
            ecx_changed: 0,
            eax_hi_nonzero_al: 0,
            calls: 0,
        };
        for inp in inputs {
            let c = (spec.call)(inp);
            uc.mem_write(SCRATCH as u64, &c.scratch).unwrap();
            uc.mem_write(TIB as u64, &TIB_SEH_END.to_le_bytes()).unwrap();
            let esp = ESP0 - 4 * c.stack.len() as u32 - 4;
            let fill: Vec<u8> = STACK_FILL
                .to_le_bytes()
                .iter()
                .copied()
                .cycle()
                .take(STACK_FILL_LEN as usize)
                .collect();
            uc.mem_write((esp - STACK_FILL_LEN) as u64, &fill).unwrap();
            let mut st = EXIT.to_le_bytes().to_vec();
            for v in &c.stack {
                st.extend_from_slice(&v.to_le_bytes());
            }
            uc.mem_write(esp as u64, &st).unwrap();
            {
                let d = uc.get_data_mut();
                d.generation += 1;
                d.esp_in = esp;
                d.nargs = c.stack.len() as u32;
                d.prev_block = 0;
            }
            // stack below esp written by the setup counts as "written" only for the return address
            for (r, v) in [
                (R::EAX, c.addr),
                (R::ECX, c.ecx),
                (R::EDX, c.edx),
                (R::EBX, 0),
                (R::ESI, 0),
                (R::EDI, 0),
                (R::EBP, 0),
                (R::ESP, esp),
                (R::EFLAGS, 0x202),
            ] {
                uc.reg_write(r, v as u64).unwrap();
            }
            uc.reg_write(R::FPCW, c.fpu.cw as u64).unwrap();
            uc.reg_write(R::FPSW, 0).unwrap();
            uc.reg_write(R::FPTAG, 0xFFFF).unwrap();
            uc.reg_write(R::MXCSR, (c.fpu.mxcsr & !0x8000) as u64).unwrap();
            let r = uc.emu_start(spec.addr as u64, EXIT as u64, 0, spec.budget as usize);
            let pc = uc.reg_read(R::EIP).unwrap() as u32;
            rep.calls += 1;
            if r.is_err() || pc != EXIT {
                rep.errors += 1;
                continue;
            }
            let edx = uc.reg_read(R::EDX).unwrap() as u32;
            let ecx = uc.reg_read(R::ECX).unwrap() as u32;
            let eax = uc.reg_read(R::EAX).unwrap() as u32;
            rep.edx_changed += (edx != c.edx) as u64;
            rep.ecx_changed += (ecx != c.ecx) as u64;
            if spec.ret == RetKind::Al {
                rep.eax_hi_nonzero_al += (eax >> 8 != 0) as u64;
            }
        }
        let d = std::mem::take(uc.get_data_mut());
        // coverage over the function's own ranges
        for &(lo, hi) in &own {
            let bytes = img.bytes(lo, (hi - lo) as usize);
            let mut dec = Decoder::with_ip(32, bytes, lo as u64, DecoderOptions::NONE);
            let mut ins = Instruction::default();
            while dec.can_decode() {
                dec.decode_out(&mut ins);
                let a = ins.ip32();
                let filler = matches!(ins.mnemonic(), Mnemonic::Nop | Mnemonic::Int3) || ins.is_invalid();
                if filler {
                    continue;
                }
                rep.instructions += 1;
                let text = format!("{:?}", ins.mnemonic());
                let exec = d.blocks.iter().any(|&(b, s)| a >= b && a < b + s);
                if !exec {
                    rep.uncovered.push((a, text.clone()));
                }
                if ins.flow_control() == FlowControl::ConditionalBranch {
                    let (t, n) = (ins.near_branch32(), ins.next_ip32());
                    // blocks ending right after this jcc
                    let owners: Vec<u32> = d
                        .blocks
                        .iter()
                        .filter(|&&(b, s)| b <= a && b + s == n)
                        .map(|&(b, _)| b)
                        .collect();
                    let taken = owners.iter().any(|&b| d.bedges.contains(&(b, t)));
                    let fall = owners.iter().any(|&b| d.bedges.contains(&(b, n)));
                    rep.jcc.push((a, text, taken, fall));
                }
            }
        }
        rep.st = d;
        rep
    }
}

fn top<K: std::fmt::Debug + Copy>(m: &BTreeMap<K, u64>, n: usize) -> String {
    m.iter()
        .take(n)
        .map(|(k, v)| format!("{k:x?}:{v}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// For every claimed spec: branch coverage of the original under (edges + generated) inputs and
/// under the generator alone; writes the comparison does not see (scratch beyond `SCRATCH_LEN`, the
/// argument area, anything else); reads of uncompared scratch and of uninitialised stack; EDX/ECX
/// changes; AL functions' upper EAX bits.
#[test]
fn verifier_audit_claimed_specs() {
    let Some(img) = img() else { return };
    let n = env_u64("VERIFY_AUDIT_CASES", 100_000);
    let specs: Vec<Spec> = claimed_specs().into_iter().filter(|s| only(s.key)).collect();
    let results = std::sync::Mutex::new(Vec::new());
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|sc| {
        for _ in 0..8 {
            sc.spawn(|| loop {
                let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(spec) = specs.get(k) else { break };
                let edges = (spec.edges)();
                let gens: Vec<Inputs> = (0..n).map(|i| (spec.generate)(&mut Rng::for_case(0x5A_0001, i))).collect();
                let all: Vec<Inputs> = edges.iter().cloned().chain(gens.iter().cloned()).collect();
                let a = audit::run(&img, spec, &all);
                let g = audit::run(&img, spec, &gens);
                let mut s = String::new();
                use std::fmt::Write as _;
                let partial: Vec<_> = a.jcc.iter().filter(|j| !(j.2 && j.3)).collect();
                let gpartial: Vec<_> = g.jcc.iter().filter(|j| !(j.2 && j.3)).collect();
                let _ = writeln!(
                    s,
                    "{} {} @ {:#x}: {} calls ({} edges), {} errors; {} instructions, {} uncovered, {} jcc, {} one-sided (generator alone: {} uncovered, {} one-sided)",
                    spec.key, spec.name, spec.addr, a.calls, edges.len(), a.errors, a.instructions, a.uncovered.len(),
                    a.jcc.len(), partial.len(), g.uncovered.len(), gpartial.len()
                );
                for (ad, t) in a.uncovered.iter().take(12) {
                    let _ = writeln!(s, "    uncovered {ad:#x}: {t}");
                }
                for (ad, t, tk, fl) in &partial {
                    let _ = writeln!(s, "    jcc {ad:#x} {t}: taken={tk} fallthrough={fl}");
                }
                for (ad, t, tk, fl) in &gpartial {
                    if !partial.iter().any(|p| p.0 == *ad) {
                        let _ = writeln!(s, "    edge-only: jcc {ad:#x} {t}: generator taken={tk} fallthrough={fl}");
                    }
                }
                let st = &a.st;
                if !st.w_scratch_hi.is_empty() {
                    let _ = writeln!(s, "    WRITES beyond the compared scratch (offset:count): {}", top(&st.w_scratch_hi, 8));
                }
                if !st.r_scratch_hi.is_empty() {
                    let _ = writeln!(s, "    READS beyond the compared scratch: {}", top(&st.r_scratch_hi, 8));
                }
                if !st.w_args.is_empty() {
                    let _ = writeln!(s, "    writes to the argument area (esp offset:count): {}", top(&st.w_args, 8));
                }
                if !st.w_other.is_empty() {
                    let _ = writeln!(s, "    OTHER writes: {}", top(&st.w_other, 8));
                }
                if !st.r_uninit_stack.is_empty() {
                    let _ = writeln!(s, "    reads of uninitialised stack (esp offset:count): {}", top(&st.r_uninit_stack, 8));
                }
                let _ = writeln!(
                    s,
                    "    regs: EDX changed in {}/{}, ECX changed in {}/{}{}",
                    a.edx_changed,
                    a.calls - a.errors,
                    a.ecx_changed,
                    a.calls - a.errors,
                    if spec.ret == RetKind::Al { format!(", AL fn with EAX>>8 != 0 in {}", a.eax_hi_nonzero_al) } else { String::new() }
                );
                results.lock().unwrap().push((spec.addr, s));
            });
        }
    });
    let mut r = results.into_inner().unwrap();
    r.sort();
    for (_, s) in r {
        print!("{s}");
    }
}

// ================================================================== 3. mutants ====================

/// A spec whose port is replaced by `m(input, scratch-before, expected-by-the-real-port)`.
fn mutate(
    spec: &Spec,
    m: impl Fn(&Inputs, &[u8; SCRATCH_LEN], Expect) -> Expect + Send + Sync + 'static,
) -> Spec {
    let mut s = spec.clone();
    let real = spec.rust.clone();
    s.rust = Arc::new(move |i, sc| m(i, sc, real(i, sc)));
    s
}

fn spec(key: &str) -> Spec {
    specs::by_key(key).unwrap_or_else(|| panic!("no spec {key}"))
}

fn with(e: Expect, f: impl FnOnce(&mut ExpRet, &mut [u8; SCRATCH_LEN])) -> Expect {
    match e {
        Expect::Returns { mut ret, mut scratch } => {
            f(&mut ret, &mut scratch);
            Expect::Returns { ret, scratch }
        }
        h => h,
    }
}

/// Generic "missing store": the last scratch byte the port changed keeps its initial value.
fn drop_last_store(i: &Inputs, before: &[u8; SCRATCH_LEN], e: Expect) -> Expect {
    let _ = i;
    with(e, |_, s| {
        if let Some(k) = (0..SCRATCH_LEN).rev().find(|&k| s[k] != before[k]) {
            s[k] = before[k];
        }
    })
}

/// Generic "stray store": one byte past the last byte the port changed is also written (0).
fn stray_store(_i: &Inputs, before: &[u8; SCRATCH_LEN], e: Expect) -> Expect {
    with(e, |_, s| {
        if let Some(k) = (0..SCRATCH_LEN).rev().find(|&k| s[k] != before[k])
            && k + 1 < SCRATCH_LEN
        {
            s[k + 1] ^= 0xFF;
        }
    })
}

fn u32_at(s: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(s[o..o + 4].try_into().unwrap())
}

#[test]
fn verifier_mutants_core_stream() {
    let Some(img) = img() else { return };
    let n = env_u64("VERIFY_MUTANT_CASES", 100_000);
    type M = Box<dyn Fn(&Inputs, &[u8; SCRATCH_LEN], Expect) -> Expect + Send + Sync>;
    let mut cases: Vec<(&str, &str, M, bool)> = vec![];
    // (spec key, mutant, function, expected to be caught)
    // --- generic store mutants on every claimed spec that writes memory are run below separately.
    // --- semantic mutants (plausible porting errors)
    // GTATranslateKey: gta-reversed's `<= 0xFF` for the MapVirtualKey default (char 0xFF stored)
    cases.push((
        "GTATranslateKey",
        "default accepts chr == 0xFF (gta-reversed)",
        // inputs: [lparam, vk, os version, MapVirtualKeyA result, out word]; out word at scratch 0x30
        Box::new(|i, _b, e| {
            let (lp, vk, os, chr) = (i[0], i[1], i[2] as i32, i[3]);
            if sa_core::input::translate_key_case(vk, lp, os).is_none() && chr & 0xFFFF == 0xFF {
                with(e, |ret, s| {
                    s[0x30..0x34].copy_from_slice(&0xFFu32.to_le_bytes());
                    *ret = ExpRet::Al(1);
                })
            } else {
                e
            }
        }),
        // V-11 (fixed in VER-2): the generator biases MapVirtualKeyA's char to 0xFE/0xFF/0x100
        true,
    ));
    // SetBaseModelInfoFlags: EDX is part of the caller-visible state (0x5B4179); since VER-2 the
    // spec compares it (`Spec::preserve`), see verifier_edx_live_across_set_base_model_info_flags.
    // CPedModelInfo::IncrementVoice: wrap test `>=` instead of `>`
    cases.push((
        "CPedModelInfoIncrementVoice",
        "wrap when cur >= max",
        Box::new(|_i, b, e| {
            with(e, |_, s| {
                let o = 0x10;
                let rd = |x: &[u8], k: usize| i16::from_le_bytes([x[o + k], x[o + k + 1]]);
                let (min, max, cur) = (rd(b, 0x3E), rd(b, 0x40), rd(b, 0x42));
                if min >= 0 && max >= 0 {
                    let v = cur.wrapping_add(1);
                    let w = if v >= max || v < min { min } else { v };
                    s[o + 0x42..o + 0x44].copy_from_slice(&w.to_le_bytes());
                }
            })
        }),
        true,
    ));
    // CLinkedUpgradeList::FindOtherUpgrade: search from the first link up (gta-reversed order)
    cases.push((
        "LinkedFindOtherUpgrade",
        "search upwards",
        Box::new(|i, b, e| {
            let o = 0x10;
            let n = u32_at(b, o + 0x78);
            let up = i[0] as u16 as i16;
            let rd = |k: usize| i16::from_le_bytes([b[o + k], b[o + k + 1]]);
            let mut r = 0xFFFFu32;
            for k in 0..n as usize {
                if rd(2 * k) == up {
                    r = rd(2 * k + 0x3C) as u16 as u32;
                    break;
                }
                if rd(2 * k + 0x3C) == up {
                    r = rd(2 * k) as u16 as u32;
                    break;
                }
            }
            with(e, |ret, _| *ret = ExpRet::Eax(r))
        }),
        true,
    ));
    // CVehicleModelInfo::GetNumRemaps: forgets the cap of 4
    cases.push((
        "VehGetNumRemaps",
        "no cap at 4",
        Box::new(|_i, b, e| {
            let o = 0x10 + 0x2FA;
            let mut n = 0;
            while n < 5 && i16::from_le_bytes([b[o + 2 * n], b[o + 2 * n + 1]]) != -1 {
                n += 1;
            }
            with(e, |ret, _| *ret = ExpRet::Eax(n as u32))
        }),
        true,
    ));
    // CReferences::ListSize: counts the head twice for one-element lists
    cases.push((
        "CReferencesListSize",
        "off by one for a single node",
        Box::new(|_i, _b, e| {
            with(e, |ret, _| {
                if let ExpRet::Eax(1) = ret {
                    *ret = ExpRet::Eax(2)
                }
            })
        }),
        true,
    ));
    // RsKeyFromScanCode: index with the full word instead of the low byte
    cases.push((
        "RsKeyFromScanCode",
        "table index not masked to a byte",
        Box::new(|i, _b, e| {
            let scan = i[0];
            if scan > 0xFF {
                with(e, |ret, _| *ret = ExpRet::Eax(0xEE))
            } else {
                e
            }
        }),
        true,
    ));
    // RsKeyFromScanCode: AL right, upper EAX not zero-extended (V-14: EAX is compared whole)
    cases.push((
        "RsKeyFromScanCode",
        "upper EAX bits left set",
        Box::new(|_i, _b, e| {
            with(e, |ret, _| {
                if let ExpRet::Eax(v) = ret {
                    *ret = ExpRet::Eax(*v | 0xA500)
                }
            })
        }),
        true,
    ));
    // GetCdPosnAndSize: returns true for size 0
    cases.push((
        "CStreamingInfoGetCdPosnAndSize",
        "true for size 0",
        Box::new(|_i, _b, e| with(e, |ret, _| *ret = ExpRet::Al(1))),
        true,
    ));
    // DoesInfiniteLineTouchScreen: width/height swapped
    cases.push((
        "DoesInfiniteLineTouchScreen1366",
        "screen width and height swapped",
        Box::new(|i, _b, e| {
            let f = |k: usize| f32::from_bits(i[k]);
            let v = sa_core::common::infinite_line_touches_screen(768, 1366, f(0), f(1), f(2), f(3));
            with(e, |ret, _| *ret = ExpRet::Al(v as u8))
        }),
        true,
    ));
    // GetDayNightBalance: gta-reversed's `/ 60` instead of `* (1/60)` on the dawn ramp
    cases.push((
        "GetDayNightBalance",
        "divides by 60 (gta-reversed)",
        Box::new(|_i, b, e| {
            let w = u32_at(b, 0x10);
            let (h, m, sec) = ((w >> 24) as u8, (w >> 16) as u8, w as u16);
            let t = (h as i32 * 60 + m as i32) as f32 + sec as f32 / 60.0;
            let v = if t < 360.0 {
                1.0
            } else if t < 420.0 {
                (420.0 - t) / 60.0
            } else if t < 1200.0 {
                0.0
            } else if t < 1260.0 {
                1.0 - (1260.0 - t) / 60.0
            } else {
                1.0
            };
            let x = sa_math::x87::X87::from_f32(v);
            with(e, |ret, _| {
                *ret = ExpRet::St0 {
                    f32: v.to_bits(),
                    ext: x.to_ext(),
                }
            })
        }),
        true,
    ));
    let mut ok = true;
    for (key, name, m, expect_caught) in cases {
        if !only(key) {
            continue;
        }
        let base = spec(key);
        let s = mutate(&base, m);
        let rep = runner::run(&s, &img, &cfg(n)).unwrap();
        let caught = rep.mismatches > 0;
        println!(
            "  {key:<36} {name:<44} mismatches={:<8} {:?} -> {}",
            rep.mismatches,
            rep.kinds,
            if caught { "CAUGHT" } else { "missed" }
        );
        if expect_caught {
            ok &= caught;
        }
    }
    assert!(ok, "a mutant expected to be caught was missed");
}

/// Generic store mutants over every claimed spec that writes scratch memory: (a) the last byte the
/// port writes is not written, (b) one byte past it is also written. Both must be caught.
#[test]
fn verifier_store_mutants_all_claimed() {
    let Some(img) = img() else { return };
    let n = env_u64("VERIFY_MUTANT_CASES", 20_000);
    let mut missed = vec![];
    let mut uncaught_writers = vec![];
    for base in claimed_specs().into_iter().filter(|s| only(s.key)) {
        // does the port write scratch memory at all (on a sample)?
        let writes = (0..2000u64).any(|k| {
            let i = (base.generate)(&mut Rng::for_case(0x51, k));
            let c = (base.call)(&i);
            matches!((base.rust)(&i, &c.scratch), Expect::Returns { scratch, .. } if scratch != c.scratch)
        });
        for (label, m) in [
            (
                "missing store",
                drop_last_store as fn(&Inputs, &[u8; SCRATCH_LEN], Expect) -> Expect,
            ),
            ("stray store", stray_store),
        ] {
            let s = mutate(&base, m);
            let rep = runner::run(&s, &img, &cfg(n)).unwrap();
            // a spec whose port never writes memory makes both mutants equivalent
            println!("  {:<44} {label:<14} mismatches={}", base.key, rep.mismatches);
            if rep.mismatches == 0 {
                missed.push((base.key, label));
                if writes {
                    uncaught_writers.push((base.key, label));
                }
            }
        }
    }
    println!("store mutants with 0 mismatches (equivalent: the port writes no memory): {missed:?}");
    println!("store mutants missed although the port writes memory: {uncaught_writers:?}");
    assert!(uncaught_writers.is_empty());
}

// =========================================== 4. specs that did not run the hooked entry (V-10) =====

/// V-10: the 44 helper/model specs now run the hooked entries on the host (`sa_oracle::hosted`):
/// host globals filled from the scratch regions, stand-ins for unported callees, and the 32-bit
/// `CPool` header bridged to the host layout. Re-run of the six specs this probe used to rewire
/// itself (10^6 cases by default), plus discriminating faults that only the entries can show.
#[test]
fn verifier_hooked_entries_through_host_globals() {
    let Some(img) = img() else { return };
    let n = env_u64("VERIFY_HOOKED_CASES", 1_000_000);
    let mut ok = true;
    for key in [
        "CPadIsEnterJustPressed",
        "CPadIsMenuKeyJustPressed",
        "CPadIsTabJustPressed",
        "CPadIsStandardKeyJustPressed",
        "GetDayNightBalance",
        "InitGBab358",
    ] {
        let s = spec(key);
        let cases = if key == "InitGBab358" { n.min(10_000) } else { n };
        let rep = runner::run(&s, &img, &cfg(cases)).unwrap();
        println!(
            "  {key:<34} hooked entry: mismatches={} {:?}",
            rep.mismatches, rep.kinds
        );
        ok &= rep.mismatches == 0;
    }
    assert!(ok);
}

/// The hosted pool specs see what the entries do: a slot type of the wrong size, the wrong
/// allocator, and a delete through the wrong pool global are all caught.
#[test]
fn verifier_hosted_pool_entries_discriminate() {
    use sa_core::mem::GPtr;
    use sa_core::pool::{self, CPool};
    use sa_core::pools::{self as ps, Slot};
    let Some(img) = img() else { return };
    let n = env_u64("VERIFY_MUTANT_CASES", 20_000);
    // CColModelPool::New (slot 0x30) run with a 0x2C slot
    let mut s = spec("CColModelPoolNew");
    s.rust = Arc::new(|_i, sc| {
        let (out, eax) = sa_oracle::hosted::run_pool::<Slot<0x2C>>(sc, &|p| pool::pool_new(p).addr());
        Expect::Returns {
            ret: ExpRet::Eax(eax),
            scratch: out,
        }
    });
    let r = runner::run(&s, &img, &cfg(n)).unwrap();
    println!(
        "  CColModelPoolNew with slot 0x2C: mismatches={} {:?}",
        r.mismatches, r.kinds
    );
    assert!(r.mismatches > 0);
    // CPool<CPtrNodeSingleLink> ctor (new[] 0x821195) checked against `operator new` 0x82119A
    let mut s = spec("CPoolCtor550180");
    let real = s.rust.clone();
    s.rust = Arc::new(move |i, sc| {
        let e = real(i, sc);
        sa_oracle::hosted::expect_allocator(sa_oracle::hosted::NEW);
        let r = std::panic::catch_unwind(|| {
            sa_oracle::hosted::run_pool::<Slot<0x08>>(sc, &|p| {
                ps::ctor_ptr_node_single(p, i[0] as i32, GPtr::NULL).addr()
            })
        });
        assert!(r.is_err(), "allocator mismatch not detected");
        e
    });
    let r = runner::run(&s, &img, &cfg(200)).unwrap();
    assert_eq!(r.mismatches, 0);
    // CEntryInfoNode::operator delete through the node-route pool global (slot 0x24 instead of 0x14)
    let mut s = spec("CEntryInfoNodeOperatorDelete");
    s.rust = Arc::new(|i, sc| {
        let obj = *i.last().unwrap();
        let _g = sa_oracle::host_globals();
        let (out, _) = sa_oracle::hosted::run_pool::<Slot<0x24>>(sc, &|p: GPtr<CPool<Slot<0x24>>>| {
            ps::NODE_ROUTE_POOL.set(p);
            ps::node_route_operator_delete(GPtr::from_addr(obj as usize));
            0
        });
        Expect::Returns {
            ret: ExpRet::Void,
            scratch: out,
        }
    });
    let r = runner::run(&s, &img, &cfg(n)).unwrap();
    println!(
        "  CEntryInfoNodeOperatorDelete via CNodeRoute's entry: mismatches={} {:?}",
        r.mismatches, r.kinds
    );
    assert!(r.mismatches > 0);
}

// ============================================== 5. targeted generator-gap probes ===================

/// `GTATranslateKey`'s default case stores `MapVirtualKeyA`'s char only when `< 0xFF` (gta-reversed:
/// `<= 0xFF`, a recorded deviation). The spec's generator produces chr == 0xFF about twice per 10^6
/// cases. With the MapVirtualKeyA result forced to 0xFF (upper half random) the gta-reversed mutant is
/// caught on every default-path case, and the real port still passes.
#[test]
fn verifier_gta_translate_key_boundary() {
    let Some(img) = img() else { return };
    let n = env_u64("VERIFY_MUTANT_CASES", 100_000);
    let base = spec("GTATranslateKey");
    let gen0 = base.generate.clone();
    let mut forced = base.clone();
    forced.generate = Arc::new(move |r| {
        let mut v = gen0(r);
        v[3] = (r.next_u32() & 0xFFFF_0000) | 0xFF;
        v
    });
    let real = runner::run(&forced, &img, &cfg(n)).unwrap();
    let mutant = mutate(&forced, |i, _b, e| {
        let (lp, vk, os, chr) = (i[0], i[1], i[2] as i32, i[3]);
        if sa_core::input::translate_key_case(vk, lp, os).is_none() && chr & 0xFFFF == 0xFF {
            with(e, |ret, s| {
                s[0x30..0x34].copy_from_slice(&0xFFu32.to_le_bytes());
                *ret = ExpRet::Al(1);
            })
        } else {
            e
        }
    });
    let m = runner::run(&mutant, &img, &cfg(n)).unwrap();
    println!(
        "  forced chr=0xFF: real port mismatches={}  gta-reversed mutant mismatches={} {:?}",
        real.mismatches, m.mismatches, m.kinds
    );
    assert_eq!(real.mismatches, 0);
    assert!(m.mismatches > 0);
}

// ====================================== 6. EDX is live across SetBaseModelInfoFlags (0x5B4179) =====

/// The caller at 0x5B4177 (`mov edx, eax; call 0x5B3AD0; add esp, 8; test dl, 0x20`) relies on the
/// callee leaving EDX unchanged (whole-program optimisation: 0x5B3AD0 never writes EDX). Since
/// VER-2 the spec declares EDX preserved: the runner loads EDX with a per-case value, requires the
/// original to return it and the port's `#[hook]` to declare `preserve = "edx"` (its entry is then
/// an EDX-saving wrapper). Both halves of that comparison are shown to discriminate.
#[test]
fn verifier_edx_live_across_set_base_model_info_flags() {
    let Some(img) = img() else { return };
    let site = img.bytes(0x5B4177, 15);
    assert_eq!(
        site,
        &[
            0x8B, 0xD0, 0xE8, 0x52, 0xF9, 0xFF, 0xFF, 0x83, 0xC4, 0x08, 0xF6, 0xC2, 0x20, 0x74, 0x06
        ]
    );
    let s = spec("CBaseModelInfoSetBaseModelInfoFlags");
    assert_eq!(s.preserve, sa_core::abi::PRESERVE_EDX);
    assert_eq!(
        sa_oracle::hook_preserve(0x5B3AD0),
        Some(sa_core::abi::PRESERVE_EDX)
    );
    let r = runner::run(&s, &img, &cfg(100_000)).unwrap();
    println!(
        "  0x5B3AD0 with EDX compared: mismatches={} {:?}",
        r.mismatches, r.kinds
    );
    assert_eq!(r.mismatches, 0);
    // a port without `preserve`: RsKeyFromScanCode's original never writes EDX either
    let mut k = spec("RsKeyFromScanCode");
    k.preserve = sa_core::abi::PRESERVE_EDX;
    let r = runner::run(&k, &img, &cfg(10_000)).unwrap();
    println!(
        "  RsKeyFromScanCode declared EDX-preserving, hook without preserve: mismatches={} {:?}",
        r.mismatches, r.kinds
    );
    assert_eq!(r.mismatches, r.total());
    // an original that writes EDX: SetAtomicModelInfoFlags (`mov dx, ...` after the call)
    let mut a = spec("CAtomicModelInfoSetAtomicModelInfoFlags");
    a.preserve = sa_core::abi::PRESERVE_EDX;
    let r = runner::run(&a, &img, &cfg(10_000)).unwrap();
    println!(
        "  SetAtomicModelInfoFlags declared EDX-preserving: mismatches={} {:?}",
        r.mismatches, r.kinds
    );
    assert!(r.mismatches > 0);
    assert!(r.examples.iter().all(|m| m.detail.contains("on return")));
}
