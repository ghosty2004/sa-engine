//! The mod host in the game (feature `mods`, MOD-2, docs/arch/ADR-004-mods.md): the mods of `--mods DIR` (natively)
//! or the page's `?mods=a,b` (web) are loaded before the boot, and called at the game's fixed frame points. M0 has
//! one: the overlay phase, at every `IDirect3DDevice9::Present` after the game's own drawing ([`overlay`]); what the
//! mods draw goes to the render layer's 2D overlay (`WgpuDevice::draw_overlay`), outside the game's D3D9 state.
//! Nothing here reads or writes game memory or calls a port.

use std::cell::RefCell;

use sa_mod_host::Host;
use sa_rw_wgpu::d3d9::WgpuDevice;

thread_local! {
    /// The host (game thread only; `None` without mods).
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
    /// The overlay rectangles of the current frame (kept to reuse the allocation).
    static RECTS: RefCell<Vec<sa_platform::gpu::OverlayRect>> = const { RefCell::new(Vec::new()) };
}

/// Loads the mods of `spec` (a folder natively, a list of mod names on the web). A mod that cannot be loaded is
/// reported and skipped; an unreadable `spec` is an error.
pub fn install(spec: &str) -> Result<(), String> {
    let mut host = Host::new(Box::new(|s| println!("sa-game: {s}")));
    let sources = sa_mod_host::sources(spec)?;
    if sources.is_empty() {
        println!("sa-game: mods: none found in {spec}");
    }
    for s in sources {
        if let Err(e) = host.load(s.as_ref()) {
            println!("sa-game: mod not loaded: {e}");
        }
    }
    HOST.with_borrow_mut(|h| *h = Some(host));
    Ok(())
}

/// The overlay phase: runs every mod's overlay callback and draws the result over the back buffer (call before
/// the device's present).
pub fn overlay(d: &mut WgpuDevice) {
    HOST.with_borrow_mut(|h| {
        let Some(h) = h.as_mut() else { return };
        let _perf = crate::perf::scope(crate::perf::MODS);
        let size = d.back_buffer().size();
        RECTS.with_borrow_mut(|rects| {
            rects.clear();
            h.overlay((size.width, size.height), rects);
            d.draw_overlay(rects);
        });
    });
}

/// The mods' state and cost, for the end of a run.
pub fn summary() -> Option<String> {
    HOST.with_borrow(|h| h.as_ref().map(Host::summary))
}
