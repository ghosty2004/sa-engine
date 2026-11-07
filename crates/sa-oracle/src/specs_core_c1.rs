//! L1 specs for PORT-CORE fork C1 (`sa_core::platform`, `sa_core::video`): the pure leaves of the
//! platform layer. `RsKeyFromScanCode` reads its two 256-byte key tables from the image's `.data`
//! (allowed reads; the Rust port reads its owned copies, whose initial values a sa-core test checks
//! against the exe); the others return constants. See `docs/arch/core-port.md`.

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::platform;

fn g_scan(r: &mut Rng) -> u32 {
    match r.below(4) {
        0 => r.next_u32(),
        _ => r.below(0x100),
    }
}

fn g_flag(r: &mut Rng) -> u32 {
    match r.below(4) {
        0 => r.next_u32(),
        1 => 1,
        _ => 0,
    }
}

/// A function returning a constant in EAX (no inputs that matter).
fn constant(key: &'static str, name: &'static str, addr: u32, value: u32) -> Spec {
    b(key, name, addr, Cdecl, Eax)
        .args(vec![A::I(|r| r.next_u32())])
        .rust(move |_| ExpRet::Eax(value))
}

/// Every C1 spec.
pub fn all() -> Vec<Spec> {
    vec![
        // EAX compared whole: the original zero-extends (`movzx eax, byte [esp+4]; mov al, [eax+tbl]`)
        b("RsKeyFromScanCode", "Rs::RsKeyFromScanCode", 0x6193F0, Cdecl, Eax)
            .args(vec![A::I(g_scan), A::I(g_flag)])
            .reads(vec![(0x8D2C00, 0x8D2E00)])
            .rust(|c| ExpRet::Eax(platform::rs_key_from_scan_code(c.i(0), c.i(1) as i32))),
        constant("RsRegisterImageLoader", "Rs::RsRegisterImageLoader", 0x619470, platform::rs_register_image_loader() as u32),
        constant("RsSetDebug", "Rs::RsSetDebug", 0x619480, platform::rs_set_debug() as u32),
        constant("MotionBlurClose", "CMBlur::MotionBlurClose", 0x71D720, platform::motion_blur_close() as u32),
        constant("psInstallFileSystem", "Ps::psInstallFileSystem", 0x745520, platform::ps_install_file_system() as u32),
        b("psPathGetSeparator", "Ps::psPathGetSeparator", 0x745500, Cdecl, Al)
            .args(vec![A::I(|r| r.next_u32())])
            .rust(|_| ExpRet::Al(platform::ps_path_get_separator())),
    ]
}
