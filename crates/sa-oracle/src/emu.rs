//! L1 backend: original code under Unicorn (x86-32, QEMU 5.0.1 based, bundled build).
//!
//! Memory map (identical addresses in the native backend where they matter):
//!
//! | range | use |
//! |---|---|
//! | 0x10000-0x12000 | exit stub (`native/exit_stub.S`), abort address, result block, x87 env |
//! | 0x20000-        | import traps: IAT slot i points at 0x20000 + 16*i (int3 filled, never allowed) |
//! | 0x100000-0x200000 | stack |
//! | 0x400000-0x1577000 | gta_sa.exe image at its preferred base, IAT patched |
//! | 0x10000000 | scratch (per-call inputs/outputs) |
//! | 0x10100000 | probe code (fidelity) |
//!
//! Guard rails, per call: a block hook allows only blocks inside the function's extent, the extents of
//! its allow-listed callees and the exit stub (anything else — a call into the game, the CRT, an import
//! trap, a bad return — stops the call with [`CallError::Disallowed`] naming the target); an instruction
//! budget ([`CallError::Budget`]); writes outside stack/scratch and reads of writable image sections stop
//! the call; interrupts and faults stop the call.
//!
//! Known Unicorn (QEMU 5.0.1) x87 defects and what this backend does about them (measured, see
//! `docs/re/l1-harness.md`):
//! - MXCSR.FZ is applied to the *x87* status (`fpu_helper.c` `update_mxcsr_status`), so with the game's
//!   MXCSR (FZ on) every x87 store of an f32/f64-subnormal value is flushed to zero. The real CPU does not
//!   do that. With [`Emu::ftz_workaround`] (default on) the emulated MXCSR has FZ cleared; this is exact
//!   for code without SSE instructions, which the runner checks with `scan` before enabling it. The
//!   reported MXCSR gets the FZ bit back.
//! - `fld` of a signalling NaN does not quiet it; arithmetic C1 and exception flags are not set; the
//!   transcendentals use host doubles. Those are handled by routing (`specs::Route`, runner input classes).

use crate::call::*;
use crate::catalog::Catalog;
use crate::pe::Image;
use std::sync::Arc;
use unicorn_engine::{Arch, HookType, MemType, Mode, Prot, RegisterX86 as R, Unicorn, X86CpuModel};

pub const STUB: u32 = 0x10000;
/// Exit address: end of the exit stub.
pub const STUB_END: u32 = 0x1005C;
/// Where a guard-rail violation sends EIP (an exit).
pub const ABORT: u32 = 0x10080;
pub const RESULT: u32 = 0x11000;
pub const ENV: u32 = 0x11100;
pub const STACK_LO: u32 = 0x10_0000;
pub const STACK_HI: u32 = 0x20_0000;
/// ESP before the arguments are pushed.
pub const ESP0: u32 = 0x1F_0000;

/// Assembled from `native/exit_stub.S` (`as --32`, `objcopy -O binary`); `exit_stub_matches_source`
/// re-assembles it when `as` is available.
pub const EXIT_STUB: [u8; 93] = [
    0xa3, 0x58, 0x10, 0x01, 0x00, 0x89, 0x15, 0x5c, 0x10, 0x01, 0x00, 0xdd, 0x3d, 0x64, 0x10, 0x01, 0x00,
    0x0f, 0xae, 0x1d, 0x78, 0x10, 0x01, 0x00, 0xd9, 0x35, 0x00, 0x11, 0x01, 0x00, 0x66, 0xa1, 0x08, 0x11,
    0x01, 0x00, 0x66, 0xa3, 0x66, 0x10, 0x01, 0x00, 0xd9, 0x25, 0x00, 0x11, 0x01, 0x00, 0xc7, 0x05, 0x7c,
    0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0xd9, 0xe5, 0xdf, 0xe0, 0x80, 0xe4, 0x45, 0x80, 0xfc, 0x41,
    0x74, 0x16, 0xd9, 0x15, 0x74, 0x10, 0x01, 0x00, 0xdb, 0x3d, 0x68, 0x10, 0x01, 0x00, 0xc7, 0x05, 0x7c,
    0x10, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x90,
];

/// Allowed code for one call: the function and its allow-listed callees, by extent.
#[derive(Clone, Debug, Default)]
pub struct Allow {
    pub ranges: Vec<(u32, u32)>,
}

impl Allow {
    /// The function at `addr` plus `callees`, extents from the catalogue.
    pub fn function(cat: &Catalog, addr: u32, callees: &[u32]) -> Result<Allow, String> {
        let mut ranges = Vec::new();
        for &a in std::iter::once(&addr).chain(callees) {
            ranges.push(
                cat.extent(a)
                    .ok_or(format!("{a:#x} is not a catalogued function start"))?,
            );
        }
        Ok(Allow { ranges })
    }

    /// An explicit range (probes).
    pub fn range(lo: u32, hi: u32) -> Allow {
        Allow {
            ranges: vec![(lo, hi)],
        }
    }

    fn contains(&self, a: u32) -> bool {
        (STUB..STUB + 0x100).contains(&a) || self.ranges.iter().any(|&(lo, hi)| a >= lo && a < hi)
    }
}

#[derive(Default)]
struct HookState {
    allow: Allow,
    last_block: u32,
    error: Option<CallError>,
    /// image addresses written (restored after the call)
    dirty: Vec<(u32, u32)>,
    image: (u32, u32),
    /// ST0 read through the register API when control reached the exit stub
    st0_reg: Option<[u8; 10]>,
}

/// One Unicorn instance with the image mapped. Not `Send`: create one per thread.
pub struct Emu {
    uc: Unicorn<'static, HookState>,
    img: Arc<Image>,
    pub budget: u64,
    /// Clear MXCSR.FZ in the emulator (see the module docs). Only valid for code without SSE.
    pub ftz_workaround: bool,
}

fn uerr(e: unicorn_engine::uc_error) -> String {
    format!("unicorn: {e:?}")
}

impl Emu {
    pub fn new(img: Arc<Image>) -> Result<Emu, String> {
        let mut uc = Unicorn::new_with_data(Arch::X86, Mode::MODE_32, HookState::default()).map_err(uerr)?;
        // A CPU with SSE/SSE2 (MXCSR); the x87 unit is the same QEMU code for every model.
        uc.ctl_set_cpu_model(X86CpuModel::PENTIUM3 as i32).map_err(uerr)?;
        // As under Windows: SSE enabled (CR4.OSFXSR | OSXMMEXCPT), x87 present (CR0.EM = 0, MP = 1,
        // NE = 1).
        let cr4 = uc.reg_read(R::CR4).map_err(uerr)?;
        uc.reg_write(R::CR4, cr4 | 0x600).map_err(uerr)?;
        let cr0 = uc.reg_read(R::CR0).map_err(uerr)?;
        uc.reg_write(R::CR0, (cr0 & !0x4) | 0x22).map_err(uerr)?;
        let page = |x: u32| (x + 0xFFF) & !0xFFF;
        uc.mem_map(STUB as u64, 0x2000, Prot::ALL).map_err(uerr)?;
        uc.mem_write(STUB as u64, &EXIT_STUB).map_err(uerr)?;
        uc.mem_write(ABORT as u64, &[0xF4]).map_err(uerr)?; // hlt (never executed: ABORT is an exit)
        let trap_len = page(img.trap_end() - crate::pe::TRAP_BASE).max(0x1000);
        uc.mem_map(crate::pe::TRAP_BASE as u64, trap_len as u64, Prot::ALL)
            .map_err(uerr)?;
        uc.mem_write(crate::pe::TRAP_BASE as u64, &vec![0xCC; trap_len as usize])
            .map_err(uerr)?;
        uc.mem_map(
            STACK_LO as u64,
            (STACK_HI - STACK_LO) as u64,
            Prot::READ | Prot::WRITE,
        )
        .map_err(uerr)?;
        uc.mem_map(img.base as u64, page(img.flat.len() as u32) as u64, Prot::ALL)
            .map_err(uerr)?;
        uc.mem_write(img.base as u64, &img.flat).map_err(uerr)?;
        uc.mem_map(SCRATCH as u64, 0x1_0000, Prot::READ | Prot::WRITE)
            .map_err(uerr)?;
        uc.mem_map(PROBE as u64, PROBE_LEN as u64, Prot::ALL)
            .map_err(uerr)?;
        uc.ctl_exits_enable().map_err(uerr)?;
        uc.ctl_set_exits(&[STUB_END as u64, ABORT as u64]).map_err(uerr)?;
        uc.get_data_mut().image = (img.base, img.end());

        uc.add_block_hook(1, 0, |uc, addr, _size| {
            let a = addr as u32;
            let d = uc.get_data_mut();
            if d.error.is_some() {
                return;
            }
            if d.allow.contains(a) {
                d.last_block = a;
                return;
            }
            d.error = Some(CallError::Disallowed {
                target: a,
                from: d.last_block,
                what: String::new(),
            });
            let _ = uc.reg_write(R::EIP, ABORT as u64);
            let _ = uc.emu_stop();
        })
        .map_err(uerr)?;
        uc.add_code_hook(STUB as u64, STUB as u64, |uc, _, _| {
            let st0 = uc
                .reg_read_long(R::ST0)
                .ok()
                .and_then(|b| <[u8; 10]>::try_from(&b[..]).ok());
            uc.get_data_mut().st0_reg = st0;
        })
        .map_err(uerr)?;
        uc.add_mem_hook(HookType::MEM_WRITE, 1, 0, |uc, _ty, addr, size, _v| {
            let a = addr as u32;
            if (STACK_LO..STACK_HI).contains(&a)
                || (STUB..STUB + 0x2000).contains(&a)
                || (SCRATCH..SCRATCH + 0x1_0000).contains(&a)
            {
                return true;
            }
            let pc = uc.reg_read(R::EIP).unwrap_or(0) as u32;
            let d = uc.get_data_mut();
            if a >= d.image.0 && a < d.image.1 {
                d.dirty.push((a, size as u32));
            }
            if d.error.is_none() {
                d.error = Some(CallError::GlobalWrite {
                    addr: a,
                    size: size as u32,
                    pc,
                });
            }
            let _ = uc.reg_write(R::EIP, ABORT as u64);
            let _ = uc.emu_stop();
            true
        })
        .map_err(uerr)?;
        for s in img.sections.iter().filter(|s| s.writable) {
            uc.add_mem_hook(
                HookType::MEM_READ,
                s.va as u64,
                (s.va + s.vsize - 1) as u64,
                |uc, _ty, addr, size, _v| {
                    let pc = uc.reg_read(R::EIP).unwrap_or(0) as u32;
                    let d = uc.get_data_mut();
                    if d.error.is_none() {
                        d.error = Some(CallError::GlobalRead {
                            addr: addr as u32,
                            size: size as u32,
                            pc,
                        });
                    }
                    let _ = uc.emu_stop();
                    true
                },
            )
            .map_err(uerr)?;
        }
        uc.add_mem_hook(HookType::MEM_INVALID, 1, 0, |uc, ty, addr, _size, _v| {
            let pc = uc.reg_read(R::EIP).unwrap_or(0) as u32;
            let d = uc.get_data_mut();
            if d.error.is_none() {
                d.error = Some(CallError::Fault {
                    kind: mem_type_name(ty).into(),
                    addr: addr as u32,
                    pc,
                });
            }
            false
        })
        .map_err(uerr)?;
        uc.add_intr_hook(|uc, n| {
            let pc = uc.reg_read(R::EIP).unwrap_or(0) as u32;
            let d = uc.get_data_mut();
            if d.error.is_none() {
                d.error = Some(CallError::Interrupt { number: n, pc });
            }
            let _ = uc.emu_stop();
        })
        .map_err(uerr)?;
        Ok(Emu {
            uc,
            img,
            budget: 1_000_000,
            ftz_workaround: true,
        })
    }

    pub fn image(&self) -> &Image {
        &self.img
    }

    /// Loads a code blob at [`PROBE`].
    pub fn load_probes(&mut self, code: &[u8]) -> Result<(), String> {
        self.uc.mem_write(PROBE as u64, code).map_err(uerr)
    }

    /// Reads emulator memory.
    pub fn read_mem(&self, addr: u32, n: usize) -> Result<Vec<u8>, String> {
        self.uc.mem_read_as_vec(addr as u64, n).map_err(uerr)
    }

    /// ST0 as read through the register API at the moment the function returned (cross-check of the
    /// exit stub's `fstp tbyte`; `None` when the x87 stack was empty).
    pub fn last_st0_reg(&self) -> Option<[u8; 10]> {
        self.uc.get_data().st0_reg
    }

    /// Runs one call. `allow` is the code the call may execute.
    pub fn call(&mut self, call: &Call, allow: &Allow) -> CallResult {
        let r = self.call_inner(call, allow);
        // Restore any image bytes the call wrote.
        let dirty = std::mem::take(&mut self.uc.get_data_mut().dirty);
        for (a, n) in dirty {
            let o = (a - self.img.base) as usize;
            let end = (o + n as usize).min(self.img.flat.len());
            let bytes = self.img.flat[o..end].to_vec();
            self.uc
                .mem_write(a as u64, &bytes)
                .map_err(|e| CallError::Backend(uerr(e)))?;
        }
        r
    }

    fn call_inner(&mut self, call: &Call, allow: &Allow) -> CallResult {
        let be = |e| CallError::Backend(uerr(e));
        {
            let d = self.uc.get_data_mut();
            d.allow = allow.clone();
            d.last_block = 0;
            d.error = None;
            d.st0_reg = None;
        }
        self.uc.mem_write(SCRATCH as u64, &call.scratch).map_err(be)?;
        let mut ctx = call.ctx();
        ctx[88..].fill(0);
        self.uc.mem_write(RESULT as u64, &ctx).map_err(be)?;
        // Stack: return address, then the args.
        let esp = ESP0 - 4 * call.stack.len() as u32 - 4;
        let mut st = Vec::with_capacity(4 + 4 * call.stack.len());
        st.extend_from_slice(&STUB.to_le_bytes());
        for v in &call.stack {
            st.extend_from_slice(&v.to_le_bytes());
        }
        self.uc.mem_write(esp as u64, &st).map_err(be)?;
        // Same register picture as native/thunk.S at the call: eax = target, ecx/edx = inputs.
        for (r, v) in [
            (R::EAX, call.addr),
            (R::ECX, call.ecx),
            (R::EDX, call.edx),
            (R::EBX, 0),
            (R::ESI, 0),
            (R::EDI, 0),
            (R::EBP, 0),
            (R::ESP, esp),
            (R::EFLAGS, 0x202),
        ] {
            self.uc.reg_write(r, v as u64).map_err(be)?;
        }
        self.uc.reg_write(R::FPCW, call.fpu.cw as u64).map_err(be)?;
        self.uc.reg_write(R::FPSW, 0).map_err(be)?;
        self.uc.reg_write(R::FPTAG, 0xFFFF).map_err(be)?;
        let mxcsr = if self.ftz_workaround {
            call.fpu.mxcsr & !0x8000
        } else {
            call.fpu.mxcsr
        };
        self.uc.reg_write(R::MXCSR, mxcsr as u64).map_err(be)?;

        let r = self.uc.emu_start(call.addr as u64, 0, 0, self.budget as usize);
        let pc = self.uc.reg_read(R::EIP).map_err(be)? as u32;
        if let Some(mut e) = self.uc.get_data_mut().error.take() {
            if let CallError::Disallowed { target, what, .. } = &mut e {
                *what = self.describe(*target);
            }
            return Err(e);
        }
        if let Err(e) = r {
            return Err(CallError::Backend(format!("{} at pc {pc:#x}", uerr(e))));
        }
        if pc != STUB_END {
            return Err(CallError::Budget { budget: self.budget });
        }
        let mut b = [0u8; CTX_LEN];
        self.uc.mem_read(RESULT as u64, &mut b).map_err(be)?;
        let esp_now = self.uc.reg_read(R::ESP).map_err(be)? as u32;
        b[96..100].copy_from_slice(&(esp_now.wrapping_sub(ESP0)).to_le_bytes());
        let mut s = [0u8; SCRATCH_LEN];
        self.uc.mem_read(SCRATCH as u64, &mut s).map_err(be)?;
        let mut o = Outcome::from_ctx(&b, &s);
        if self.ftz_workaround {
            o.mxcsr |= call.fpu.mxcsr & 0x8000;
        }
        Ok(o)
    }

    fn describe(&self, a: u32) -> String {
        if let Some(i) = self.img.import_at_trap(a) {
            return format!("import {} via IAT {:#x}", i.name, i.iat);
        }
        if a == ABORT {
            return "abort".into();
        }
        match crate::catalog_cached() {
            Some(c) => c.describe(a),
            None => format!("{a:#x}"),
        }
    }
}

/// Unicorn memory-type names for messages.
pub fn mem_type_name(t: MemType) -> &'static str {
    match t {
        MemType::READ_UNMAPPED => "read unmapped",
        MemType::WRITE_UNMAPPED => "write unmapped",
        MemType::FETCH_UNMAPPED => "fetch unmapped",
        _ => "protected",
    }
}
