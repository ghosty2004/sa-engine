//! A broken test mod (MOD-2 M0 acceptance): draws a red box with a countdown for 30 frames, then traps (a panic,
//! `unreachable` in wasm) or, with feature `spin`, loops forever. The host must disable it, report it and keep the
//! game (and the other mods) running.

use sa_mod::{Mod, Overlay, Rgba};

/// The overlay call that breaks.
const BREAK_AT: u32 = 30;

#[derive(Default)]
struct Broken;

impl Mod for Broken {
    fn overlay(&mut self, frame: u32, o: &mut Overlay) {
        if frame < BREAK_AT {
            let _ = o.rect(8.0, 8.0, 300.0, 28.0, Rgba::new(0xC0, 0, 0, 0xC0));
            let _ = o.text(14.0, 14.0, 16.0, Rgba::WHITE, &format!("breaking in {}", BREAK_AT - frame));
            return;
        }
        if cfg!(feature = "spin") {
            let mut i = 0u64;
            loop {
                i = std::hint::black_box(i.wrapping_add(1));
            }
        }
        panic!("broken mod: deliberate trap at frame {frame}");
    }
}

sa_mod::entry!(Broken);
