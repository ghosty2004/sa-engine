//! Static scan of a function's code (linear sweep over its catalogued extent and those of its
//! allow-listed callees) for instructions that decide how far Unicorn can be trusted:
//! - x87 instructions measured as unfaithful ([`crate::fidelity::UNFAITHFUL`]): route to L2;
//! - any SSE instruction: the MXCSR.FZ workaround (see `emu`) is then not applicable.
//!
//! A linear sweep may decode jump-table data as instructions; that can only add hazards (conservative).

use crate::catalog::Catalog;
use crate::pe::Image;
use iced_x86::{CpuidFeature, Decoder, DecoderOptions, Instruction};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hazards {
    /// unfaithful x87 mnemonics found, with the first address
    pub x87: Vec<(String, u32)>,
    /// first SSE instruction found
    pub sse: Option<(String, u32)>,
    pub instructions: usize,
}

pub fn scan(img: &Image, cat: &Catalog, addr: u32, callees: &[u32]) -> Result<Hazards, String> {
    let mut h = Hazards::default();
    for &a in std::iter::once(&addr).chain(callees) {
        let (lo, hi) = cat
            .extent(a)
            .ok_or(format!("{a:#x} is not a catalogued function start"))?;
        let bytes = img.bytes(lo, (hi - lo) as usize);
        let mut dec = Decoder::with_ip(32, bytes, lo as u64, DecoderOptions::NONE);
        let mut ins = Instruction::default();
        while dec.can_decode() {
            dec.decode_out(&mut ins);
            h.instructions += 1;
            let m = format!("{:?}", ins.mnemonic());
            let ip = ins.ip32();
            if crate::fidelity::UNFAITHFUL.contains(&m.as_str()) && !h.x87.iter().any(|(x, _)| *x == m) {
                h.x87.push((m.clone(), ip));
            }
            let sse = ins.cpuid_features().iter().any(|f| {
                matches!(
                    f,
                    CpuidFeature::SSE
                        | CpuidFeature::SSE2
                        | CpuidFeature::SSE3
                        | CpuidFeature::SSSE3
                        | CpuidFeature::SSE4_1
                        | CpuidFeature::SSE4_2
                        | CpuidFeature::SSE4A
                        | CpuidFeature::AVX
                        | CpuidFeature::AVX2
                )
            });
            if sse && h.sse.is_none() {
                h.sse = Some((m, ip));
            }
        }
    }
    Ok(h)
}
