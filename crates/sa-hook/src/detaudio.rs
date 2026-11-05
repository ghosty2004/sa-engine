//! Determinism mode, audio (INF-4). Measured: the main thread's audio entities call rand depending on
//! whether sounds are still playing (`CAEWeatherAudioEntity::UpdateParameters` 0x506647 →
//! `CAEAudioUtility::ResolveProbability`, reached through `CAEStaticChannel::GetPlayTime` 0x4F0F78),
//! and that state comes from DirectSound's real-time play cursor and from two free-running threads.
//! Neutralised here:
//!
//! 1. `CAEStreamThread` (0x4F15C0) and `CAESmoothFadeThread` (0x4EEE90) are created suspended and
//!    resumed by their `Start` (0x4F11F0 / 0x4EEA10, `call [ResumeThread]` at 0x4F11F7 / 0x4EEA1A).
//!    Those two calls are redirected to shims that record `this` and leave the thread suspended; the
//!    loop bodies run on the main thread at every `CAEAudioHardware::Service` call instead (stream: `Service` 0x4F1290
//!    then `m_pStreamingChannel->Service()` (vtable +4), as 0x4F1614..0x4F161E; fade: `Service`
//!    0x4EED10), while their active flags (+0x11 / +0x809) are set.
//! 2. `CAEAudioUtility::GetCurrentTimeInMS`' QueryPerformanceCounter (0x4D9E88) reads the virtual clock.
//! 3. Every DirectSound secondary buffer reports a virtual play state: the buffer vtable's
//!    GetCurrentPosition/GetStatus/Play/Stop/SetCurrentPosition/SetFrequency/Release are wrapped;
//!    the position advances with the virtual clock at frequency x block align bytes per second, a
//!    non-looping buffer stops (position 0) when it reaches its end. Real calls still happen (sound
//!    still plays), only what the game reads back is virtual.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::win::{self, log};

pub static STREAM_THIS: AtomicU32 = AtomicU32::new(0);
pub static FADE_THIS: AtomicU32 = AtomicU32::new(0);
static CELL_STREAM: AtomicU32 = AtomicU32::new(0);
static CELL_FADE: AtomicU32 = AtomicU32::new(0);

/// stdcall `ResumeThread(handle)` replacement at 0x4F11F7 (ecx = the CAEStreamThread).
#[unsafe(naked)]
unsafe extern "C" fn stream_resume_shim() {
    std::arch::naked_asm!(
        "mov dword ptr [{t}], ecx",
        "mov eax, 1",
        "ret 4",
        t = sym STREAM_THIS,
    )
}

/// stdcall `ResumeThread(handle)` replacement at 0x4EEA1A (ecx = the CAESmoothFadeThread).
#[unsafe(naked)]
unsafe extern "C" fn fade_resume_shim() {
    std::arch::naked_asm!(
        "mov dword ptr [{t}], ecx",
        "mov eax, 1",
        "ret 4",
        t = sym FADE_THIS,
    )
}

fn get_u32(a: u32) -> u32 {
    // SAFETY: game objects/vtables known to be live (checked non-null by callers).
    unsafe { std::ptr::read_volatile(a as usize as *const u32) }
}

fn get_u8(a: u32) -> u8 {
    // SAFETY: as above.
    unsafe { std::ptr::read_volatile(a as usize as *const u8) }
}

/// Patches the thread-start sites and the audio clock site. Runs at attach (det mode).
pub fn patch_sites(cell_qpc: u32) -> Result<(), String> {
    CELL_STREAM.store(stream_resume_shim as *const () as usize as u32, Ordering::Release);
    CELL_FADE.store(fade_resume_shim as *const () as usize as u32, Ordering::Release);
    let cell = |c: &AtomicU32| c as *const AtomicU32 as usize as u32;
    let resume: [u8; 6] = [0xFF, 0x15, 0xA4, 0x80, 0x85, 0x00];
    let qpc: [u8; 6] = [0xFF, 0x15, 0xC8, 0x80, 0x85, 0x00];
    let sites = [
        (
            0x4F11F7u32,
            resume,
            cell(&CELL_STREAM),
            "CAEStreamThread::Start ResumeThread -> stepped from CAEAudioHardware::Service",
        ),
        (
            0x4EEA1A,
            resume,
            cell(&CELL_FADE),
            "CAESmoothFadeThread::Start ResumeThread -> stepped from CAEAudioHardware::Service",
        ),
        (
            0x4D9E88,
            qpc,
            cell_qpc,
            "CAEAudioUtility::GetCurrentTimeInMS (QPC) -> virtual",
        ),
    ];
    for (site, expect, _, what) in &sites {
        if win::read(*site, 6).as_deref() != Some(&expect[..]) {
            return Err(format!("{site:#x} ({what}): unexpected bytes"));
        }
    }
    for (site, _, disp, what) in &sites {
        win::patch(site + 2, &disp.to_le_bytes())?;
        log(&format!("oracle: audio site {site:#x} {what}"));
    }
    Ok(())
}

/// `CAEAudioHardware::Service` (thiscall, 0x4D9870). In determinism mode one iteration of each audio
/// thread loop runs first. Hardware Service is called once per frame by `CAudioEngine::Service` and
/// repeatedly by the game's own wait loops (e.g. 0x4DC0E0, reached from
/// `CCutsceneMgr::LoadCutsceneData_preload` 0x5B05FC, spins on it until the stream thread has prepared
/// the cutscene track), so stepping here keeps those waits finite and their iteration count
/// deterministic.
pub extern "thiscall" fn hw_service_hook(this: u32) {
    crate::table::count(crate::table::Id::AudioHardwareService);
    if crate::detwin::det_on() {
        step_threads();
    }
    // SAFETY: the trampoline has this signature.
    let orig: extern "thiscall" fn(u32) = unsafe {
        std::mem::transmute(crate::table::original(crate::table::Id::AudioHardwareService) as usize)
    };
    orig(this)
}

/// One iteration of each audio thread loop, on the main thread.
pub fn step_threads() {
    let s = STREAM_THIS.load(Ordering::Acquire);
    if s != 0 && get_u8(s + 0x11) != 0 {
        // SAFETY: CAEStreamThread::Service (thiscall) and the streaming channel's vtable Service, as
        // the thread's own loop calls them (0x4F1614, 0x4F161E).
        unsafe {
            let service: extern "thiscall" fn(u32) = std::mem::transmute(0x4F1290usize);
            service(s);
            let ch = get_u32(s + 8);
            if ch != 0 {
                let f: extern "thiscall" fn(u32) = std::mem::transmute(get_u32(get_u32(ch) + 4) as usize);
                f(ch);
            }
        }
    }
    let f = FADE_THIS.load(Ordering::Acquire);
    if f != 0 && get_u8(f + 0x809) != 0 {
        // SAFETY: CAESmoothFadeThread::Service (thiscall), as SmoothFadeProc (0x4EEEA8) calls it.
        unsafe {
            let service: extern "thiscall" fn(u32) = std::mem::transmute(0x4EED10usize);
            service(f);
        }
    }
}

// ------------------------------------------------------------------ DirectSound buffers -------

#[derive(Debug, Clone, Copy, Default)]
struct VBuf {
    playing: bool,
    looping: bool,
    /// Position (bytes) at `t0`.
    pos: u32,
    t0: u64,
    /// Bytes per second.
    rate: u32,
    size: u32,
    block_align: u32,
}

static BUFS: Mutex<Option<HashMap<u32, VBuf>>> = Mutex::new(None);
/// Original vtable entries (index → function).
static ORIG: [AtomicU32; 21] = [const { AtomicU32::new(0) }; 21];
static VBUF_CALLS: AtomicU32 = AtomicU32::new(0);

fn now_ticks() -> (u64, u64) {
    (crate::detwin::virtual_now(), crate::detwin::qpf())
}

type GetCaps = extern "system" fn(u32, *mut u32) -> i32;
type GetFormat = extern "system" fn(u32, *mut u8, u32, *mut u32) -> i32;
type GetFrequency = extern "system" fn(u32, *mut u32) -> i32;

fn orig(i: usize) -> usize {
    ORIG[i].load(Ordering::Acquire) as usize
}

/// Reads the static properties of a buffer through the real methods.
fn describe(b: u32) -> VBuf {
    let mut v = VBuf::default();
    // SAFETY: the original methods of a live buffer.
    unsafe {
        let mut caps = [0u32; 5];
        caps[0] = 20;
        if (std::mem::transmute::<usize, GetCaps>(orig(3)))(b, caps.as_mut_ptr()) >= 0 {
            v.size = caps[2];
        }
        let mut wfx = [0u8; 64];
        let mut n = 0u32;
        if (std::mem::transmute::<usize, GetFormat>(orig(5)))(b, wfx.as_mut_ptr(), 64, &mut n) >= 0 {
            v.block_align = u16::from_le_bytes([wfx[12], wfx[13]]) as u32;
        }
        let mut f = 0u32;
        if (std::mem::transmute::<usize, GetFrequency>(orig(8)))(b, &mut f) >= 0 {
            v.rate = f * v.block_align.max(1);
        }
    }
    v
}

fn with_buf<T>(b: u32, f: impl FnOnce(&mut VBuf, u64, u64) -> T) -> T {
    VBUF_CALLS.fetch_add(1, Ordering::Relaxed);
    let (now, qpf) = now_ticks();
    let mut g = BUFS.lock().unwrap_or_else(|e| e.into_inner());
    let map = g.get_or_insert_with(HashMap::new);
    let v = map.entry(b).or_insert_with(|| {
        let mut v = describe(b);
        v.t0 = now;
        v
    });
    f(v, now, qpf)
}

/// Current virtual position; marks a finished non-looping buffer stopped.
fn advance(v: &mut VBuf, now: u64, qpf: u64) -> u32 {
    if !v.playing {
        return v.pos;
    }
    let moved = ((now.saturating_sub(v.t0)) as u128 * v.rate as u128 / qpf.max(1) as u128) as u64;
    let p = v.pos as u64 + moved;
    if v.size == 0 {
        return 0;
    }
    if v.looping {
        (p % v.size as u64) as u32
    } else if p >= v.size as u64 {
        v.playing = false;
        v.pos = 0;
        v.t0 = now;
        0
    } else {
        p as u32
    }
}

fn rebase(v: &mut VBuf, now: u64, qpf: u64) {
    let p = advance(v, now, qpf);
    v.pos = p;
    v.t0 = now;
}

extern "system" fn vb_get_current_position(b: u32, play: *mut u32, write: *mut u32) -> i32 {
    let p = with_buf(b, advance);
    // SAFETY: caller-provided out pointers (either may be null).
    unsafe {
        if !play.is_null() {
            *play = p;
        }
        if !write.is_null() {
            *write = p;
        }
    }
    0
}

extern "system" fn vb_get_status(b: u32, status: *mut u32) -> i32 {
    // SAFETY: the original method (keeps flags such as BUFFERLOST/LOCHARDWARE).
    let hr = unsafe {
        (std::mem::transmute::<usize, extern "system" fn(u32, *mut u32) -> i32>(orig(9)))(b, status)
    };
    if hr >= 0 && !status.is_null() {
        let (playing, looping) = with_buf(b, |v, now, qpf| {
            advance(v, now, qpf);
            (v.playing, v.looping)
        });
        // SAFETY: valid out pointer.
        unsafe {
            let mut s = *status & !(0x1 | 0x4); // DSBSTATUS_PLAYING | DSBSTATUS_LOOPING
            if playing {
                s |= 0x1;
                if looping {
                    s |= 0x4;
                }
            }
            *status = s;
        }
    }
    hr
}

extern "system" fn vb_play(b: u32, r: u32, prio: u32, flags: u32) -> i32 {
    // SAFETY: the original method.
    let hr = unsafe {
        (std::mem::transmute::<usize, extern "system" fn(u32, u32, u32, u32) -> i32>(orig(12)))(
            b, r, prio, flags,
        )
    };
    if hr >= 0 {
        with_buf(b, |v, now, qpf| {
            rebase(v, now, qpf);
            v.playing = true;
            v.looping = flags & 1 != 0;
        });
    }
    hr
}

extern "system" fn vb_stop(b: u32) -> i32 {
    // SAFETY: the original method.
    let hr = unsafe { (std::mem::transmute::<usize, extern "system" fn(u32) -> i32>(orig(18)))(b) };
    with_buf(b, |v, now, qpf| {
        rebase(v, now, qpf);
        v.playing = false;
    });
    hr
}

extern "system" fn vb_set_current_position(b: u32, pos: u32) -> i32 {
    // SAFETY: the original method.
    let hr = unsafe { (std::mem::transmute::<usize, extern "system" fn(u32, u32) -> i32>(orig(13)))(b, pos) };
    if hr >= 0 {
        with_buf(b, |v, now, _| {
            v.pos = if v.size > 0 { pos % v.size } else { 0 };
            v.t0 = now;
        });
    }
    hr
}

extern "system" fn vb_set_frequency(b: u32, freq: u32) -> i32 {
    // SAFETY: the original method.
    let hr =
        unsafe { (std::mem::transmute::<usize, extern "system" fn(u32, u32) -> i32>(orig(17)))(b, freq) };
    if hr >= 0 {
        with_buf(b, |v, now, qpf| {
            rebase(v, now, qpf);
            let mut f = 0u32;
            // SAFETY: the original GetFrequency (resolves DSBFREQUENCY_ORIGINAL).
            unsafe { (std::mem::transmute::<usize, GetFrequency>(orig(8)))(b, &mut f) };
            v.rate = f * v.block_align.max(1);
        });
    }
    hr
}

extern "system" fn vb_release(b: u32) -> u32 {
    // SAFETY: the original method.
    let n = unsafe { (std::mem::transmute::<usize, extern "system" fn(u32) -> u32>(orig(2)))(b) };
    if n == 0
        && let Ok(mut g) = BUFS.lock()
        && let Some(m) = g.as_mut()
    {
        m.remove(&b);
    }
    n
}

/// Creates a throw-away DirectSound device and secondary buffer to find the secondary-buffer vtable
/// (shared by every secondary buffer of the dsound implementation) and wraps the methods above.
/// Runs on the main thread from the `_cinit` shim, before the game creates any buffer.
pub fn virtualise_dsound() -> Result<String, String> {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress, LoadLibraryA};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
    // SAFETY: COM calls through vtables of objects we create and release here.
    unsafe {
        let mut m = GetModuleHandleA(c"dsound.dll".as_ptr() as *const u8);
        if m.is_null() {
            m = LoadLibraryA(c"dsound.dll".as_ptr() as *const u8);
        }
        let create =
            GetProcAddress(m, c"DirectSoundCreate8".as_ptr() as *const u8).ok_or("no DirectSoundCreate8")?;
        let create: extern "system" fn(*const c_void, *mut u32, *mut c_void) -> i32 =
            std::mem::transmute(create);
        let mut ds = 0u32;
        let hr = create(std::ptr::null(), &mut ds, std::ptr::null_mut());
        if hr < 0 || ds == 0 {
            return Err(format!("DirectSoundCreate8 failed {hr:#x}"));
        }
        let dsv = get_u32(ds);
        let call = |i: u32| get_u32(dsv + 4 * i) as usize;
        let set_coop: extern "system" fn(u32, *mut c_void, u32) -> i32 = std::mem::transmute(call(6));
        set_coop(ds, GetDesktopWindow(), 2);
        // 16-bit mono 22050 Hz, 4096 bytes, CTRLVOLUME | CTRLFREQUENCY | GETCURRENTPOSITION2
        let wfx: [u16; 9] = [1, 1, 22050, 0, 44100, 0, 2, 16, 0];
        let mut desc = [0u32; 9];
        desc[0] = 36;
        desc[1] = 0x80 | 0x20 | 0x10000;
        desc[2] = 4096;
        desc[4] = wfx.as_ptr() as usize as u32;
        let create_buf: extern "system" fn(u32, *const u32, *mut u32, *mut c_void) -> i32 =
            std::mem::transmute(call(3));
        let mut buf = 0u32;
        let hr = create_buf(ds, desc.as_ptr(), &mut buf, std::ptr::null_mut());
        let release_ds: extern "system" fn(u32) -> u32 = std::mem::transmute(call(2));
        if hr < 0 || buf == 0 {
            release_ds(ds);
            return Err(format!("CreateSoundBuffer failed {hr:#x}"));
        }
        let vt = get_u32(buf);
        for (i, o) in ORIG.iter().enumerate() {
            o.store(get_u32(vt + 4 * i as u32), Ordering::Release);
        }
        let release_buf: extern "system" fn(u32) -> u32 = std::mem::transmute(orig(2));
        release_buf(buf);
        release_ds(ds);
        let wraps: [(u32, usize); 7] = [
            (2, vb_release as *const () as usize),
            (4, vb_get_current_position as *const () as usize),
            (9, vb_get_status as *const () as usize),
            (12, vb_play as *const () as usize),
            (13, vb_set_current_position as *const () as usize),
            (17, vb_set_frequency as *const () as usize),
            (18, vb_stop as *const () as usize),
        ];
        for (i, f) in wraps {
            win::patch(vt + 4 * i, &(f as u32).to_le_bytes())?;
        }
        Ok(format!("secondary buffer vtable {vt:#x}: 7 methods virtualised"))
    }
}

pub fn status() -> String {
    let n = BUFS
        .lock()
        .map(|g| g.as_ref().map(|m| m.len()).unwrap_or(0))
        .unwrap_or(0);
    format!(
        "audio stream_thread={:#x} fade_thread={:#x} vbufs={n} vbuf_calls={}",
        STREAM_THIS.load(Ordering::Relaxed),
        FADE_THIS.load(Ordering::Relaxed),
        VBUF_CALLS.load(Ordering::Relaxed)
    )
}
