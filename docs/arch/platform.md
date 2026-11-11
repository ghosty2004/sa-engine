# STD-1/STD-2: the native platform layer (phase B) and `sa-game`

Phase B (PLAN §2, F10) runs the ported engine as a Linux executable without Wine and without
executing any original code (it reads the data sections of the user's gta_sa.exe, section 6). This document describes the platform layer that makes that possible (`sa-platform`),
the render-device seam (`sa_platform::gpu` + `sa_rw_wgpu::d3d9`), the `sa-game` skeleton that
boots the ported code natively, and the ordered list of what is still missing on the boot → main
menu → walking path (`docs/arch/platform-mvp-gaps.csv`).

## 1. Principle: the ports keep calling Win32; phase B implements Win32's semantics

The ported functions call imports exactly where the original does, through `orig!` declarations
(ADR-001 §4): `QueryPerformanceCounter` in `CTimer`, `fopen`/`_chdir` in `CFileMgr`,
`CreateFileA`/`ReadFile`/semaphores/`CreateThread` in `CdStream`, `PeekMessageA` in
`MessageLoop`, `IDirectInputDevice8::GetDeviceState` in `CPad`, `IDirect3DDevice9` in the RW
driver. In phase A these reach Wine. In phase B (STD-2, 2026-11-11) a declaration `=> unported`
resolves in this order (`sa_core::orig`, `__orig_body`):

1. a **host stand-in** registered under the declaration's path (`sa_core::orig::host::set`,
   signature checked by `TypeId`; sa-game's `bind.rs` and the L1 specs use these);
2. the **address registry** (`sa_core::orig::registry`, 32-bit x86 only): the declaration's callee
   address is evaluated at call time exactly as phase A evaluates it (a literal address; a vtable
   slot read from the object, `vslot`; a function pointer read from its cell, `cell`/`table`; the
   IAT cell of an import call instruction, `site`; a run-time address, `dyn`) and looked up in a
   table sorted by original address. A hit calls the native entry with the declaration's
   signature and the original convention (`extern "C-unwind"`/`"stdcall-unwind"`/
   `"thiscall-unwind"`/`"fastcall-unwind"`, so a panic further down unwinds to the caller);
3. otherwise a panic `NAME is not ported; phases B/C have no original code to call (0xADDR = what)`
   naming the declaration, the address and, when known, the address's name (an import's
   `DLL!name`, a COM method). These panics are the live MVP gap list.

The registry holds (sa-game `native.rs`): every `#[hook]` port of `sa_ports::ALL` (the macro now
emits a phase B native entry next to the phase A shim; `HookReg::entry` returns it), every
`#[native]` port of `sa_ports::natives()` (ports that cannot be hooked in phase A: lone `ret`s,
entries that loops branch back into, register arguments, `CdStreamOpen`/`CdStreamRead`; see
section 9), the CRT functions phase B replaces (`crt.rs`), the exe's imports by name
(`imports.rs`, below), native COM objects (`com.rs`, `dx.rs`) and the temporary RW driver
stand-ins (`rw_standin.rs`, section 9; registered only where no port exists). This replaces
`tools/native/wire_ported.py`/`wired.rs`: a declaration of any ported callee, in any crate,
dispatches to the port without per-declaration wiring.

**Imports by name.** sa-game writes a trap address (`0x20000 + 0x10·i`) into each of the 214 IAT
cells of the arena (the shipped IAT is pre-bound by the packer) and registers one native
implementation per import at its trap (`imports.rs`: 65 so far, most wrapping `bind.rs`). A
`site`/`cell` declaration of that import in any module reaches it; an unimplemented import fails
as `… (0x20b90 = USER32.dll!PeekMessageA)`. `site` declarations are resolved by reading the
`call dword ptr [cell]` bytes from a copy of the exe's `.text` kept in memory (not mapped).

**COM.** The DirectInput objects (`IDirectInput8A`, the mouse and two pads) live on a page sa-game
maps at 0x30000 (the pseudo addresses `bind.rs` already used); their vtables hold trap addresses
whose implemented methods are registry entries (`com.rs`). The same scheme is how the
`IDirect3D9`/`IDirect3DDevice9` objects of the RW D3D9 driver can be served by `WgpuDevice`.

**CRT.** `crt.rs` replaces the MSVC CRT functions the ports declare: allocation, string functions,
`rand`/`srand` (sa-core's MSVC LCG), and `sprintf`/`printf`/`sscanf` through naked cdecl entries
that hand the variadic stack words to a formatter with MSVC's conventions (`crt_fmt.rs`: 3-digit
exponents, `%p` as 8 upper-case digits, `1.#INF00`).

So phase B does not rewrite port code. `sa-platform` implements the Win32/CRT/DirectX semantics
the game observes, target-independently, and `sa-game` binds them. The in-process Win32/Wine
backend (phase A, sa-hook) is unchanged: in phase A the declarations still call the original
address.

## 2. `sa-platform` modules

| module | what | targets |
|---|---|---|
| `fs` | `GameFs` (read, random-access `RandomRead`, exists, is_dir, list_dir, and for writable trees write_file/create_dir/remove_file), `MemFs` (writable, case-insensitive) | all |
| `vfs` | the Windows namespace the game sees: the install mounted read-only at `C:\Program Files\Rockstar Games\GTA San Andreas` (the initial `_getcwd`), a writable per-user folder at `C:\Users\Player\Documents` (`SHGetFolderPath(CSIDL_PERSONAL)` / the "Personal" shell folder), a current directory (`_chdir`), case-insensitive resolution of every component through the mounts. `FileTable`: the open files behind the integer handles the game stores (`FILE*`, `HANDLE`), MSVC CRT semantics the game can observe: `fopen` modes, text-mode CR/LF and Ctrl-Z handling, `fgets`, EOF/error flags, `ftell` = file offset, append mode, write-back on close; random reads at an offset (overlapped `ReadFile`) | all |
| `kernel` | kernel32 objects for `CdStream`: counting semaphores (`CreateSemaphoreA`, `ReleaseSemaphore`, `WaitForSingleObject(Ex)` with timeouts), suspended threads started by `ResumeThread` (entry resolved by the caller from the original code address to its Rust port), thread priorities (recorded), `LocalAlloc`/`LocalFree` | all (threads: native) |
| `win32` | keys: physical key → virtual key, scan code, extended flag (US layout, Num Lock on); `lParam` layout of key messages (repeat 1, scan, `KF_EXTENDED`, context, previous state, transition); `WM_SYSKEY*` for Alt combinations and F10; `MapVirtualKeyA` (VK↔VSC, VK→char, VSC→VK_EX); `GetKeyState`/`GetAsyncKeyState` with L/R modifier tracking and toggle bits; `MessageQueue` (`PeekMessageA` with/without `PM_REMOVE`, `PostQuitMessage` → `WM_QUIT` after the queue drains, `WM_CHAR` from layout text events, mouse, wheel, focus/activate, size, close messages); `Win32Time` (`QueryPerformanceFrequency` = 10 MHz, the value Wine reports, so `CTimer::m_snTimerDivider` is 10,000 as in the phase A runs; `QueryPerformanceCounter`; `timeGetTime`/`GetTickCount` wrapping at 2^32) | all |
| `input` | full physical key set, mouse buttons incl. X1/X2, ordered `InputEvent`s per frame, `DiMouseState2` (the 0x14-byte `DIMOUSESTATE2` `CPad::GetMouseState` reads: relative mickeys, wheel in 120 units, `0x80` buttons) and `DiJoyState2` (the 0x110-byte `DIJOYSTATE2` `CPad::UpdatePads` reads: left stick lX/lY, triggers on lZ, right stick lRx/lRy, buttons 0–9 = A B X Y LB RB Back Start LS RS, D-pad on POV 0, axes in the -2000..2000 range `diPadSetRanges` sets) | all |
| `audio` | `AudioOut` sink (44.1 kHz stereo i16, the format the game asks DirectSound for; non-blocking push, queued frames = write-cursor distance, frames played = play cursor), `NullAudio`, `remix` | all |
| `gpu` | the `IDirect3DDevice9` seam (section 4) | all |
| `clock` | monotonic clock | all |
| `native` | `NativeFs` (now writable: case-insensitive create, listing cache invalidation), winit window (every key mapped, text/char events, cursor position, X buttons, focus/resize/close events, optional click-to-capture), `Pads` (gilrs, feature `gamepad`), `CpalAudio` (feature `audio`: ring buffer, linear resampling and channel remix to the device format, stream owned by its own thread because `cpal::Stream` is not `Send`), `documents_dir()` (`$XDG_DATA_HOME/sa-engine/documents`) | native |

Checked: `cargo clippy -D warnings` on host (`--features gamepad,audio --all-targets`),
`wasm32-unknown-unknown` and `i686-pc-windows-gnu` (`--no-default-features`); 22 unit tests.

## 3. Input mapping to `CPad` / `CControllerConfigManager`

The original gets input three ways, and phase B feeds all three at the same API level:

1. **Keyboard**: `WM_KEYDOWN`/`WM_KEYUP` → `MainWndProc` → `GTATranslateKey` (`MapVirtualKeyA` for
   the default case, `GetKeyState` for the shift probes) → `RsKeyboardEventHandler` →
   `Input::KeyboardHandler` → `CPad::TempKeyState`. Phase B posts the exact messages (`win32::
   MessageQueue`) that the ported `Win::MessageLoop` pulls with `PeekMessageA`, and binds
   `MapVirtualKeyA`/`GetKeyState`. **Gap:** `Win::__MainWndProc` (0x747EB0) is not ported, so
   `DispatchMessageA` has nowhere to deliver; sa-game counts the undelivered messages per id.
2. **Mouse**: DirectInput `GetDeviceState(DIMOUSESTATE2)` in `CPad::GetMouseState`: bound to
   `InputState::di_mouse_state()` (raw relative motion from winit device events).
3. **Pads**: DirectInput `GetDeviceState(DIJOYSTATE2)` in `CPad::UpdatePads`: bound to
   `DiJoyState2::from_pad` of the gilrs pad (pad 1/2 = connection order, as
   `EnumDevicesCallback` assigns them). `CControllerConfigManager` then maps buttons through the
   ported `InitDefaultControlConfigJoyPad` tables as in the original.

## 4. Render-device seam

`sa_platform::gpu::D3d9Device` mirrors the `IDirect3DDevice9` subset the RW D3D9 driver uses
(docs/re/rw/d3d9.md): resources (textures incl. render targets and DXT, vertex/index buffers,
vertex declarations, shaders), state (`SetRenderState`, `SetTextureStageState`,
`SetSamplerState`, `SetTexture`, `SetTransform`, material, lights, viewport, scissor, FVF /
declaration, stream source, indices, render target), frames (`BeginScene`/`EndScene`, `Clear`,
`DrawPrimitive`, `DrawIndexedPrimitive`, `DrawPrimitiveUP`, `DrawIndexedPrimitiveUP`,
`Present`) and a read-back for screen grabs and tests. Values are D3D9's own enums, so the binding
from the ported `_rwD3D9*` code (PORT-RW-4, sa-rwcore) is a mechanical translation of its COM
`vslot` declarations, and a D3D9 call stream captured in phase A (M4) can be replayed through it.
Locks are modelled as "write these bytes at unlock".

`sa_rw_wgpu::d3d9::WgpuDevice` implements it on wgpu (section 4.1).

### 4.1 `WgpuDevice`

`crates/sa-rw-wgpu/src/d3d9.rs` (+ `d3d9_ff.wgsl`, `d3d9_common.wgsl`; module docs list every
approximation). `WgpuDevice::new(device, queue, w, h)` renders into its own back buffer
(Rgba8Unorm + Depth24PlusStencil8); `new_headless(w, h)` for tests and offscreen frames;
`blit_to(view, format)` shows the back buffer on a window surface after `present()`;
`take_errors()` returns recorded wgpu validation errors instead of panicking.

* **Ordering.** Draws and clears go into a pending list encoded as one render pass at flush time.
  It flushes before an update of a buffer range or texture a pending draw uses (append with
  NOOVERWRITE does not flush), on render-target change, reset, present, read-back and blit.
  Per-draw uniforms go into a dynamic-offset buffer; UP vertices and generated indices into an arena.
* **Fixed function in one WGSL uber-shader**, driven by uniforms; pipelines cached by vertex
  layout, topology, blend, colour mask, depth, depth bias, stencil and cull. Strips, fans and line
  strips are expanded to index lists with D3D winding (front faces clockwise).
* **Implemented:** World/View/Proj row-vector transforms, XYZRHW through the viewport with
  perspective-correct `1/rhw` and the D3D9 half-pixel convention, every D3DDECLTYPE wgpu has
  (D3DCOLOR swizzled), FVF and multi-stream declarations; texture stages 0–1 (DISABLE…LERP except
  PREMODULATE/BUMPENVMAP, args DIFFUSE/CURRENT/TEXTURE/TFACTOR/SPECULAR with COMPLEMENT and
  ALPHAREPLICATE, TEXCOORDINDEX 0–1); alpha test; fog (table/vertex, LINEAR/EXP/EXP2, range fog);
  lighting (AMBIENT, material, COLORVERTEX and material sources, 8 directional/point/spot lights);
  blending (all D3DBLEND factors, BLENDOP, separate alpha, BLENDFACTOR); depth (ZENABLE/ZWRITE/
  ZFUNC/bias), stencil incl. two-sided, CULLMODE, COLORWRITEENABLE, scissor; samplers (WRAP/
  MIRROR/CLAMP/BORDER, MIN/MAG/MIP filters, anisotropy, MAXMIPLEVEL); every uncompressed RW texture
  format converted to RGBA8, DXT1/3/5 as BC1/2/3 (software-decoded without BC support or for
  non-multiple-of-4 sizes), mip chains, sub-rect updates with pitch, render-target textures;
  DrawPrimitive/DrawIndexedPrimitive/DrawPrimitiveUP/DrawIndexedPrimitiveUP (UP unsets stream 0 as
  D3D9 does); out-of-range draws return `InvalidCall`.
* **Approximated:** lighting in world space without the specular term; fog distance |view z|;
  a stage without a texture samples opaque white; A8 samples (0,0,0,a); BORDERCOLOR snapped to
  transparent black / opaque black / opaque white; MIRRORONCE = MIRROR. Ignored: MIPMAPLODBIAS,
  texture transforms, FILLMODE, SHADEMODE, point size, clip planes, LASTPIXEL.
* **NotAvailable:** D3D9 vertex/pixel shader bytecode (the game's building/vehicle pipelines and
  RW's generated skin shaders need WGSL equivalents), cube textures, P8 and V8U8,
  `read_render_target` on wasm32 (needs an async variant: WebGPU maps buffers asynchronously).
* **Tests** (13, real Vulkan adapter; skipped without one): clear/read-back, pre-transformed
  triangle with the half-pixel edge, textured MODULATE + alpha test, transforms + depth + culling,
  fan, indexed draw + update ordering, alpha blending, DXT1 (BC and software paths), directional
  lighting, linear fog, render-target texture, shaders NotAvailable, a composite scene
  (`SA_D3D9_PNG=out.png`).

`stretch_rect(src, src_rect, dst, dst_rect, linear)` (STD-4, the only seam extension of the RW
driver port): `StretchRect` between render targets (`None` = back buffer, else a render-target
texture). `WgpuDevice` copies equal-size rectangles of two different targets on the GPU
(`copy_texture_to_texture`, both Rgba8Unorm) and resamples scaled or same-target copies on the CPU
from a read-back (nearest, or bilinear for D3DTEXF_LINEAR; NotAvailable on wasm32, which has no
blocking read-back). `GetRenderTargetData` is built on `read_render_target`, `UpdateSurface` on
`update_texture` (sa-game `d3d9.rs`).

Not yet in the trait: `SetClipPlane`, auto-generated mipmaps, cube textures (sa-game's
`CreateCubeTexture` returns the seam's NotAvailable), multisampling, named
D3DTOP/D3DTA/D3DBLEND/D3DCMP constants.

## 5. `sa-game`

```sh
cargo build -p sa-game --target i686-unknown-linux-gnu      # phase B MVP is a 32-bit process
target/i686-unknown-linux-gnu/debug/sa-game --game wine/gta-sa --frames 40 --report run.md
```

(`.cargo/config.toml` points pkg-config at `/usr/lib32/pkgconfig` for that target only. The crate
still builds for x86_64, where nothing dispatches through the registry.)

Start-up: check the install, map the arena from the user's exe (section 6), map the DirectInput
object page, install the address registry (section 1), check that every `globals!` declaration
inside the arena equals the exe image's initial value (1,019 declarations, 0 differ), bind the
platform, then replay `WinMain` (0x748710) call by call (`crates/sa-game/src/winmain.rs`,
transcribed from the 1.0 US bytes; every callee is an `orig!` declaration resolved by the
registry), one check outside WinMain's order (a `gta3.img` sector through `CdStream`'s
streaming thread), and up to N iterations of WinMain's main loop (one `PeekMessageA`, or the
`gGameState` machine of jump table 0x748EF8: splashes, logo/title movies, frontend loading,
frontend idle, loading, idle) with synthetic input (Enter every 8 iterations). The loop stops
after three identical failures. Each step is `rust: ok`, `rust: blocked` (the first unported
callee, by name and address), `rust: CRASHED` (memory fault or a wait that never ends, found by
running the step in a forked child first; `SA_GAME_NO_PROBE=1` runs steps directly, e.g. under
gdb). `--report` adds the platform calls served and every native entry the registry dispatched to.

Result on 2026-11-11 (i686, the user's install):

| WinMain step | result |
|---|---|
| `SystemParametersInfoA` + `Win::IsAlreadyRunning` | ok |
| `RsEventHandler(rsINITIALIZE)` → `App::AppEventHandler` → `CGame::InitialiseOnceBeforeRW` (CFileMgr, memory manager), `RsInitialize` → `psInitialize` | runs `CFileMgr::Initialise`, `InitUserDirectories`, `C_PcSave::SetSaveDirectory` (CRT `sprintf`), then **blocked: `CText::Load` (0x6A01A0, 223 instructions)** inside `InitialiseLanguage` |
| `Win::Win32_InitApplication` | ok (`RegisterClassA` records `__MainWndProc` for `DispatchMessageA`) |
| `GetCommandLineA` + `Win::CommandLineToArgv` (0x746480) | ok, argc = 1 (no `rsPREINITCOMMANDLINE`) |
| `Win::Win32_InitInstance` → `ps->window` | ok |
| `MakeControllerActionsBlank` + `InitDefaultControlConfiguration` | ok (the `.HOODLUM` key read uses its constant result in phase B, section 6) |
| `DirectInput8Create` + `diMouseInit` + `diPadInit` | ok (native DirectInput objects) |
| `GetMouseSetUp` + `InitDefaultControlConfigMouse` | ok (lmb/rmb/mmb bound) |
| `RsEventHandler(rsRWINITIALIZE)` → `Game::RwInitialize` → `Rs::RsRwInitialize` | **blocked: `RwEngineInit` (0x7F3170, 208 instructions)**, the RW engine bring-up |
| `rsCOMMANDLINE` + window style | ok |
| `RsEventHandler(rsCAMERASIZE)` → `CameraSize` | ok (no camera yet: returns early) |
| `SystemParametersInfoA` ×5 + `ShowWindow` + `UpdateWindow` | ok |
| `SetDirMyDocuments` + `gta_sa.set` (`LoadSettings`/`ReinitControls`) + `SetDir` | ok (no `gta_sa.set` on a first run) |
| `SetErrorMode` + `RwInitialized` + `RsMouseSetPos` + `CGamma::Init` | **blocked: `RwD3D9GetCurrentD3DDevice` (0x7F9D50)**, RW D3D9 driver (PORT-RW-4) |
| check: `CdStreamInit` + `CdStreamOpen(gta3.img)` + `CdStreamRead` + `CdStreamSync` | ok: header `VER2`, 16,297 entries, first `levelmap_stream0.ipl` (the x86_64 null-pointer crash is gone under i686) |
| main loop, `gGameState` INITIAL: `CLoadingScreen::LoadSplashes` | **crashed**: `CTxdStore::AddTxdSlot` on a null TXD pool (`CTxdStore::Initialise` runs in `CGame::InitialiseRenderWare`, behind the RW bring-up) |

Summary (`--frames 10`): 12 steps ok, 3 blocked, 3 crashed (the main loop, same cause each
time), 53 platform calls served natively, 40 native entries reached through the registry.

The next blockers in order are therefore `CText::Load` (+ `CKeyArray`/`CData`/
`CMissionTextOffsets::Load`), the RW engine bring-up (`RwEngineInit`/`Open`/`Start`,
`_rwDeviceSystemRequest`, subsystem/video-mode queries, `psSelectDevice`) which ends in the RW
D3D9 driver's system function, then `CGame::InitialiseRenderWare` and the loading-screen draw path
(see section 7). Window messages: `DispatchMessageA` calls the registered window procedure through
the registry; `Win::__MainWndProc` (0x747EB0, 459 instructions) is not ported, so keyboard
messages are counted as undelivered for now.

`--screenshot out.png` / `--window [--exit-after S]` are unchanged from STD-1 (status screen
through `WgpuDevice`).

## 6. The legacy address arena

Ported code reaches game state by its 1.0 US address (`GPtr::from_addr`, `g::<T>(addr)`,
`rdata(addr)` strings, vtable addresses stored in objects, `globals!`). sa-game runs as a 32-bit
process and maps 0x858000–0xCB1000 with `MAP_FIXED_NOREPLACE`, filled at start-up from the data
sections of `<game dir>/gta_sa.exe` (SHA-256 `a559aa77…cbd26` checked; read only, never copied
into the repository): `.rdata` (then made read-only, as the loader does), `.data` and its
zero-filled `.bss` extent, `_TEXT_HA`, `_rwdseg`, `.rsrc`. No code section is mapped. The IAT
cells get trap addresses (section 1). This replaced STD-1's zeroed arena and its 18 string-literal
patches.

`globals!` storage aliases the arena: in a 32-bit phase B process `Global::ptr()` returns the
original address once sa-game has registered the arena (`sa_core::orig::registry::set_arena`), so
raw-address and `globals!` accesses share one copy (the `RS_PS` split is gone). Before that, and on
other targets, globals keep their Rust statics.

`CControllerConfigManager::InitDefaultControlConfiguration` reads the GO_FORWARD key through the
no-CD chunk `stc; mov ebx, [0x15608DC]; adc ebx, [0x5DF174]` (a `.HOODLUM` dword plus `.text`
bytes). Both are static (the only reference to 0x15608DC is this read), the sum is 0x3FB (`rsUP`);
phase B uses that constant (`PROT_GO_FORWARD`), phase A still reads the bytes.

A SIGSEGV/SIGBUS handler names the step and the section of the faulting address.

## 7. The MVP gap list

`tools/native/mvp_gaps.py [--next N]` (deterministic, ~8 s) writes `docs/arch/platform-mvp-gaps.csv`:
every function the original executes on the boot → menus → loading → walking path (runtime
coverage `catalog/raw/coverage-all.csv`) that phase B does not run natively, in call order (DFS
from `WinMain` over the static call graph, callees in call-site order, `.HOODLUM` stubs followed,
switch tables resolved, phase markers deferring later phases; then `ref`, `crt-init` and
`dynamic` tiers). A function counts as Rust when its state is ≥ implemented **or** it has a
`#[hook]`/sa-game registration in the source tree; `kind=native` marks excluded functions sa-game
replaces at their address (CRT, the DirectInput thunk).

Counts (2026-11-11, STD-2): 7,274 functions on the path, 3,355 in Rust, **3,592 gaps**, 297
excluded, 30 native replacements. By tier: static 1,270, ref 855, crt-init 1,303 (C++ static
initialisers: in phase B their results come from `globals!`/the exe's `.data`, so most need no
port), dynamic 164. By phase: boot 1,819 (mostly initialisers), loading 778, in-world 682,
frontend-menu 149. By subsystem: UNK-game 1,491, RW 580, AI 362, RENDER 262, AUDIO 225, WORLD 134,
POP 125, HUD 105, SCRIPT 86, VEH 71.

### Next 50 functions, in call order (static tier; STD-3, after the RW bring-up)

Counts after STD-3: 7,274 functions on the path, 4,263 in Rust, **2,655 gaps**, 285 excluded,
71 native replacements (CRT, COM/DirectX objects, the RW driver stand-ins of section 9.4).

| # | address | function | subsystem | instructions |
|---|---|---|---|---|
| 1 | 0x748710 | `Win::NOTSA_WinMain` | CORE | 583 |
| 2 | 0x6a0064 | `(unnamed)` | UNK-game | 49 |
| 3 | 0x728640 | `CSprite2d::DrawBarChart` | RENDER | 322 |
| 4 | 0x5078a0 | `CAudioEngine::ServiceLoadingTune` | AUDIO | 22 |
| 5 | 0x4f0000 | `CAESoundManager::Service` | AUDIO | 856 |
| 6 | 0x4d80b0 | `CAEAudioEnvironment::GetPositionRelativeToCamera-vec` | AUDIO | 191 |
| 7 | 0x59acd0 | `CMatrix` | UNK-game | 10 |
| 8 | 0x50ae10 | `CCamera::Get_Just_Switched_Status` | CAM | 2 |
| 9 | 0x4d86b0 | `CAEAudioHardware::PlaySound` | AUDIO | 111 |
| 10 | 0x4e0280 | `CAEMP3BankLoader::GetSoundBuffer` | AUDIO | 83 |
| 11 | 0x4e0380 | `CAEMP3BankLoader::GetLoopOffset` | AUDIO | 14 |
| 12 | 0x4f0c40 | `CAEStaticChannel::SetAudioBuffer` | AUDIO | 268 |
| 13 | 0x406ce0 | `(unnamed)` | UNK-game | 6 |
| 14 | 0x4d7b50 | `CAEAudioChannel::SetNotInRoom` | AUDIO | 42 |
| 15 | 0x4d8870 | `CAEAudioHardware::SetChannelVolume` | AUDIO | 14 |
| 16 | 0x4d7950 | `CAEAudioChannel::SetPosition` | AUDIO | 20 |
| 17 | 0x4d9870 | `CAEAudioHardware::Service` | AUDIO | 54 |
| 18 | 0x4d8990 | `CAEAudioHardware::RescaleChannelVolumes` | AUDIO | 293 |
| 19 | 0x4d9e50 | `CAEAudioUtility::AudioLog10` | AUDIO | 11 |
| 20 | 0x4d7c60 | `CAEAudioChannel::SetVolume` | AUDIO | 62 |
| 21 | 0x4eeaa0 | `CAESmoothFadeThread::RequestFade` | AUDIO | 163 |
| 22 | 0x4eec80 | `CAESmoothFadeThread::SetBufferVolume` | AUDIO | 47 |
| 23 | 0x4d8da0 | `CAEAudioHardware::UpdateReverbEnvironment` | AUDIO | 51 |
| 24 | 0x4d7aa0 | `CAEAudioChannel::SetReverbAndDepth` | AUDIO | 61 |
| 25 | 0x4e0450 | `CAEMP3BankLoader::UpdateVirtualChannels` | AUDIO | 93 |
| 26 | 0x4dfe30 | `CAEMP3BankLoader::Service` | AUDIO | 268 |
| 27 | 0x5523d0 | `delete` | UNK-game | 18 |
| 28 | 0x54ef40 | `CSimpleTransform::UpdateRwMatrix` | UNK-game | 27 |
| 29 | 0x5523c0 | `new` | UNK-game | 2 |
| 30 | 0x4708e0 | `CStreamedScripts::RemoveStreamedScriptFromMemory` | SCRIPT | 12 |
| 31 | 0x75d530 | `_rpTriListMeshGenerate` | RW | 169 |
| 32 | 0x7591b0 | `RpBuildMeshGenerateDefaultTriStrip` | RW | 7 |
| 33 | 0x7591d0 | `(unnamed)` | RW | 355 |
| 34 | 0x759790 | `(unnamed)` | RW | 774 |
| 35 | 0x75a090 | `(unnamed)` | RW | 100 |
| 36 | 0x75a1b0 | `(unnamed)` | RW | 55 |
| 37 | 0x75a250 | `(unnamed)` | RW | 103 |
| 38 | 0x75a370 | `(unnamed)` | RW | 269 |
| 39 | 0x75a660 | `(unnamed)` | RW | 219 |
| 40 | 0x748f50 | `RpAnisotTextureGetMaxAnisotropy` | RW | 8 |
| 41 | 0x748f20 | `RpAnisotGetMaxSupportedMaxAnisotropy` | RW | 3 |
| 42 | 0x748f30 | `RpAnisotTextureSetMaxAnisotropy` | RW | 9 |
| 43 | 0x7016a0 | `(unnamed)` | UNK-game | 46 |
| 44 | 0x5d5ac0 | `CCarFXRenderer::Initialise` | RW | 5 |
| 45 | 0x5da020 | `CCustomCarEnvMapPipeline::CreatePipe` | RW | 73 |
| 46 | 0x5d9f80 | `CCustomCarEnvMapPipeline::CreateCustomObjPipe` | RW | 60 |
| 47 | 0x8064d0 | `(unnamed)` | RW | 101 |
| 48 | 0x806de0 | `(unnamed)` | RW | 105 |
| 49 | 0x8067b0 | `(unnamed)` | RW | 171 |
| 50 | 0x80f0d0 | `(unnamed)` | RW | 323 |

Not on this static list but needed on the same path: `Win::__MainWndProc` (0x747EB0, keyboard
delivery; sa-game drives the logo/title skips with DirectInput mouse clicks meanwhile), the RW D3D9
driver functions PORT-RW-4 left out (section 9.4), the Rx pipeline compiler (`RxPipelineLock`,
`RxLockedPipeAddFragment`, `RxLockedPipeUnlock` and their helpers 0x8064D0, 0x806DE0, 0x8067B0,
0x80F0D0), the custom render pipelines (`CCustomCarEnvMapPipeline::CreatePipe` and friends), and
the audio engine service (`CAudioEngine::ServiceLoadingTune` → `CAESoundManager::Service`, which the
loading screen calls every frame).

## 8. Decisions (STD-2) and decisions needed

Made by the main session (2026-11-11) and implemented: phase B MVP is a 32-bit process; the arena
is filled from the user's exe at start-up; `globals!` alias the arena; `orig!` has an
address-keyed registry fed by `#[hook]`; the boot is replayed in WinMain order.

Decided by the main session on STD-2's questions (2026-11-11, implemented by STD-3):

1. **F10 stays as written** (the final phase B has no gta_sa.exe dependency). **MVP-only
   dependency:** the MVP reads `.rdata`/`.data`/`.text` from the user's exe at start-up (the
   arena, section 6, and the import call sites). Plan: the data the ports read by address moves
   into `globals!` declarations (whose initial values are already checked against the image,
   1,119 so far) and engine-owned tables, after which the arena is filled from those
   declarations instead of the exe, and the `site` import resolution uses recorded call-site →
   import tables; then the exe read is removed before F10 is attempted.
2. **`#[native(addr, conv, name)]`** (sa-macros) emits only the phase B registry entry
   (`sa_core::abi::NativeReg`), listed per crate (`hooks::NATIVES`, or `hooks::NATIVE_MODULES` for
   crates with several module lists) and collected by `sa_ports::natives()`. `unhookable.rs` is
   gone; its six ports carry `#[native]` now.
3. **Platform values not measured on phase A** stay as chosen (Wine's defaults where Wine has
   one), documented: `GetVersionExA` Windows 10 (`OSVer = 4`), mouse 3 axes / 5 buttons,
   `GetCommandLineA` the program name only, and (STD-3, `dx.rs`) every probed DirectX DLL
   present, 256 MiB video memory all free, `DirectSoundCreate` succeeds, one adapter, desktop
   1920x1080x32 @ 60 Hz, adapter modes 640x480 … 1920x1080 @ 60 Hz in X8R8G8B8 and R5G6B5,
   no multisampling, `D3DCAPS9` of a fixed-function HAL device (no vertex/pixel shaders,
   section 9).

**D3D9 shaders** (STD-GFX, docs/arch/gpu.md): the game's custom pipelines are fixed function;
its shaders are RW's run-time generated vertex shaders (assembled by D3DX) and RpMatFX's six
ps_1_1 blobs. They are translated from bytecode to WGSL by `sa_rw_wgpu::sm` and run on
`WgpuDevice`; `D3DXAssembleShader` is native. Still open: the caps report shaders only with
`SA_GAME_SHADERS=1` until RW's generator (0x75F0B0) is ported.

## 9. STD-3: RW engine bring-up and the loading screen

### 9.1 `#[native]`

See decision 2 above. `mvp_gaps.py` counts `#[native(addr = …)]` (any path prefix) as Rust, and
`crt.rs`/`com.rs`/`dx.rs`/`rw_standin.rs` registrations as `native`.

### 9.2 RW engine life cycle (sa-rwcore `engine.rs`)

Ported from the 1.0 US bytes: `_rwDeviceSystemRequest`, `RwEngineInit`/`Open`/`Start`/`Stop`/
`Close`/`Term`, the subsystem and video-mode queries, the error module and `RwErrorSet`/`Get`,
`_rwerror`, `_rwStringOpen` (the CRT string table), `_rwFileSystemOpen` (the CRT file table),
`_rwPipeAttachDummy`, `_rwGetNumEngineInstances`, the colour/pipe/chunk-group module callbacks;
`RwFreeListCreateAndPreallocateSpace` (memory.rs); `Ps::psSelectDevice` (sa-core video.rs,
following the 1.0 US bytes: the `rdtsc` store and the inlined default-mode search differ from
gta-reversed). The function tables the engine stores (string, file, standard functions, plugin
callbacks) hold original addresses; calls through them dispatch by address, so `crt.rs` now also
serves the RW string/file tables (`strcpy`, `strcat`, `strcmp`, the `rwstr*` wrappers, `fopen`…
`ftell`, `rwfexist`) and `pow`.

### 9.3 Native `IDirect3DDevice9` (`crates/sa-game/src/d3d9.rs`, `dx.rs`)

PORT-RW-4 (merged by STD-3) ports the RW D3D9 driver layer; it calls the device through `orig!`
`vslot` declarations exactly as the original. Phase B serves those calls with native COM objects:
`IDirect3D9` (`dx.rs`: adapter count, modes, display mode, caps, `CheckDeviceFormat`,
`CheckDeviceMultiSampleType`) and `IDirect3DDevice9` plus its resources (`d3d9.rs`: textures,
surfaces, vertex/index buffers, vertex declarations as heap objects `{vtable, handle}`; buffers and
texture levels keep a CPU copy, `Lock` hands it out, `Unlock` uploads the locked bytes). Every
device method translates to the `D3d9Device` seam on the stand-in's `WgpuDevice`, so the D3D9 call
stream of the ported driver reaches wgpu unchanged. Implemented so far: `TestCooperativeLevel`,
`GetAvailableTextureMem`, `GetDeviceCaps`, `Present` (+ `--present-dir`), `GetBackBuffer`,
`CreateTexture`, `CreateVertexBuffer`, `CreateIndexBuffer`, `Set/GetRenderTarget`,
`Set/GetDepthStencilSurface`, `Begin/EndScene`, `Clear` (whole viewport), `SetTransform`,
`SetViewport`, `SetMaterial`, `SetLight`, `LightEnable`, `SetClipPlane` (accepted, not modelled),
`Set/GetRenderState`, `SetTexture`, `SetTextureStageState`, `SetSamplerState`, palette calls
(accepted; P8 is not available), the four draw calls, vertex declarations, `SetFVF`, shaders
(creation fails: the caps report none; setting NULL works), `SetStreamSource`, `SetIndices`;
texture `GetLevelCount`/`GetLevelDesc`/`GetSurfaceLevel`/`LockRect`/`UnlockRect`, surface
`GetDesc`/`LockRect`/`UnlockRect`, buffer `Lock`/`Unlock`/`GetDesc`, `AddRef`/`Release`.

STD-4 added: `IDirect3D9::CreateDevice` (the device: a headless `WgpuDevice` of the present
parameters' back-buffer size, the window's client size for 0; the swap chain reports the
parameters' back-buffer and auto depth-stencil formats; behaviour flags not modelled; no adapter:
NOTAVAILABLE), `GetAdapterIdentifier` (Driver "sa-engine", Description "sa-engine wgpu"),
`CheckDeviceType` (X8R8G8B8 display with X8R8G8B8/A8R8G8B8 back buffer, R5G6B5 with R5G6B5),
`CheckDepthStencilMatch` (every depth format `CheckDeviceFormat` accepts); device
`EvictManagedResources` (no-op), `Reset` (back buffer resized; the same size keeps it, its
contents being undefined after a D3D9 reset), `CreateCubeTexture` (the seam's NotAvailable),
`CreateRenderTarget` / `CreateDepthStencilSurface` (surfaces on their own seam texture, usable as
render target / depth), `CreateOffscreenPlainSurface` (system-memory bytes, locked in place),
`GetRenderTargetData` (read-back converted to A8R8G8B8/X8R8G8B8/R5G6B5/X1R5G5B5/A1R5G5B5),
`UpdateSurface` (system memory → texture level of the same format), `StretchRect` (seam
`stretch_rect`); surfaces and level descs report their usage and pool (RW's camera begin update
tests RENDERTARGET); USER32 `IsIconic` (0: the native window is never minimised). The device and
`--present-dir` moved from `rw_standin.rs` to `d3d9.rs` (`Present`). Not modelled: multisampling,
additional swap chains (RW's `RwD3D9CameraAttachWindow` path), cube textures, lockable render
targets.

### 9.4 RW driver stand-ins (`crates/sa-game/src/rw_standin.rs`, temporary)

Functions PORT-RW-4 left out that the boot reaches; registered only where no `#[hook]`/`#[native]`
port exists (ports win), each to be replaced by a port:

* (STD-4: the `_rwD3D9System`, raster create / destroy, camera begin update / clear, raster
  show and VS-cache-reset stand-ins are replaced by ports, section 9.7.)
* RpSkin/RpMatFX object callbacks (no-data cases), `RpAnisotPluginAttach` and its engine
  callbacks. (The Rx pipeline compiler, the node-definition getters and the RpSkin/RpMatFX engine
  Open/Close callbacks are ported now: sa-rwcore `rxcompile.rs`, `skin.rs`, `matfx.rs`, batches
  `std4-rxpipe-1`/`-2`; the custom game pipelines in sa-render `custompipes.rs`, batch
  `std4-rxpipe-3`.)

`--present-dir DIR` writes every presented frame to `DIR/present-NNNN.png` (at most 200; keep
it under `target/`, never commit it: the frames contain the user's game assets).

### 9.5 Result (2026-11-11, i686, the user's install)

`sa-game --game wine/gta-sa --frames 80 --present-dir target/std3-present` (no forked probes once
a GPU device exists: `SA_GAME_NO_PROBE=1`, forking a process with wgpu threads is unsafe):

| WinMain step | result |
|---|---|
| `SystemParametersInfoA` + `Win::IsAlreadyRunning` | ok |
| `RsEventHandler(rsINITIALIZE)` (CText/GXT loaders, psInitialize incl. the DirectX probes, `CMenuManager::LoadSettings`) | ok |
| `Win32_InitApplication`, command line, `Win32_InitInstance`, controls, DirectInput, mouse set-up | ok |
| `RsEventHandler(rsRWINITIALIZE)` → `RwEngineInit` (core modules), `App::PluginAttach` (world, skin, anim, HAnim, node name, visibility, anim blend, TXD store, MatFX, UVAnim, building/car pipelines, anisotropy, breakable, collision, 2D effects, pipeline), `RwEngineOpen`, `psSelectDevice` (800x600x32 exclusive), `RwEngineStart` (device on the seam, every engine module open), `CGame::InitialiseRenderWare` (TXD store, camera, `RpWorldCreate`, lights, fonts, HUD, post effects) | ok |
| fullscreen style, `CameraSize` (800x600), `ShowWindow`, `gta_sa.set`, `CGamma::Init` (device caps) | ok |
| CdStream check (gta3.img) | ok |
| main loop INITIAL: `LoadSplashes` (loadscs.txd through the ported native texture reader), `LoadingScreen` init, `DoPCTitleFadeIn`/`FadeOut` | ok: **the splash is drawn through the wgpu seam** (CSprite2d → RwIm2D → PORT-RW-4 Im2D → native `IDirect3DDevice9` → `WgpuDevice`), 200 presented frames written to `target/std3-present/` |
| LOGO → PLAYING_LOGO (no movie: `CoCreateInstance` fails natively), skip by input | ok |
| TITLE → PLAYING_INTRO, skip | ok |
| FRONTEND_LOADING: `InitialiseEssentialsAfterRW` / `InitialiseCoreDataAfterRW` | **blocked: `CCarFXRenderer::Initialise` (0x5D5AC0)** → `CCustomCarEnvMapPipeline::CreatePipe` (custom Rx pipelines) |

Summary (`--frames 80`): 30 steps ok, 3 blocked (the same blocker), 0 crashed; ~774,000 platform
calls served natively.

### 9.6 Verification queue

The ports STD-3 and its helpers wrote (or registered: PORT-RW-4's four unhooked raster functions
now carry `#[native]`) are listed in `docs/arch/std3-ports.csv` (address, catalogue name,
`hook`/`native`), 221 rows. All are written, not verified: the `hook` rows go to the phase A L1/L2
queue; the `native` rows need another phase A strategy (sa-hook cannot patch them).

### 9.7 STD-4: the RW D3D9 driver ports

The `rw_standin.rs` driver stand-ins are replaced by ports of the 1.0 US bytes (all unverified):

* sa-rwcore `d3d9sys.rs` (batch `std4-d3d9-1`, plus the VS cache reset in `std4-d3d9-2`):
  `_rwD3D9System` 0x7F5F70 (every request: OPEN, CLOSE, START through `IDirect3D9::CreateDevice`
  and the post-creation part, STOP 0x7F7100, REGISTER, mode queries, USEMODE, FOCUS, STANDARDS
  0x7F72E0, texture memory, sub-systems, FINALIZESTART, max texture size, request 0x16), the mode
  list 0x7F7540, `D3D9SetPresentParameters` 0x7F6CB0, the multisample probe 0x7F6BF0, the cache
  resets 0x7F6B00 / 0x7F6B40, `D3D9DeviceReleaseVideoMemory` 0x7F7F70, the CPU probes
  0x80DBE0 / 0x80DC40 / 0x80DCA0 / 0x80DD00 (CPUID through `sa_core::abi::game_fpu::cpuid`), the
  x87 processor functions 0x857404 / 0x85742B / 0x857432, `_rwX87MatrixMultiply` 0x8572A0 (the
  multiply FINALIZESTART installs), `_rpD3D9VertexShaderCacheReset` 0x760CF0;
  `_rwD3D9StandardFuncNotImplemented` 0x7F7530 is `#[native]` (3 bytes).
* sa-rwcore `d3d9rast.rs` (`std4-d3d9-2`): `_rwD3D9RasterCreate` 0x4CCE60, `_rwD3D9RasterDestroy`
  0x4CBB00, `_rxD3D9VideoMemoryRasterListRestore` 0x4CC970, the protected auto-mipmap check
  0x4CBF80; `#[native]` (a register argument, modelled as a cdecl argument): the palette index
  release 0x4CB730. The format check 0x4CC5C0, texture 0x4CB7C0, camera texture 0x4CB9C0, Z buffer
  0x4CCD70 and `_rxD3D9VideoMemoryRasterListRemove` 0x4CB560 were `#[native]` too; since WRITER-REGCONV
  they are `#[hook(conv = reg(...))]` (batch `regconv-rw-1`; phase B entry as for every hook).
* sa-rwcore `d3d9cam.rs` (`std4-d3d9-2`): `_rwD3D9CameraBeginUpdate` 0x7F8F20,
  `_rwD3D9CameraClear` 0x7F7730, `_rwD3D9RasterShowRaster` 0x7F99B0 (the device reset paths share
  the existing post-reset sequence).
* sa-core `game.rs` (`std4-d3d9-3`): `D3DResourceSystem::DestroyTexture` 0x730B70 and
  `D3DTextureBuffer::Setup` / `Push` / `Resize` / `PushWithoutIncreasingCounter` (0x72FE80,
  0x72FFF0, 0x730020, 0x730AD0), reached by the raster destroy of textures.

The game's x87 control word and MXCSR (`_rwProcessorInitialize`, FINALIZESTART's FTZ, STOP) go
through `sa_core::abi::game_fpu`: the real registers in phase A (MXCSR: the thread's game MXCSR
inside a hook guard), cells in phases B/C, so the native process keeps the host's FPU state (glibc's
x87 libm and the GPU driver run on the game thread).

Result (`--frames 80 --present-dir`): the boot reaches the same point (gGameState
FRONTEND_LOADING, blocked on `CCarFXRenderer::Initialise`), 30 steps ok, and all 200 presented
frames are byte-identical to the stand-in build's. The first `RwRasterShowRaster` with
rwRASTERFLIPWAITVSYNC now resets the device as the original does (release video memory, `Reset`,
post-reset sequence, VM raster restore).

## 10. STD-4: front end, main menu, new game

### 10.0 Static initialisers and native audio

- **C++ static initialisers** (`crates/sa-game/src/cinit.rs`). The CRT entry (0x824570) calls
  `_cinit` (0x823B76) before `WinMain`; it runs the C table `[0x8A5A14, 0x8A5A30)` (CRT internals,
  not needed natively) and the C++ table `__xc_a..__xc_z` = `[0x8A4000, 0x8A5A10)` (1,668 slots,
  NULL slots skipped). sa-game runs the C++ table as its first boot step, in table order, at the
  point the original runs it: a slot with a registered port is called, a slot without one is
  skipped and listed in the `--report` ("C++ static initialisers without a port"), a port that
  panics (unported callee) is recorded in the step's detail and the next slot runs. The
  `eh vector constructor iterator` (0x821D30) is a CRT replacement (`crt.rs`) that calls the element
  constructor through the registry. Without this step the global objects keep vtable 0 (e.g.
  `AudioEngine`'s entities, so `CAESoundManager::Service` called address 0).
- **DirectSound** (`crates/sa-game/src/dsound.rs`). `EAX.DLL!#6` (EAXDirectSoundCreate) returns a
  native IDirectSound8; buffers (primary with IDirectSound3DListener, secondary with
  IDirectSound3DBuffer) keep their data in the process and a mixer thread advances the play cursors
  in real time (5 ms period) and mixes into the sa-platform audio sink (NullAudio by default; cpal
  with `--features audio` and `SA_GAME_AUDIO=cpal`). `GetCaps` reports 64 hardware 3D buffers.
  IKsPropertySet (EAX) is not offered, so the game's EAX path reports no EAX. Not implemented:
  `SetFX` / `GetObjectInPath` (only used by the radio's FX path).
- **Audio threads**: `CreateThread` maps `CAESmoothFadeThread::SmoothFadeProc` (0x4EEE90) and
  `CAEStreamThread::MainLoop` (0x4F15C0) to their Rust ports; both run as native threads, as in the
  original, with the same `Sleep` cadence and critical sections (`imports.rs`: recursive per-thread
  critical sections keyed by the guest address).
- **Media Foundation / WMA**: `CAEWMADecoder::InitLibrary` and `CAEMFDecoder::InitLibrary` run their
  ports; the statically linked MF loader (0x8195C0) is a CRT-style replacement returning the
  original's "DLL not found" value, so user tracks in those formats are unavailable natively.


### 10.1 Decisions recorded (main session, 2026-11-11)

1. **MVP platform values** (STD-3's list: every probed DirectX DLL present, 256 MiB video memory,
   desktop 1920x1080, 60 Hz modes, no multisampling, a caps set without vertex/pixel shaders,
   Latin-1 `MultiByteToWideChar`, no intro movies) are accepted as the MVP's choices. STD-4 adds,
   on the same terms: adapter description "sa-engine wgpu", `CheckDeviceType` accepting an
   A8R8G8B8 back buffer on the X8R8G8B8 desktop, `IsIconic` 0, initial x87 control word 0x027F and
   MXCSR 0x1F80 in the phase B FPU cells, a device gamma ramp that is stored and returned but not
   applied (identity at start), `SetTimer` returning the id without generating `WM_TIMER`,
   `DefWindowProcA` 0.
2. **CRT `pow`.** The exe's `pow` (0x8220F0) does not always run x87 code: when the CRT's SSE2 flag
   (0xC9C400) is set and neither MXCSR nor the x87 control word unmasks an exception (the check
   ignores the precision bits, so D3D9's PC=24 does not matter) it jumps to the SSE2 kernel 0x829B99
   (about 350 instructions, table-driven with tables in `.rdata` from 0x8884F0); otherwise the x87
   path (0x82218D: `fyl2x`/`f2xm1`/`fscale`). On every CPU the game supports today the SSE2 kernel
   runs. A bit-exact port of that kernel is plain IEEE double arithmetic plus integer bit work
   (exactly reproducible in Rust); it is not written yet, so phase B keeps the host's `pow`
   (`crates/sa-game/src/crt.rs`). Proposed ledger text for docs/ROADMAP.md: "**CRT pow (phase B)**:
   sa-game serves `pow` (0x8220F0) with the host libm `pow` on doubles; the original uses the MSVC
   SSE2 kernel 0x829B99 (x87 0x82218D only with unmasked FP exceptions). Results can differ in the
   last ulp. Users: `CGamma::SetGamma` (ramp words after `_ftol(pow * 65535)`), the handling
   animation flags through 0x822130 (exact powers of two, unaffected), and the RW/CRT callers listed
   by `tools/native/mvp_gaps.py`. Closing: port 0x829B99 (+ 0x822130's kernel 0x829B80) bit-exact."
3. **The forked step probe** is switched off automatically once the GPU device exists
   (`crate::d3d9::with_device`): forking a process with wgpu threads is unsafe; the crash reporter
   still names the step of a fault. `SA_GAME_NO_PROBE=1` is no longer needed.

### 10.2 What changed in sa-game

* **Code bytes readable.** Game `.text`/`_rwcseg` (0x401000–0x858000) and `.HOODLUM`
  (0x1556000–0x1577000) are mapped read-only and never executable (`arena.rs`,
  `map_read_only_code`): the no-CD protection chunks read code bytes and `.HOODLUM` dwords as
  data and turn them into addresses or constants (e.g. `[[0x1560904] + [0x685002]]` = 0xC87FE0 in
  `TidyUpD3DTextures`, `[0x15608E8] - [0x156B63F] - 1` = 60.0 in the menu's item layout). With the
  bytes present every port computes in phase B what it computes in phase A; a call or jump into
  these ranges still faults. Same MVP-only exe dependency as the arena (decision 1 of section 8).
* **Window procedure.** `Win::__MainWndProc` (0x747EB0) is ported (sa-core `input.rs`), so key
  messages reach `RsKeyboardEventHandler` → `CPad`. Native imports added: `PostQuitMessage`,
  `DefWindowProcA`, `SetCapture`, `SetFocus`, `ReleaseCapture`, `ClipCursor`, `SetCursor`,
  `SetTimer`, `SendMessageA`, `IsIconic`. The synthetic input holds Enter for two game frames
  (key-down message at iteration 4, key-up at 7 of every 8).
* **`SA_GAME_STUB="ADDR=RET[/N],..."`** (exploration only, never a port): up to 16 addresses
  return RET without doing anything (`N` callee-popped stack dwords), so a run can look past a
  blocker another lane is porting. Printed at start-up.
* **`SA_GAME_PRESENT_SKIP=K`** skips the first K presented frames for `--present-dir` (which keeps
  writing at most 200).

### 10.3 Result (2026-11-11, i686, the user's install)

`sa-game --game wine/gta-sa --frames 30` (no `SA_GAME_STUB`, no `SA_GAME_NO_PROBE`):

| step | result |
|---|---|
| CRT `_cinit`: C++ static initialisers | ok: 750 run, 871 without a port skipped (listed in `--report`), 46 ports panicked on unported callees (listed in the step detail) |
| WinMain boot steps (as in section 9.5) | ok (`gta_sa.set` is now found: an earlier run's `SaveSettings` wrote it into the user documents folder of sa-engine) |
| CdStream check | ok |
| INITIAL → LOGO → TITLE (logo/title skipped by input) | ok |
| FRONTEND_LOADING: `InitialiseEssentialsAfterRW` (car/building pipelines, grass), `InitialiseCoreDataAfterRW` (temp col models, handling.cfg, surface infos, ped stats + decision makers, time/pop cycle, `CAudioEngine::Initialise` true, break/bone/IK managers) | ok |
| FRONTEND_LOADED → FRONTEND_IDLE: `Game::FrontendIdle` every frame: `CMenuManager::Process`, `CAudioEngine::Service`, `DrawFrontEnd` | ok: **the main menu is drawn natively** (Start Game / Options / Quit Game), keyboard Enter (window procedure → `CPad`) opens Start Game → New Game / Load Game / Delete Game |
| New Game | the frame where Enter is pressed blocks in `DrawScriptSpritesAndRectangles` (0x464980, only that frame); then `m_bStartGameLoading` → WinMain's restart path → LOADING_STARTED |
| LOADING_STARTED: `CGame::Initialise` → `Init1` → `CPlantMgr::LoadModels` | **blocked: `RpBuildMeshGenerateDefaultTriStrip` (0x7591B0)** (RW's tri-strip mesh generator, ~1,900 instructions with 0x7591D0, 0x759790, 0x75A090, 0x75A1B0, 0x75A250, 0x75A370, 0x75A660) |

Summary: 39 steps ok, 4 blocked (the frame above and the mesh generator ×3), 0 crashed. Evidence
(not committed; the frames contain the user's game assets): `target/std4-evidence/01-main-menu.png`,
`target/std4-evidence/02-start-game-submenu.png`.

MVP gap list after STD-4 (`tools/native/mvp_gaps.py`): 7,274 functions on the path, 4,752 in Rust,
**2,183 gaps** (STD-3: 2,655), 283 excluded, 56 native replacements.

### 10.4 Next gaps, in call order

1. `RpBuildMeshGenerateDefaultTriStrip` 0x7591B0 and its tree (above), then the rest of `Init1`.
2. `Init2`: `CWaterLevel::WaterLevelInitialise` (508), `CTheScripts::Init` (405 + ~15 small),
   `CEntryExitManager::PostEntryExitsCreation`/`GetEntryExitIndex`, `CShadows::Init`, `CSkidmarks::Init`,
   `CGridRef::Init`, `CClouds::Init`, `CSpecialFX::Init` (+ `CCheckpoints::Init`), `CClock::Initialise`,
   `CMovingThings::Init`, `CStats::Init` (+3), `CRopes::Init`, `CReplay::Init`, `CShopping::Init`,
   `CDraw::SetFOV`.
3. After `Init2`: `CTheScripts::StartTestScript`/`Process`, `CCamera::Process` (1,495 + `CamControl`
   3,912, collision and line-of-sight code), `Init3` (`InteriorManager_c`, `FurnitureManager_c`,
   `ProcObjectMan_c`): about 31,000 unported instructions statically.
4. `DrawScriptSpritesAndRectangles` 0x464980 (menu frame of New Game), the 46 panicking and 871
   unported static initialisers, WinMain's quit path (0x748E62).
5. Driver: cube textures (the seam has none on the fixed-function path; STD-GFX's shader path has
   them), additional swap chains / `RwD3D9CameraAttachWindow` 0x7F8D70, multisampling.
6. Shaders: the caps report no vertex/pixel shaders, so RW and the game take their fixed-function
   paths and no D3D9 shader is created on the boot path; STD-GFX (merged) translates D3D9 shaders
   to WGSL behind `SA_GAME_SHADERS=1`, off until RW's vertex-shader generator 0x75F0B0 is ported.

### 10.5 Verification queue

`docs/arch/std4-ports.csv` lists every port STD-4 and its helpers wrote (address, catalogue name,
`hook`/`native`, batch, file; regenerate with `tools/native/std4_ports.py 919673c`): 270 rows, all
written, none verified. Hook batches: `std4-rxpipe-1..3`, `std4-d3d9-1..4`, `std4-core-1`,
`std4-data-1..2`, `std4-render-1`, `std4-win-1`, `std4-audio-1..3`, `std4-load-1..2` (each ≤ 60).

## 11. STD-5: from New Game into `CGame::Initialise`

### 11.1 Decisions (main session, 2026-11-11, on STD-4's questions)

1. **CRT `pow`**: phase B keeps the host `pow` for the MVP. The ledger text of section 10.1 item 2 is
   the one the main session copies to the ROADMAP ledger. STD-5 extends the same choice to the x87
   intrinsics the ports call through `sa_core::orig::x87_unary` / `x87_binary`: in phases B/C
   `_CIasin` (0x821E70), `_CIacos` (0x822380) and `_CIpow` (0x822130) are served by `libm` (the
   exe's SSE2 kernels can differ in the last bit; same closing condition: port the kernels).
2. **x87 in phase B**: accepted: the native process keeps the host FPU state; ports model x87
   precision explicitly through `sa_math::x87::X87`.
3. **Static initialisers**: a partial table is acceptable for the MVP; the `--report` list stays;
   the initialisers that panic are ported when they are on the boot path. STD-5 ported every one
   that panicked (37 → 0): the model-info store element constructors, `CColModel`/`CBoundingBox`,
   `CPad`, the LOD lists, the interior / furniture / procedural-object managers, `List_c`,
   `ListItem_c`, the formation offsets and the trivial ones (`sa-core` `object_ctors.rs`).
4. PORT-AUDIO-3 / PORT-HUD-3 hooks without batches: handled by the main session.
5. **`CdStreamOpen` slot 7**: kept, documented, until measured.
6. **Documents folder**: each `sa-game` run uses a fresh temporary documents folder
   (`$TMPDIR/sa-game-documents-PID`, removed at exit); `--documents DIR` or
   `--persistent-documents` (`$XDG_DATA_HOME/sa-engine/documents`) keep one.

### 11.2 Merges

STD-4 (`worktree-agent-abba3c53bd7571d3a`) into master, then WRITER-MVP-G and WRITER-MVP-H (both
only carried MVP-D / MVP-E / MVP-F, already in master: no changes), VERIFIER-2 (Game::Idle through
its timer call sites; the `orig!` `callsite` form resolves to the port in phase B because the
`ported` arm never evaluates the address), PORT-CAM-3 (CamControl and the mode processors; its
per-hook footprint states replaced the single one, `footprint_residue_u8` now reads
`entry_residue`). The STD-4 and writer branches had ported 39 functions twice; the verified or
writer-batch copy stays hooked, the other is kept unregistered with a note (`Not registered: ...`).

### 11.3 sa-game changes

* CRT replacements added: MSVC 8 `qsort` (0x8247E0, the exe's algorithm: median of three, the
  `loguy`/`higuy` partition, selection `shortsort` for ≤ 8 elements; equal elements must end in the
  same order), the non-EH `vector constructor iterator` (0x403D60, protected entry).
* Fixed: the native `_stricmp`/`_strnicmp` compared nothing (`0..u32::MAX as isize` is an empty
  range in a 32-bit process), so every `stricmp` returned 0 (e.g. `CStreaming::ReadIniFile` took
  every line for `memory` and called `atol(NULL)`).
* `SA_GAME_PRESENT_EVERY=N` keeps every N-th presented frame (after `SA_GAME_PRESENT_SKIP`); every
  32nd frame not kept overwrites `present-last.png`.

### 11.4 Result (2026-11-11, i686, the user's install)

`sa-game --game wine/gta-sa --frames 40`:

| step | result |
|---|---|
| CRT `_cinit` | ok: 796 run, 871 without a port skipped, **0 panicked** (STD-4: 46) |
| WinMain boot, CdStream check, logo / title, front end, main menu, Start Game → New Game | ok (as STD-4; `DrawScriptSpritesAndRectangles` is ported, the New Game frame no longer blocks) |
| LOADING_STARTED: `CGame::Initialise` → `Init1`: plants (RW tri-strip mesh generation), stunt jumps, effects (`FxManager` load), models / IDE, `object.dat`, vehicle dirt textures, gta3.img directory, zones, IPL instances (buildings, dummies, static matrices), garages, entry-exits, pickups, the loading screen's progress bar | ok |
| `CPlayerInfo::LoadPlayerSkin` → `CPlayerSkin::GetSkinTexture` | **blocked: `RtBMPImageRead` (0x7CDF60, 545 instructions)** |

The frame after the block retries `CGame::Initialise` from a half-initialised state and faults (null
page); that is a consequence of the block, not a separate gap. Evidence (not committed, contains
the user's game assets): `target/std5-evidence/01-loading-screen-progress.png` and
`02-loading-screen-last-frame.png` (the loading screen's progress bar advancing through `Init1`; the
background is white: the loading-screen splash raster is not drawn yet), `00-before-new-game.png`.

MVP gap list after STD-5 (`tools/native/mvp_gaps.py`): 7,274 functions on the path, 5,504 in Rust,
**1,430 gaps** (STD-4: 2,183), 282 excluded, 58 native replacements.

### 11.5 Next gaps, in call order

1. `RtBMPImageRead` 0x7CDF60, then the rest of `Init1` (`CClothes::LoadClothesFile`, the
   `CCarEnterExit` anim offsets) and the loading-screen splash raster.
2. `Init2`: `CWaterLevel::WaterLevelInitialise` (508), `CTheScripts::Init` (405) and the script
   brain functions, `CClock::Initialise`, `CShopping::LoadStats`, the shadow / cloud / special FX
   initialisers.
3. `CTheScripts::Process`, `CCamera::Process` (PORT-CAM-3's CamControl is merged; `CCamera::Process`
   0x52B730 itself and `Process_FollowPed_SA`, left unhooked by CAM-3, need `#[native]` entries), the
   remaining `CMatrix` members (rotations, `operator*`, `Invert`, `Reorthogonalise`: 20 functions,
   ~1,300 instructions), `Init3`.
4. In world: `RpHAnimHierarchyUpdateMatrices`, the ped / skin render path, CPad → CJ.
5. A generic way to see past a block: the retry after a blocked `CGame::Initialise` should not run.

### 11.6 Verification queue

`docs/arch/std5-ports.csv` (regenerate with `tools/native/std5_ports.py f6b0d46`) lists every port
STD-5 wrote or registered: hooks in batches `std5-tristrip-1`, `std5-ctor-1`, `std5-pool-1`,
`std5-script-1`, `std5-fx-1`, `std5-rwworld-1`, `std5-rw-1`, `std5-objdata-1`, `std5-stream-1`,
`std5-zones-1`, `std5-world-1`, `std5-world-2`, `std5-math-1..3`, `std5-ped-1` (each ≤ 60), and
`#[native]` entries for protected or one-instruction entries and for existing "unhookable" ports
(model-info virtuals, streaming no-ops). All written, none verified.

## 12. STD-6: Init1..Init3, scripts and the first in-world frame (in progress)

### 12.1 Decisions (main session, 2026-11-11)

1. **`libm` for `_CIasin` / `_CIacos` / `_CIpow` in phase B** is accepted for the MVP, with a ledger
   line like the one for `pow` (section 10.1 item 2): the exe's SSE2 kernels can differ in the last
   bit; closing condition: port the kernels. STD-6 adds CRT `modf` (0x8232F0) to sa-game's CRT
   table; it is exact (no ledger line).
2. **No retry after a blocked `CGame::Initialise`**: sa-game reports the blocked function and stops
   cleanly with exit code 4 ("main loop: CGame::Initialise did not complete; stopping (no retry)").
   The game state is read through `winmain::game_state()`; the stop triggers while the state is
   still `LOADING_STARTED` (8) after the init frame.
3. **Mixed-subsystem batches** need not be split (the writer gate prints a note only).

### 12.2 Merges

STD-5 (`worktree-agent-ab3123672f7d6d5c3`), master (7fd59f2), VEH-BIG, RENDER-BIG, MVP-I,
VERIFIER-4, PORT-AUDIO-3 (its `port3_*.rs`, verdicts and fixes; `sa-macros` / `abi` conflicts
resolved in favour of the base), PORT-CAM-3 (final), WRITER-MVP-J, WRITER-RENDER-BIG-2 and PORT-HUD-3
(final head 7a3b4cf: the `UseCommonVehicleTexDicationary` constant fix, `peek`, the AX return kind).
Lists were unioned and de-duplicated; keycells `READERS` unioned by `at` (30). Where two branches
registered the same address, one registration stays (the verified / writer-batch copy) and the
other is kept unregistered with a note.

### 12.3 sa-game changes

1. Exit code 4 when `CGame::Initialise` does not complete (12.1 item 2).
2. `SA_GAME_SCRIPT_SKIP=1` (exploration only, never a port): an unported script opcode is logged
   (`script: skipped opcode XXXX (n params)`) and its parameters skipped (`CollectParameters(n)`, n
   from `catalog/script-params.csv`), so one run lists the opcodes a script path needs. The hook is
   `sa_script::cmdlib::UNPORTED_OPCODE_HANDLER` (unset = the usual panic).
3. CRT `modf` (0x8232F0) in the CRT table.
4. 183 existing ports whose docs say "not hookable" (protected `jmp` entries, entries shorter than
   5 bytes, loops into the entry bytes) are registered as phase B natives (per-crate
   `std6_natives.rs`, listed in `sa_ports::NATIVES`); the `CGeneral` math functions sa-hook hooks
   locally in phase A (`LimitRadianAngle`, `LimitAngle`, `GetRadianAngleBetweenPoints`,
   `GetATanOfXY`, `SolveQuadratic`) are natives in `sa_core::general::math_natives`;
   `CCam::Process_FollowPed_SA` (PORT-CAM-3, unhooked in phase A for its FPATAN precision) is a
   phase B native.

### 12.4 Boot progress

| step | status |
|---|---|
| `RtBMPImageRead`, `CClothes::LoadClothesFile`, `CCarEnterExit` anim offsets (Init1) | ported, pass |
| model streaming: RpSkin / RpHAnim / RpMatFX / 2dfx / collision plugin stream reads, car / building pipelines, `LoadAnimFile` | ported, pass |
| Init2: `WaterLevelInitialise`, `CTheScripts::Init` (+ `ReadObjectNamesFromScript`, switch data), `CClock` | ported, pass |
| Init3 | pass |
| `CTheScripts::Process` running main.scm (intro) | runs; opcode 07AF (`ProcessCommands1900To1999`) unported (skipped with `SA_GAME_SCRIPT_SKIP=1`) |
| `CCamera::Process` (0x52B730, new port) with `CamControl`, `CCam::Process`, `Process_FollowPed_SA` | passes |
| furniture groups, shadow camera gradient raster, `CColBox::Set`, `TransformVectors`, `RwMatrixOrthoNormalize`, `CMatrix` members | ported, pass |
| next block | `CAEGlobalWeaponAudioEntity::ServiceAmbientGunFire` 0x4DF210 (frame 24, still `LOADING_STARTED`) |

A look-ahead with exploration stubs (`SA_GAME_STUB`) shows the following blocks after it, all in the
first game frame (25): `CStreaming::StreamZoneModels` 0x40A560, `CRect::IsPointInside` 0x4042D0,
`CCover::Update` 0x6997E0, `CTheScripts` death / arrest check 0x485A50, and more not yet listed.
No frame has been presented yet: the loading-screen splash path does not reach `Present` natively
(not investigated), so `target/std6-evidence/` is empty.

### 12.5 Verification queue

`docs/arch/std6-ports.csv` (regenerate with `tools/native/std6_ports.py 7fd59f2`) lists every port
STD-6 wrote or registered: hooks in batches `std6-rw-1`, `std6-rw-2`, `std6-render-1`,
`std6-render-2`, `std6-anim-1`, `std6-script-1`, `std6-stream-1`, `std6-ped-1`, `std6-veh-1`,
`std6-math-1`, `std6-math-2`, `std6-world-1`, `std6-cam-1`, `std6-core-1`, `std6-input-1` (each
≤ 60) and `#[native]` entries. All written, none verified. Entries that are a `jmp` into game
`.text` (not a `.HOODLUM` stub) are natives (`RpAnimBlendFrameGetName` 0x4D5EF0,
`VehicleVisibilityCB_BigVehicle` 0x732AB0, `CustomCarPipeAtomicSetup` 0x5D5B20,
`RwMatrixOrthoNormalize` 0x7F1920), as are protected entries (`TransformVectors` 0x54EEA0).
`CCamera::Process` has frame slots the original leaves uninitialised on the transition path with
`fS > 1` / NaN (spec open question 2); the port reads 0 there.

### 12.6 Next gaps

1. `ServiceAmbientGunFire`, then the look-ahead list of 12.4 (`StreamZoneModels` is one of the
   functions the main session asked to be told about).
2. Script opcodes the intro needs (07AF first; run with `SA_GAME_SCRIPT_SKIP=1` to list the rest).
3. The loading-screen `Present` path, for the evidence PNGs.
4. In world: the skinned ped render path, `CPad` keyboard input → walking.
5. Remaining `CMatrix` members: `Reorthogonalise`, `ForceUpVector`, `Rotate`, `Scale` / `ScaleAll`,
   `operator*=`, `Detach`, `ConvertTo/FromEulerAngles`.
