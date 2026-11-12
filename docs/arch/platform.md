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

### 12.7 Main-session decisions on STD-6's questions (recorded by STD-7, 2026-11-11)

1. **`CCamera::Process` uninitialised slots**: the port reads 0 where the original reads stack slots it never
   wrote (the transition path with `fS > 1` or NaN). Proposed ledger text for docs/ROADMAP.md (the main
   session copies it): "**CCamera::Process uninitialised transition slots (phases A/B)**: on the camera
   transition path with `fS > 1` or a NaN `fS`, `CCamera::Process` (0x52B730) reads stack slots the original
   never writes (spec open question 2); the port reads 0 there, the original whatever its caller left. No
   recording reaches the path so far. Closing: model the slots with `peek` (ADR-001 §5b) once a recording
   exercises the path, or prove it unreachable."
2. **`jmp`-entry ports as `#[native]`**: acceptable for the MVP (a harness hook class for in-`.text` `jmp`
   entries comes later). The list: `RpAnimBlendFrameGetName` 0x4D5EF0, `VehicleVisibilityCB_BigVehicle`
   0x732AB0, `CustomCarPipeAtomicSetup` 0x5D5B20, `RwMatrixOrthoNormalize` 0x7F1920 (STD-6);
   `CMatrix::Scale` 0x459350 and `CTheZones::FindSmallestZoneForPosition` 0x572360 (STD-7, section 13).
   `TransformVectors` 0x54EEA0 is a `#[hook]` again (MVP-K's, 9-byte expect; the main session preferred the
   hook, `cargo test -p sa-hook --lib` accepts it); STD-6's native copy was removed.
3. **`ProcessAllSearchLights` 0x4939F0**: the main session's final decision keeps MVP-K's whole port
   (`script_search_lights.rs`, batch `mvpk-script-2`) registered; STD-6's partial ADR-002 port was removed
   from `the_scripts_init.rs`.

## 13. STD-7: past the first game frames (in progress)

### 13.1 Merges

STD-6 (fast-forward, head 97a6025), WRITER-MVP-K (whole branch, then its final head ab22554),
HARNESS-5, X87-2 (its hooked `Process_FollowPed_SA` replaces STD-6's native; the native was dropped from
`sa-cam/std6_natives.rs`), VERIFIER-5 (twice: the driver fixes, then the vehbig promotions with the
`CBike::ProcessControl` lean-recovery fix), and WRITER-MVP-L's own three commits (cherry-picked: its branch
carries a "WIP STD-6" snapshot that conflicts with the final STD-6 everywhere). Conflicts: hook/module lists
unioned; keycells `READERS` 30 (unchanged by HARNESS-5) then 32 with MVP-L's two; `tools/e2e/l2-extras.txt`
= ours plus the VERIFIER-5 lines; one registration per address (`CPool<CStuntJump>::New` 0x49C9E0 keeps
MVP-L's hook; the 0x4700E0 helper keeps MVP-J's hook). `cargo test -p sa-hook --lib`: 58 passed.

### 13.2 sa-game changes

* **`--present-dir` creates the directory.** `File::create` failed silently in a missing directory, so no
  PNG was written; STD-6's "no frame has been presented" was this (1,600+ frames are presented per run).
* `SA_GAME_STUB` takes up to 48 addresses (exploration only).
* `SA_GAME_SCRIPT_SKIP` reads the `reads` column of `catalog/script-params.csv` (`T` = an 8-byte text
  label, `Cn` = n parameters), so opcodes with label parameters are skipped without misaligning the script.
* `SA_GAME_SCRIPT_TRACE=1` (exploration only): every command's script name, offset and command word
  (through `sa_script::cmdlib::COMMAND_TRACE_HANDLER`).
* `SA_GAME_WALK=START:COUNT` (evidence runs): W (GO_FORWARD) pressed at iteration START and held for COUNT
  iterations; the synthetic Enter stops 8 iterations before START.

### 13.3 Boot progress (`SA_GAME_SCRIPT_SKIP=1`, i686, the user's install)

| step | status |
|---|---|
| boot, front end, New Game, `CGame::Initialise` (Init1..3), loading screen | ok (as STD-6) |
| frame 24 (`LOADING_STARTED → IDLE`): `ServiceAmbientGunFire`, `StreamZoneModels` (MVP-K), `CRect::IsPointInside`, `CCover::Update` / `AddCoverPoint`, `DoDeathArrestCheck` (the obfuscated call decoded: `[0x15604F4] ^ [0x736A72]` = `CMessages::ClearSmallMessagesOnly`, return 0x1564FAC) | ported, pass |
| main.scm start-up (`initial` script): 36 opcodes ported (list in `docs/arch/std7-ports.csv`); `IsPedDead`, `RemoveScriptFromList`, `ScriptAttachAnimGroupToCharModel`, `SetEntryExitFlag`, the pool `New` / `GetRef` thunks | the start-up block runs to its first `WAIT` |
| frame 25 game logic: `IsCoopGameGoingOn`, `FindSmallestZoneForPosition`, `GetEventGlobalGroup`, `CompressFromFullMatrix`, `EnableReplays`, 130 lone-`ret` functions, `RpAnimBlendClumpUpdateAnimations` + the skinned / off-screen / velocity-extraction frame callbacks, `ScanForCollisionEvents` (native), `ScanForEntitiesInRange` | ported, pass |
| next block | **`CRopes::RegisterRope` 0x556B40** (from `CObject::ProcessControl` of a crane magnet object, 0x8CD6BC..0x8CD6C4 models) |
| after it (other frames / scripts) | `CClothesBuilder::CreateSkinnedClump` 0x5A69D0 (CJ's clothes rebuild, script frame 27), more script opcodes |

No game frame has rendered yet: every game frame stops at the first unported callee before
`RenderScene`. The loading screens are presented (13.4).

### 13.4 The loading-screen `Present` path

Frames reach the native device (evidence `target/std7-evidence/00-boot-splash.png`,
`01-loading-screen-new-game.png`: the new-game loading screen with its artwork and progress bar).
STD-5's white loading screen is the original's behaviour on a slow load: `NewChunkLoaded` changes the
splash every 5 s of real time and `CurrDisplayedSplash` is not bounded (`DisplayNextSplash` only stops at
6); the debug build loads slowly enough to pass splash 6, and `m_aSplashes[7..]` are other globals (a sprite
without a texture draws white). A release build of sa-game should load within the six splashes.

### 13.5 Ports and verification queue

`docs/arch/std7-ports.csv` (regenerate with `tools/native/std7_ports.py 8b8890d`): 21 hooks in batches
`std7-audio-1`, `std7-math-1`, `std7-ai-1`, `std7-script-1`, `std7-core-1`, `std7-world-1`, `std7-anim-1`
(each ≤ 60), 18 `#[native]`s, 118 lone-`ret` natives (`sa_core::std7_natives::LONE_RET_NATIVES`, one shared
entry: every catalogued game-`.text` function whose body is a lone `ret` and that had no registration) and
36 script opcodes (cases in the partial switches; `cmdlib::zone_info_by_label`,
`register_script_brain_for_code_use`, `streamed_script_index_to_sp0` are shared helpers). All written,
none verified. A seam test `im2d_declaration_untextured_uses_vertex_colour` (sa-rw-wgpu) covers RW's
untextured Im2D path.

### 13.6 Divergences and ledger proposals

1. **`CCollisionEventScanner::ScanForCollisionEvents` 0x604500 registered natively** (PORT-AI-5 left it
   unregistered: the c-bug split run showed the reference pool's order differing after the call). Proposed
   ledger text: "**ScanForCollisionEvents reference order (phase B)**: the native build runs PORT-AI-5's
   port of 0x604500, whose stack events register and release entity references in a different order than
   the original (c-bug recording, frame 233: `CReferences::pEmptyList` 0xBA1768 / `aRefs` 0xB9B9A8 differ);
   the referenced entities and the events raised are the same. Closing: reproduce the original's
   construction / destruction order of the stack events, then hook it in phase A."
2. **`CEntryExitManager::SetEntryExitFlag` 0x43EF90** is not in the catalogue (no name in gta-reversed or
   the plugin SDK); registered as a native under that name.
3. **Uninitialised stack bytes** read by the original (stack `CAESound`s, text-label buffers past the label,
   `AddCoverPoint`'s reference position before the first point) are zero in the ports (`uninit-locals`
   hazard on the hooks).

### 13.7 Next gaps, in order

1. `CRopes::RegisterRope` 0x556B40 (~260 instructions, two unrolled segment-placement loops; not reversed
   in gta-reversed).
2. `CClothesBuilder::CreateSkinnedClump` 0x5A69D0 and its unported tree: `PreprocessClothesDesc` 0x5A44C0,
   the register-argument helper 0x5A42B0 (EAX/ESI/EDI/EBX: the fat / muscle blend weights; HARNESS-5's
   `reg("args = eax esi edi ebx")`), `ConstructGeometryArray` 0x5A55A0 (+ `BlendGeometry` 0x5A4940 /
   0x5A4F10, `CopyGeometry` 0x5A5340), `ConstructTextures` 0x5A6040 (+ the three `BlendTextures`,
   `PlaceTextureOnTopOfTexture`, `GetTextureFromTxdAndLoadNextTxd`), `ConstructGeometryAndSkinArrays`
   0x5A6530 (+ `StoreBoneArray`, `AddWeightToBoneVertex`, `BuildBoneIndexConversionTable`): about 3,000
   instructions. It builds CJ's clump, so it gates "CJ visible".
3. The remaining frame update callbacks as recordings reach them: non-skinned 0x4D30A0, compressed
   0x4D2E40 / 0x4D32D0, the 3D-velocity variants 0x4D1A50 / 0x4D2450 and the compressed / non-skinned
   velocity variants 0x4D1DB0 / 0x4D27F0, `CQuaternion::Get` 0x59C080.
4. Further start-up script opcodes (`SA_GAME_SCRIPT_SKIP=1` lists them), the first `RenderScene`, the
   skinned ped render path, then walking (`SA_GAME_WALK` is ready).
5. `CMatrix`: `Reorthogonalise` 0x59B6A0, `ForceUpVector` 0x59B7E0, `Rotate` 0x59B460,
   `ConvertTo/FromEulerAngles` 0x59A840 / 0x59AA40 (STD-7 ported `Detach`, `Scale`, `operator*=` and
   `CompressFromFullMatrix`).

### 13.8 Decisions needed

1. The `ScanForCollisionEvents` native (13.6 item 1): accept for the MVP with the ledger line, or keep it
   unregistered (the native game then stops on the first collision frame).
2. A release-build `sa-game` for evidence runs (loading speed, 13.4), or keep debug builds and accept the
   white loading screen past splash 6.

## 14. STD-8: the first in-game frames

### 14.1 Main-session decisions on STD-7's questions (recorded by STD-8, 2026-11-12)

1. **`ScanForCollisionEvents` 0x604500 registered natively** is accepted for the MVP, with the ledger line of 13.6
   item 1.
2. **Evidence runs use a release-build `sa-game`** (`cargo build --release -p sa-game --target
   i686-unknown-linux-gnu`); debug builds are for debugging only.
3. **The main session copies the ledger texts into the ROADMAP** (13.6, 14.6).

### 14.2 Merges

Base: STD-7 (`worktree-agent-af554962f754b69a7`, head 407b907; the branch was reset to it: master's "WIP" squash
commits duplicate STD-7's merged history). Taken as cherry-picks of the branches' own commits (each branch carries
master's squash commits, which conflict with STD-7's originals everywhere): WRITER-MVP-M (mvpm-ai-1..3,
mvpm-script-1, mvpm-render-1..6, mvpm-ped-1, mvpm-anim-2, mvpm-coll-1, mvpm-world-1; its `FrameUpdateCallBackOffscreen`
commit skipped: STD-7 registers 0x4D2E10), PORT-SCRIPT-4 (integration, host-test fix, docs), PORT-MENU (three
commits), VERIFIER-6 (DrawTriPlants srand site, render split L2, 28 promotions, water-pass re-run), the srand
at_site fixes (`a6386803`, df75d3a) and WRITER-CLUMP (`a825eb3a`, cb0b3cb: the `CreateSkinnedClump` tree, batch
`clump-1`). Conflicts: hook / module lists unioned; `tools/e2e/l2-extras.txt` unioned; the generated queue files
regenerated. One registration per address: `CBoundingBox::SetMinMax` 0x470100 keeps PORT-SCRIPT-4's verified hook
(`sa-script/misc.rs`; STD-7's `std7-math-1` copy in `sa-core/rect_inside.rs` dropped). The clothes builder's
phase B dependencies: 0x5A47E0 was already on the base (MVP-I, `sa-ped/clothes.rs`); 0x7316D0 is ported here.
`cargo test -p sa-hook --lib`: 58 passed.

### 14.3 sa-game changes

* **Every C++ static initialiser runs natively.** 871 of the 1,668 `__xc_a..__xc_z` slots had no port, so
  globals like `CWeather::m_WeatherAudioEntity` kept vtable 0 and 806 float constants (`1/50`, `x * y`, `1 / x²`,
  ...) stayed 0. `tools/native/static_inits.py` classifies the initialisers by instruction shape and generates
  `crates/sa-core/src/static_inits.rs` (865 table rows: `fdiv`, `fmul`, `fsub`, `frecip_mul`, `ftol_mul`,
  `copy32`/`copy16`, constructor + `atexit`, tail constructor, `atexit` only, `eh vector constructor` + `atexit`);
  the shapes are const-generic natives in `static_init_kinds.rs`, which also holds the five one-off initialisers.
  The x87 arithmetic runs at the CRT's initial precision (`_CW_DEFAULT`: 53 bits), so f64 arithmetic plus one
  f64 → f32 rounding is exact for these shapes. Not ported: 0x8339CA (the CRT security-cookie set-up; nothing
  ported reads the cookie). The constructors they reach that had no port are natives in `trivial_ctors.rs`.
  Result: 1,666 static initialisers run, 1 skipped.
* **The frame limiter sleeps** instead of spinning: when `gGameState` is IDLE and `1000 / frameLimit` ms have not
  passed, the iteration sleeps for the rest (same frames, no busy loop), so about every second `--frames`
  iteration runs `Game::Idle`. The step detail says `rsIDLE (Game::Idle) ran` for those.
* **The synthetic menu input stops in game**: Enter and the left button (pressed every 8 iterations to get
  through the menus) are no longer sent once `gGameState` is IDLE (the left button is fire / punch: it started a
  `CTaskSimpleFight` on CJ).
* `SA_GAME_PRESENT_GAME=1`: `--present-dir` counts and writes only the frames presented inside `Game::Idle`.
* `SA_GAME_BACKTRACE=1` (exploration): every panic prints its Rust call chain (finds the caller of a blocked
  virtual / table call).
* `SA_GAME_STUB` stubs may pop up to 8 stack dwords (`ADDR=RET/N`, `N <= 8`).
* CRT `eh vector destructor iterator` (0x821E02) in the CRT table.

### 14.4 Boot progress (release build, `SA_GAME_SCRIPT_SKIP=1`, i686, the user's install)

| step | status |
|---|---|
| boot, front end, New Game, `CGame::Initialise`, loading screens | ok (as STD-7) |
| C++ static initialisers | 1,666 run, 1 skipped (14.3) |
| frame 25: `CRopes::RegisterRope` (crane magnet), `CMatrix::Reorthogonalise` / `ForceUpVector`, `ShouldStandardCameraBeProcessed`, the grass globals, `TestLineOfSight`'s static matrix (VM decoded) | ported, pass |
| frame 25 next block | **`CRope::Update` 0x557530** (~400 instructions, not reversed) for the crane magnet rope; with it stubbed (`SA_GAME_STUB=0x557530=0/0`) also **`CAEWeatherAudioEntity::UpdateParameters` 0x505A00** (~700 instructions; the city-noise / wind sounds the weather entity plays from the first frame) |
| with those two stubbed: frames 25..63 | `Find2NodesForCarCreation` (VM decoded), CJ's clump (`CreateSkinnedClump` tree of WRITER-CLUMP + `_rpTriListMeshGenerate`, the skin geometry destructor, the default-TXD slot getter), `EndGangWar` / `ReleasePedsInAttackWave` (VM decoded), the seek / partner / fight tasks, collision helpers: ported, pass; **the first `Game::Idle` frames render natively**: `RenderScene` runs, the intro text "Francis INTL. Airport, Liberty City, 1992." is presented (21–27 in-game frames per run) |
| next block (intro mission, peds around the airport) | `CEventHandler::ComputePedCollisionWithPedResponse` 0x4BDB80 (~400, not reversed), `ComputePotentialPedCollideResponse` 0x4C2610 and `ComputePedThreatResponse` 0x4C19A0 (reversed in gta-reversed), `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached` 0x5F1FA0 (~400), `CCutsceneMgr::LoadCutsceneData_preload` 0x5B05A0 (~400) |

Evidence (not committed): `target/std8-evidence/keep/01-first-game-frame-intro-text.png` (the first in-game
frame presented natively) and `02-intro-sequence-*.png` (the in-game frames of one run, both with the two
exploration stubs of row 3). CJ is not on screen yet: the intro mission's first frames are the black screen with
the location text before the airport cutscene; the walking evidence needs the frames past it.

### 14.5 Ports and verification queue

`docs/arch/std8-ports.csv` (regenerate with `tools/native/std8_ports.py`): 21 hooks in batches `std8-render-1`,
`std8-math-1`, `std8-core-1`, `std8-pop-1`, `std8-ped-1`, `std8-world-1`, `std8-ai-1`, `std8-rw-1` (each ≤ 60),
17 `#[native]`s (new ports and existing ports registered: the `CNodeAddress` constructors, `EndGangWar`,
`FindVehicleOfPlayer`, `ChooseGangOccupation` (a `jmp` to `ChooseGangPedModel`), the two speech-context
predicates, `CCamera::Get_Just_Switched_Status`, `IsThisCarBeingTargettedByAnyCrane`, the
`CTaskComplexSeekEntity<XYOffset>::CreateSubTask` entry 0x665270 (a code key: its body is shared with 0x46F1E0's
port), the trivial constructors) and 870 static-initialiser natives. Five hooks are `L1-pure` with existing
L1 specs (`CMatrix::Reorthogonalise`, `ForceUpVector`, `CVector2D::Normalise`, `CColSphere::IntersectEdge`,
`IntersectRay`; the last three delegate to sa-math's models); the writer gate's L1 step was not run (no Wine lane
in this task). The gate's hooks step passes for every `std8-*` batch; it reports PORT-SCRIPT-4's three search-light
hooks (verified on master, no `unverified`) as new on this base. All written, none verified.

### 14.6 Divergences and ledger proposals

1. **No-CD VM values decoded from their use** (phases B/C; phase A keeps the VM call where noted). Proposed ledger
   text: "**Protection-VM results decoded from their use (phases B/C)**: the no-CD VM 0xCB73E0 returns a value or
   performs a store the bytes do not show; STD-8 decoded three from the surrounding code instead of evaluating the
   VM bytecode: `TestLineOfSight`'s static `CMatrix` constructor store (`m_pAttachMatrix` 0x9659A0 = 0, as every
   other function-static matrix; phase A keeps the VM call), `Find2NodesForCarCreation`'s loop start (1, the loop's
   own pointer starts at slot 1) and `ReleasePedsInAttackWave`'s flag mask (`~0x2000`, `bClearRadarBlipOnDeath`, as
   gta-reversed's source). Closing: evaluate the VM descriptors (0x155FF68, 0x1560064, 0x155FF14) and compare."
2. **The native frame limiter sleeps** (sa-game, not a port): no game-visible difference; the original's spin
   iterations call nothing but `PeekMessageA` and the timer.
3. **Uninitialised stack words** in `Find2NodesForCarCreation`'s node slots (upper halves) and the stack
   `CEventScriptCommand` of `ReleasePedsInAttackWave` are zero in the ports (`uninit-locals` on the hooks).

### 14.7 Next gaps, in order

1. `CRope::Update` 0x557530 and `CAEWeatherAudioEntity::UpdateParameters` 0x505A00: without them no in-game frame
   completes (both stubbed in the evidence runs).
2. The intro's ped AI: `ComputePedCollisionWithPedResponse` 0x4BDB80, `ComputePotentialPedCollideResponse`
   0x4C2610, `ComputePedThreatResponse` 0x4C19A0, `ComputeEntityBoundingBoxCornersUncached` 0x5F1FA0; the airport
   cutscene: `LoadCutsceneData_preload` 0x5B05A0; then the next blockers `SA_GAME_STUB` look-ahead lists.
3. CJ on screen: the skinned ped render path (`RpSkin` D3D9 pipeline, `RpHAnimHierarchyUpdateMatrices`), the
   remaining frame-update callbacks (13.7 item 3), `CQuaternion::Get`.
4. Walking: `SA_GAME_WALK=START:COUNT` with START past the intro's first in-world frame.
5. `CMatrix`: `Rotate` 0x59B460, `ConvertTo/FromEulerAngles` 0x59A840 / 0x59AA40 (`Reorthogonalise`,
   `ForceUpVector` done).

### 14.8 Decisions needed

1. Whether evidence runs may skip the intro mission (e.g. an sa-game option that starts at the first save point
   in Ganton) to reach "CJ walking" before the airport cutscene and the intro's ped AI are ported, or whether the
   intro path is to be ported first.
2. The VM-decoded values of 14.6 item 1: accept for the MVP with the ledger line, or require the VM evaluation.

## 15. STD-9: the world on screen (`--start-in-world`)

### 15.1 Main-session decisions on STD-8's questions (recorded by STD-9, 2026-11-12)

1. **Both**: an sa-game option starts the player in the world without the intro mission (15.3), so the CJ-walking
   evidence does not wait for the intro; the intro path itself keeps being ported (WRITER-INTRO).
2. **The three no-CD VM values decoded from their use** (`TestLineOfSight`'s static matrix, `Find2NodesForCarCreation`'s
   loop start 1, `ReleasePedsInAttackWave`'s mask `~0x2000`) are accepted for the MVP with the ledger line of 14.6
   item 1.

### 15.2 Merges

Base: STD-8 (`worktree-agent-aec7f5b6ca90ad56e`, 51b8fcd; the branch was reset to it). Cherry-picked: INTEGRATE-2's
REGCONV (8d70493: STD-7's `FrameUpdateCallBackOffscreen` 0x4D2E10 in `sa-anim/src/frame_update.rs` kept over
REGCONV's `rp_anim_blend.rs` copy, which differs from integration-2.md's choice) and GEN-ALL-2 (ed288e8), regenerated
on this tree (`tools/gen/patterns.py`; the generator now also counts path-qualified `#[sa_core::native(addr` /
`#[crate::native(addr` registrations as ported, which dropped generated duplicates of `ListItem_c::ListItem_c`,
`CAESound::CAESound`, `CScriptsForBrains` and others; `tools/native/static_inits.py` re-run after it); INTEGRATE-2's
other commits (MVP-M, VERIFIER-6, SRAND-FIX, the search-light registration) were already on STD-8 and were skipped
as empty; its documentation commit 90b2042. WRITER-MVP-N's seven own commits (fb31b52..50588be); its complete
`ProcessCommands2600To2699` replaces STD-9's partial switch (std9-script-1, CREATE_USER_3D_MARKER only). The
generated queue files were regenerated. PORT-SAVE and WRITER-INTRO had no commits of their own yet.
Two duplicate registrations remain from the STD-8 base (both sides already there): 0x6F9DE0
`CPlantSurfPropMgr::GetSurfProperties` (hook in `sa-render/mvpa_render1.rs`, native in `sa-core/gameinit.rs`) and
0x407F20 `CStreaming::LoadInitialVehicles` (hook in `sa-stream/streaming.rs`, native in `gameinit.rs`); the registry
uses the hook.

### 15.3 sa-game changes

* **`--start-in-world`**: main.scm's MAIN script runs unchanged (it creates the player at Grove Street,
  `CREATE_PLAYER 0 2488.56 -1666.84 13.38`, and loads the scene there); only its `START_MISSION 2` (0x0417 at
  main.scm offset 0xEA48, the INTRO mission: the airport cutscene, the ride to Grove Street and the intro's ped scenes)
  is skipped, and the screen MAIN faded out just before it (`DO_FADE 0 0` at 0xEA3B) is faded in through the game's
  own `CCamera::Fade(0.0, FADE_IN)`, as the intro would. MAIN then continues as after the launch (`SELECT_INTERIOR
  0`, `SET_PLAYER_CONTROL on`, `RELEASE_WEATHER`, its loop). Skipped game state: everything the intro mission sets
  (its flags, the cutscene, CJ's arrival animation and the intro's peds and vehicles).
* `SA_GAME_SCRIPT_TRACE` and `--start-in-world` share one per-command callback (`native::before_command`).
* The synthetic mouse movement stops in game (it pitched the player camera at the ground, as Enter / the left button
  were stopped by STD-8).
* `IDirect3DVertexDeclaration9::GetDeclaration` (the elements given at creation plus `D3DDECL_END`).

### 15.4 Boot progress (release build, `--start-in-world`, `SA_GAME_SCRIPT_SKIP=1`, i686, the user's install)

| step | status |
|---|---|
| boot, front end, New Game, `CGame::Initialise`, loading screens, static initialisers | ok (as STD-8) |
| frames 25..28: `CRope::Update` + `UpdateWeightInRope` (std9-render-2), `CAEWeatherAudioEntity::UpdateParameters` (std9-audio-1) | ported, pass: **no stub needed** |
| main.scm `START_MISSION 2` | skipped (`--start-in-world`), screen faded in |
| in-world frames: `FindTunnelAttributesForCoors` (native), `InsertEntityIntoSortedList`, `CMatrix` To/From Euler and `Rotate`, `RxPipelineExecute` + Rx heap / packet, CREATE_USER_3D_MARKER (now MVP-N's switch), `CBox::Recalc`, `CCustomCarEnvMapPipeline::PreRenderUpdate`, `_rxD3D9Instance` (+ strip-to-list, triangle sort), `D3DResourceSystem::CreateIndexBuffer` (argument fix), the building DN render callback and instance-data draws, `SetupLightingForEntity` (native), `RenderPedCB` / `RenderAtomicWithAlpha`, the sorted-list renderers, `RenderEntity` + fading, `RenderBuffer::StartStoring`, `SetupVehicleVariables`, `CHudColours::GetIntColour` (native) | ported, pass |
| with 10 exploration stubs (below) | **in-world frames present natively**: Grove Street, CJ's house, the HUD (clock, money, health, radar) and the first help message; 79 of 120 iterations present a frame |

Exploration stubs still needed for the screens (`SA_GAME_STUB=0x7c7b90=1/0,0x80e270=1/0,0x455720=0/0,0x4f6420=0/2,0x732f30=0/0,0x5d9900=0/0,0x4a2c40=0/4,0x701780=0/0,0x733f80=0/0,0x733240=0/0`): `_rpD3D9SkinAllInOneNode` 0x7C7B90 (CJ's skin pipeline: CJ is
not drawn), `_rwD3D9ImmRenderPrimitiveNode` 0x80E270 (Im3D primitives: markers, ropes), `CPickups::DoPickUpEffects`
0x455720, `CAEVehicleAudioEntity::AddAudioEvent` 0x4F6420, `CVisibilityPlugins::RenderWeaponPedsForPC` 0x732F30, the
car env-map render callback 0x5D9900, `FxEmitterBP_c::Render` 0x4A2C40, `CPostEffects::HeatHazeFX` 0x701780, the vehicle
hi-detail render callbacks 0x733F80 / 0x733240.

Evidence (not committed): `target/std9-evidence/keep/01-grove-street-in-world-no-cj.png` (camera still pitched by the
old synthetic mouse), `02-grove-street-cj-house-native.png` (Grove Street from CJ's position, the sky drawn wrong:
yellow / red / black blocks), `03-grove-street-walk-last.png` (`SA_GAME_WALK=60:80`).

### 15.5 Walking

`SA_GAME_WALK=60:80` holds W from iteration 60: the frames advance until the ped collision responses are reached
(`ComputePotentialPedCollideResponse` 0x4C2610, `ComputeEntityBoundingBoxCornersUncached` 0x5F1FA0, WRITER-INTRO's
list) and a null-function call in the car AI (`update_car_ai_original (0x0)`, a vehicle with a null vtable slot);
CJ himself is not drawn (skin node stubbed), so the PNG sequence shows the camera moving, not CJ. No gamescope window
run was made (STD-9 ran out of time; the offscreen `--present-dir` path is unchanged).

### 15.6 Ports and verification queue

`docs/arch/std9-ports.csv` (regenerate with `tools/native/std9_ports.py`): 32 hooks in batches `std9-audio-1`,
`std9-coll-1`, `std9-math-1`, `std9-render-1..8`, `std9-rw-1..2` (each ≤ 60) and 4 `#[native]`s (0x72D9F0, 0x5D5B10,
0x553E40, 0x58FD50: protected entries whose ports existed or are thunks). `CMatrix` To/From Euler and `Rotate` are
`L1-pure` hooks over sa-math's models (existing L1 specs); `CBox::Recalc` is `L1-pure` without a spec. All written,
none verified. Writer gate: hooks PASS (since 51b8fcd, every new hook unverified, batches ≤ 60), queue regenerated;
`cargo test -p sa-hook --lib`: 58 passed; clippy -D warnings clean for the touched crates on i686-unknown-linux-gnu and
for sa-hook on i686-pc-windows-gnu. L1 / Wine steps not run (no Wine lane).

### 15.7 Divergences and ledger proposals

1. **`CRope::Update`'s stack frame is modelled** (`Frame` in `rope_update.rs`): the original reuses stack slots
   across sections (the harness pick-up's shortening reads the slot 0x2C another section wrote); the port keeps the
   frame so those reads match; slots read before any write are zero (`uninit-locals`).
2. **`RenderAtomicWithAlpha`'s alpha buffer** holds 256 entries; the original's 0x98-byte buffer overflows past 152
   materials (no SA model has that many).
3. `_CIpow` in `CRope::Update` and the thunder volume runs through `orig::x87_binary` (libm in phases B/C, STD-5's
   choice).

### 15.8 Next gaps, in order

1. CJ on screen: `_rpD3D9SkinAllInOneNode` 0x7C7B90 (~300 instructions) with the skin instancing 0x7C8D60, the
   vertex-shader render callback 0x7C8060 (or the software path: matrix setup 0x7CAA30 / 0x7CA850, the eight vertex
   blenders 0x7C9DA0..0x7CAF50, instancing 0x7C9680, render 0x7C85B0) and the lighting update 0x7C90D0.
2. The sky (clouds / sky gradient draw wrong) and the remaining stubbed render callbacks of 15.4.
3. Walking: the ped collision responses (WRITER-INTRO), the null car-AI call, then the PNG sequence of CJ walking
   and a gamescope window run for frame pacing.
4. The intro path without `--start-in-world` as WRITER-INTRO's ports land.

### 15.9 Decisions needed

1. Whether `--start-in-world` should also set what the intro's end leaves (its global flags), or stay a pure skip.
2. Whether the 0x6F9DE0 / 0x407F20 hook-vs-native duplicates of the STD-8 base are settled by dropping the natives.

## 16. STD-10: CJ on screen, the sky, walking

### 16.1 Main-session decisions on STD-9's questions (recorded by STD-10, 2026-11-12)

1. **`--start-in-world` stays a pure skip of `START_MISSION 2`** (no faked intro flags). If a missing intro flag blocks
   walking, that one flag is set with a ledger line. None was needed: CJ walks without any intro flag (16.5).
2. **Hook-vs-native duplicates**: the native is dropped where a hook covers the native path. 0x6F9DE0
   `CPlantSurfPropMgr::GetSurfProperties`: the mvpa-render-1 hook (`sa-render/mvpa_render1.rs`) has the same body as
   `sa-core/gameinit.rs`'s native; the native is dropped and gameinit's `orig!` call goes through the registry.
   0x407F20 `CStreaming::LoadInitialVehicles` (a lone `ret`, not hookable) was registered natively twice (sa-stream and
   sa-core); sa-core's copy is dropped. One registration per address.

### 16.2 Merges

Base: STD-9 (`worktree-agent-a5944be11b642b989`, 622f85e; the branch was reset to it). Cherry-picked own commits:
WRITER-INTRO 4627242, a8d686f, fa12c38, 7647d85, dd7849a (intro-ai-1..5, intro-anim-1; base STD-8, list conflicts
unioned) and WRITER-O 19c4c83, 2f6593e, 86975df (`CAEVehicleAudioEntity::AddAudioEvent` 0x4F6420, `FxEmitterBP_c::Render`
0x4A2C40 + `RenderHeatHaze` 0x4A1940, the RtQuat helpers; its MVP-N cherry-picks skipped). The generated queue files
were regenerated after each step. `cargo test -p sa-hook --lib`: 58 passed.

### 16.3 sa-game changes

* **Texture levels keep their CPU copy after `UnlockRect`** (`d3d9.rs`). The copy was removed at unlock, so a later read
  lock (`RwRasterLock` for reading) saw zeros: `CClothesBuilder`'s texture blending read black sources and CJ's clothes
  textures were black. The module doc already described the kept copy.
* **`SA_GAME_WALK` sends the W key-down.** W was marked held before `key_down`, which queues the message only for a key
  not yet held: only the key-up reached the game (STD-9's walking runs never pressed W).
* `SA_GAME_DUMP=ADDR:LEN[,...]` (exploration): every `Present` prints game memory as dwords with their f32 reading;
  `BASE>OFF>OFF` follows pointers (each `>` reads the dword so far and adds the offset).
* The SIGSEGV / SIGBUS report gives the faulting EIP and the reporter's own address (`eip - on_fault` + `nm`'s link
  address of `on_fault` locates the code in the PIE build).

### 16.4 Boot progress (release build, `--start-in-world`, `SA_GAME_SCRIPT_SKIP=1`, i686, the user's install)

| step | status |
|---|---|
| boot to in-world frames | ok (as STD-9) |
| CJ's skin pipeline: `_rpD3D9SkinAllInOneNode` + instancing / declaration / reinstancing / software render, bone matrices, SSE blenders (std10-skin-1/2) | ported, pass: **CJ drawn, posed and textured** |
| sky: `_rwD3D9ImmRenderPrimitiveNode` (std10-rw-1) | ported, pass: **the sky gradient, sun and clouds draw** (the yellow / red / black blocks were the Im3D node stubbed: no sky polygons, only the sun corona / cloud sprites over the clear colour) |
| vehicle render callbacks 0x7331E0 / 0x733240 / 0x733F80 + `GetDotProductWithCameraVector` (std10-render-1), `RenderWeaponCB` (std10-render-2) | ported, pass |
| `AddAudioEvent` 0x4F6420, `FxEmitterBP_c::Render` 0x4A2C40 (WRITER-O) | taken, pass: stubs dropped |
| walking: `CColSphere::IntersectSphere` (std10-world-1), `CEventStuckInAir::GetEventPriority` (std10-ai-1) | ported, pass |

Exploration stubs still used (`SA_GAME_STUB=0x41da30=0/0,0x5f1890=0/0,0x5ea390=0/0,0x6a2530=0/1,0x455720=0/0,0x732f30=0/0,0x5d9900=0/0,0x701780=0/0,0x6f4040=0/4`):
`CCarAI::UpdateCarAI` 0x41DA30 (its partial port falls back to the original for law enforcers and missions > 1: a null
call in phase B), `CPedGeometryAnalyser::ComputeRouteRoundSphere` 0x5F1890, `CPedAttractorPedPlacer::PlacePedAtEffect`
0x5EA390, `CAutomobile::SetRandomDamage` 0x6A2530 (thiscall, 1 argument), `CPickups::DoPickUpEffects` 0x455720,
`RenderWeaponPedsForPC` 0x732F30, the car env-map render callback 0x5D9900, `CPostEffects::HeatHazeFX` 0x701780,
`CDoor::Process` 0x6F4040 (thiscall, **4** arguments: STD-10's first stub popped none and corrupted the caller's stack in
`CAutomobile::PreRender`, the intermittent SIGSEGV at 0x6FAF2Exx).

Evidence (not committed, `target/std10-evidence/keep/`): `01-cj-visible-grove-street.png` (CJ textured, before the sky
fix), `02-cj-sky-im3d.png` (CJ and the sky), `walk/present-00{30..50}.png` and `walk-long/present-00{40..76}.png` (CJ
walking from the street to the curb in front of his house, `SA_GAME_WALK=40:200`).

### 16.5 Walking

`SA_GAME_WALK=40:200` (W from iteration 40): CJ runs from the Grove Street cul-de-sac to the pavement in front of his
house (about 75 presented frames). The next blockers: `CTaskSimpleInAir::ProcessPed` 0x680600 (stepping off the
kerb; ~350 instructions, reversed in gta-reversed), `CPedAttractorManager::IsApproachable` 0x5EA220 and the
`CEventAttractor` constructor 0x4AF350 (ambient peds), then the stubbed functions above.

No gamescope window run: `sa-game --window` still shows STD-2's status screen (`screen.rs`); the game loop runs only in
the offscreen `--frames` mode. A windowed game loop (the seam's frames presented to the winit surface, real input) is
the missing piece for the frame-pacing check.

### 16.6 Ports and verification queue

`docs/arch/std10-ports.csv` (regenerate with `tools/native/std10_ports.py`): 21 hooks in batches `std10-skin-1` (7),
`std10-skin-2` (6), `std10-rw-1` (1), `std10-render-1` (4), `std10-render-2` (1), `std10-world-1` (1), `std10-ai-1` (1).
All written, none verified. Writer gate: hooks PASS (79 new hooks since 622f85e incl. WRITER-INTRO's, all unverified,
batches ≤ 60), queue regenerated; `cargo test -p sa-hook --lib` 58 passed; clippy -D warnings clean for sa-rwcore,
sa-render, sa-world, sa-ai, sa-core, sa-game on i686-unknown-linux-gnu and for sa-hook on i686-pc-windows-gnu. Not run:
L1 / Wine steps (no Wine lane).

Not ported in the skin path: the x87 (non-SSE) branch of the node (`_rpSkinPrepareMatrices` 0x7CA850 and the blenders
0x7C9DA0 / 0x7CA410 / 0x7CA330 / 0x7CA240: taken only when `_rwIntelSSEsupported` is 0) and the vertex-shader render
step 0x7C8060 / instancing (taken only with vertex-shader caps, which sa-game does not report).

### 16.7 Divergences and ledger proposals

1. **`CTimeCycle::Initialise`'s directional multiplier is an uninitialised local** (0x5BBAC0: the 1.0 US `timecyc.dat`
   has 51 values per line; the 52nd `%f` (directional mult) is never scanned, so `m_nDirectionalMult` is the
   byte of whatever the stack slot [esp+0xCC] held, times 100). The port's local is 0: `m_fIllumination` is 0, the
   directional light of peds and vehicles is black, CJ is lit by the ambient light only. Proposed ledger text:
   "**timecyc directional multiplier (phases B/C)**: the original reads an uninitialised stack float for
   `m_nDirectionalMult` (timecyc.dat has no such column); the port uses 0 (no directional light on peds / vehicles).
   Closing: measure the original's `m_nDirectionalMult` (0xB79FD8, 184 bytes) in a Wine run and use that value."
   With the bytes forced to 100 (exploration) CJ gets the expected directional shading.
2. The Im3D node's primitive count for a primitive type outside 2..6 is the original's uninitialised local (0 in the
   port; `uninit-locals` on the hook).
3. `_rpD3D9SkinGeometryReinstance`'s element buffer holds 65 elements (the original's 18; `GetDeclaration` writes at
   most 15 here).

### 16.8 Next gaps, in order

1. `CTaskSimpleInAir::ProcessPed` 0x680600 (walking off the kerb), `CPedGeometryAnalyser::ComputeRouteRoundSphere`
   0x5F1890, `CPedAttractorManager::IsApproachable` 0x5EA220, `CEventAttractor` ctor 0x4AF350,
   `PlacePedAtEffect` 0x5EA390.
2. `CCarAI::UpdateCarAI` 0x41DA30 complete (law enforcers, missions > 1), `CAutomobile::SetRandomDamage` 0x6A2530,
   `CDoor::Process` 0x6F4040 (entry through the protection chunk 0x407DD5).
3. The remaining render stubs: `RenderWeaponPedsForPC` 0x732F30, `CCustomCarEnvMapPipeline::CustomPipeRenderCB`
   0x5D9900, `CPostEffects::HeatHazeFX` 0x701780, `CPickups::DoPickUpEffects` 0x455720; the five FrameUpdateCallBack
   variants (0x4D2E40, 0x4D1DB0, 0x4D1A50, 0x4D30A0, 0x4D2450).
4. A windowed game loop in sa-game, then the gamescope frame-pacing run.
5. The measured directional multiplier (16.7 item 1).
6. Visual leftovers: a near palm tree drawn black (top left of the in-world frames), a grey Im3D triangle at the
   bottom right (likely a marker or shadow polygon; not investigated).

### 16.9 Decisions needed

1. The directional multiplier (16.7 item 1): measure it in a Wine run (needs a Wine lane), or accept another value
   (e.g. 100, `(uint8)1.0f * 100`, which the SA mobile data implies) with the ledger line.
2. Whether STD-11 builds the windowed game loop in sa-game (needed for the gamescope frame-pacing evidence) or that
   goes to a platform lane.

## 17. STD-11: the windowed game loop, walking past the kerb

### 17.1 Main-session decisions on STD-10's questions (recorded by STD-11, 2026-11-12)

1. **Directional light: measured in the original.** Measured value: **0** (17.4); the port's 0 already reproduces it,
   no code change.
2. **STD-11 builds the windowed game loop** (17.3).

### 17.2 Base

Rebased (main session's request, mid-task) onto master ca917d6 (INTEGRATE-3: STD-7/8/9, PORT-MENU, CLUMP, MVP-N,
WRITER-INTRO, GEN-ALL-2). STD-10's own commits were cherry-picked onto it (STD-10's WRITER-INTRO picks are already on
master and were left out); WRITER-O's three commits (`writero-audio-1`, `writero-render-1`, `writero-rw-1`) came along
with STD-10's branch. At each conflict the generated queue files were taken from master; they were regenerated at the
end. WRITER-P had no commits of its own yet. The pre-rebase branch is kept locally as `std11-pre-rebase`.

### 17.3 sa-game: the windowed game loop (`--window`)

* `sa-game --window` runs the real boot and main loop in a winit window (`game_window.rs`). The window and its wgpu
  device / surface exist before the boot (as `WinMain` creates its window before RW starts); `IDirect3D9::CreateDevice`
  makes the seam device on that GPU device (`d3d9::set_window`), and every `IDirect3DDevice9::Present` draws the back
  buffer into the surface and presents it (`AutoNoVsync`: the game's frame limiter paces, not the vertical blank).
* Every event-loop redraw runs one main-loop iteration (one window message or one `gGameState` step), as the offscreen
  mode does. Input: the window's keyboard and mouse events are queued as window messages (the window procedure feeds
  `CPad`); its held keys / buttons and relative motion are the DirectInput state. `--auto-input` adds the offscreen
  mode's synthetic input (Enter through the menus, `SA_GAME_WALK`) for unattended evidence runs.
* Frame pacing is the original's frame limiter (`winmain.rs` state 9, 0x748D81: `Idle` only after `1000 / frameLimit`
  ms since the last `CTimer::Update`; the iterations in between sleep the rest of the interval).
* The forked probe of the boot steps is off in window mode (`boot::NO_PROBE`): the display connection and the GPU device
  exist before the boot.
* `--frame-times FILE` writes the intervals between in-game presents (ms); the report and stdout give count, mean,
  median, p95, p99, max. `--present-dir` works in window mode too; `Present` now reads the back buffer back only when
  `--present-dir` is given (it read it back on every frame before).
* `--exit-after S` counts from the end of the boot; `--frames N` limits the iterations in a window too.
* STD-2's status-screen window is gone (`screen.rs` keeps only `--screenshot`).
* **32-bit X11 client libraries**: winit's X11 backend loads libXcursor, libXrandr, libXi, libXinerama, libXrender,
  libxkbcommon and libxkbcommon-x11; the system has no 32-bit copies of them (only lib32-libx11 / libxcb). For the
  evidence runs the Arch multilib packages (lib32-libxcursor 1.2.3, lib32-libxrandr 1.5.5, lib32-libxi 1.8.3,
  lib32-libxkbcommon(-x11) 1.13.2, lib32-libxinerama 1.1.6, lib32-libxrender 0.9.12; checksums verified against the
  pacman sync database) were extracted under `target/std11-libs/` and put on `LD_LIBRARY_PATH`; nothing was installed
  (decision 17.9 item 1).

Verification: gamescope headless only (800x600, its own `setsid` process group), `gamescopectl screenshot` for the
window's content, `xdotool keydown w` / `keyup w` on the gamescope display for real key input (342 WM_KEYDOWN and 337
WM_CHAR messages dispatched in the first run: the key path works).

| run | what | in-game present intervals |
|---|---|---|
| win4: window, no PNG capture, W held 20 s | frame pacing | 202 intervals, mean 41.12 ms (24.3 fps), min 25.01, median 39.58, p95 49.50, p99 81.60, max 155.76 ms (limiter: 25 fps = 40 ms) |
| win3: window + `--present-dir` (PNG read-back and encode per frame) | the walking video | 303 intervals, mean 92.29 ms (10.8 fps) |
| offscreen + `--present-dir` | as STD-10 | 112..185 ms mean (the PNG encode dominates) |

In win4 most later iterations block on `CBike::ProcessRiderAnims` 0x6B7280 (a bike appears in the real-time run; its
710 blocked iterations present nothing), so its intervals cover the first ~8 s of in-game time.

### 17.4 The directional multiplier, measured

Original under Wine: lane 3 (flock), `sa-launcher` + the lane's `sa_hook.dll` with no ports selected (`SA_HOOK_PORTS`,
`SA_HOOK_MODE`, `SA_L2`, `SA_UNVERIFIED` unset: only the Core observer hooks, none on the path of `CTimeCycle::Initialise`'s
caller up to the call), headless gamescope, New Game, `peek 0xB79FD8 184` through the control channel in `gGameState` 9
(script kept in the agent's scratch directory, not committed). Two runs: **all 184 bytes 0** (r3, r4). Arrays the same
loop fills are populated (`m_nWaterFogAlpha` 0xB7A090, `m_nHighLightMinIntensity` 0xB7A148 non-zero), so the table had
been read. The slot `[esp+0xCC]` of 0x5BBAC0 is caller esp − 0x1C at the call 0x53BD6B in 0x53BCF0: the frame of the
sibling call just before it (0x44E4E0 on 0x96F050) decides the residue. The port's local is 0: it matches; no change.
Proposed ledger line: "**timecyc directional multiplier**: the original reads an uninitialised stack float for
`m_nDirectionalMult`; measured 0 under Wine (two runs); the port uses 0. The residue comes from the frame of the call
0x44E4E0 before `CTimeCycle::Initialise`; measured under Wine only." The directional light of peds and vehicles is
therefore black in the original too; STD-10's forced 100 was exploration only.

### 17.5 Boot progress (release build, `--start-in-world`, `SA_GAME_SCRIPT_SKIP=1`, i686, the user's install)

| step | status |
|---|---|
| boot to in-world frames, CJ, sky | ok (as STD-10, now on master ca917d6) |
| walking off the kerb: `CTaskSimpleInAir::ProcessPed` (with its part behind the protection chunk 0x404880), `DeleteAnimCB`, the empty 0x678E80, `CTaskComplexStuckInAir::CreateSubTask` / `ControlSubTask`, `CTaskComplexFallAndGetUp::MakeAbortable`, the `CEventStuckInAir` / `CEventObjectCollision` / wander-prostitute destructors | ported, pass |
| ambient peds at attractors: `IsApproachable`, `CEventAttractor` ctor / dtor / `AffectsPed` / `IsEffectActive`, `CPedIntelligence::ProcessEventHandler`, `CTaskComplexUseEffect` first / next / control sub-task and destructor | ported, pass |
| `CQuaternion::Get(RwMatrix*)`, `CGeneral::GetNodeHeadingFromVector` | ported |
| **CJ walks from the street over the kerb and the lawn to his garage door**, offscreen and in the window with a real (xdotool) W key | with the exploration stubs below |

Exploration stubs used for the evidence (`SA_GAME_STUB=0x41da30=0/0,0x5f1890=0/0,0x5ea390=0/0,0x6a2530=0/1,0x455720=0/0,0x732f30=0/0,0x5d9900=0/0,0x701780=0/0,0x6f4040=0/4,0x5efca0=0/4,0x602590=0/2,0x5f3dd0=0/0`):
STD-10's nine plus `CPedAttractorManager::RegisterPedWithAttractor` 0x5EFCA0 (thiscall, 4 arguments: no attractor, so
`CTaskComplexUseEffect` falls back to `CTaskSimpleNone`), `CInterestingEvents::Add` 0x602590 (thiscall, 2) and
`CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox` (6-argument overload) 0x5F3DD0 (cdecl, ~1,900
instructions, not ported). Without 0x5F3DD0's stub 440 of 1,888 window iterations block on it.

Evidence (not committed, `target/std11-evidence/`): `win3/cj-walk-window.mp4` (25 fps video of the 201 in-game frames
of the window run: CJ walking to his garage), `win3/gs-{09,16,22,29}.png` (gamescope screenshots of the window during
the walk), `off10/present-0{040,080,120,160}.png` (offscreen walk past the kerb), `win4/frame-times.txt` (pacing),
`light-r3/`, `light-r4/` (the measurement runs; `in-game.png` is the original's frame).

### 17.6 Ports and verification queue

`docs/arch/std11-ports.csv` (regenerate with `tools/native/std11_ports.py`): 20 hooks in batches `std11-ai-1` (19) and
`std11-anim-1` (1), and one `#[native]` (0x678E80, a lone `ret 4`; `sa_ai::std11_natives`). All written, none verified.
0x4ACDD0 (entry `push -1; jmp` into the protection chunk 0x406A96) is hooked with the 7-byte expect.

### 17.7 Divergences and ledger proposals

1. 17.4 (directional multiplier, measured 0).
2. `CTaskSimpleInAir::ProcessPed` and `CTaskComplexStuckInAir::CreateSubTask` / `ControlSubTask` store into their own
   argument slots (dead afterwards; `stack-residue`); `ProcessPed` reads the uninitialised colpoint z when the ped is
   already standing (the outcome does not depend on it; the port's colpoint is zeroed: `uninit-locals`).
3. AL-returning ports leave EAX's upper bytes (status word / callee residue) unreproduced (`ax-al-return`).

### 17.8 Next gaps, in order

1. `ComputeRouteRoundEntityBoundingBox` 0x5F3DD0 (~1,900 instructions) and `ComputeRouteRoundSphere` 0x5F1890; then
   `PlacePedAtEffect` 0x5EA390.
2. The attractor registry: `RegisterPedWithAttractor` 0x5EFCA0 (decoded: a 10-way dispatch on the effect type into
   `RegisterPed` 0x5EF980 with the manager's per-type arrays at +0x10 for type 0, +0 for type 1, +0x10·type otherwise;
   not ported because `RegisterPed` and the attractor classes follow), `DeRegisterPed` 0x5EC850, 0x5EBF50,
   `CPedAttractor::GetTaskForPed` 0x5EC500; `CInterestingEvents::Add` 0x602590 (protection chunk 0x401F31).
3. `CBike::ProcessRiderAnims` 0x6B7280 (~3.3 KB, unreversed in gta-reversed): blocks every real-time window run once a
   bike spawns.
4. `CDoor::Process` 0x6F4040: its entry chunk 0x407DD5 is obfuscated (`push ecx; pushf; clc; mov ecx, [0x1560894];
   sbb ...`); `CCarAI::UpdateCarAI` complete, `SetRandomDamage` 0x6A2530.
5. The render stubs (`RenderWeaponPedsForPC`, the car env-map callback, `HeatHazeFX`, `DoPickUpEffects`) and the five
   `FrameUpdateCallBack` variants: not started.
6. The last offscreen run (off11, 300 iterations, W from 40: 308 ok, 8 blocked): `CTaskComplexChat` ctor 0x682CB0 (5),
   0x41C050 (`car_has_reason_to_stop`, 2), `CGarages` 0x449D10 (`tidy_up_garage_close`, 1, near CJ's garage).
7. Visual leftovers (the grey triangle bottom right of the first in-world frames, the black near palm): not
   investigated; no original frame of the same spot yet (the original reaches it only after the intro).

### 17.9 Decisions needed

1. Install the 32-bit X11 client packages (lib32-libxcursor, lib32-libxrandr, lib32-libxi, lib32-libxkbcommon,
   lib32-libxkbcommon-x11, lib32-libxinerama, lib32-libxrender) system-wide for `sa-game --window`, or keep a
   project-local copy on `LD_LIBRARY_PATH`.
2. `CDoor::Process`'s obfuscated entry chunk: decode it by hand (as 14.6 item 1's VM values) or measure it in a Wine
   trace.

## 18. STD-12: stubs out of the walk, CDoor's entry chunk, the real-time tail

### 18.1 Main-session decisions on STD-11's questions (recorded by STD-12, 2026-11-12)

1. **32-bit X11 client libraries stay project-local** on `LD_LIBRARY_PATH` (`target/std11-libs/`, copied per worktree);
   a system install is the user's decision.
2. **`CDoor::Process`'s obfuscated entry chunk: decoded by hand and confirmed under Wine** (18.3).

### 18.2 Base and merges

Base STD-11 (`worktree-agent-a9a0d6cdcec07ed1b`, c013fb7). Cherry-picked own commits: WRITER-P 1b4c361 (car env-map
render callback 0x5D9900 and its helpers, train / big-vehicle visibility callbacks, boot statics, CColBox copy, float
pow), WRITER-Q 3c7512b, 6f24819, 0d79933, c7b7840, 1ac88ae (route round entity box / sphere, PlacePedAtEffect,
`CBike::ProcessRiderAnims`, `TidyUpGarageClose`, the chat ctor / `CarHasReasonToStop` natives, `CInterestingEvents::Add`,
the attractor pool and registry). Their generated queue files were skipped and regenerated at the end. WRITER-P's
five static-initialiser hooks replace sa-core's five `NATIVES_MISC` natives at the same addresses (one registration
per address). Not rebased onto master (INTEGRATE-4 had not landed when STD-12 ended).

### 18.3 `CDoor::Process` 0x6F4040: the entry chunk 0x407DD5

Entry `push -1; jmp 0x407DD5`; the chunk is `push ecx; pushf; clc; mov ecx, [0x1560894]; sbb ecx, [0x66D4C0]; popf;
xchg [esp], ecx; jmp 0x6F4047`. Decoded from the exe image: `0xD9C72A11 - 0xD9429FE9 - CF(0) = 0x848A28`, pushed with
ECX and the flags preserved, i.e. the ordinary `push -1; push handler` SEH prologue; 0x848A28 is `mov eax, 0x8A18A4;
jmp __CxxFrameHandler (0x8215A0)`. **Wine measurement** (lane 1, flock; original exe, sa-launcher + master's sa_hook.dll
with no ports selected, headless gamescope, `scratchpad/std12/measure-door.sh`): `peek` of 0x1560894, 0x66D4C0, the
chunk bytes 0x407DD5..+24, the handler 0x848A28..+10 and the entry 0x6F4040..+8 at start, at the main menu and in game
(`gGameState` 9, 20.9 s in): all identical to the exe image in all three probes (`d9c72a11`, `d9429fe9`, chunk
`51 9c f8 8b 0d 94 08 56 01 1b 0d c0 d4 66 00 9d 87 0c 24 e9 5a c2 2e 00`, handler `b8 a4 18 8a 00 e9 ...`): the
decoded value is 0x848A28 at run time too. Evidence `target/std12-evidence/door-r1/door.txt`.

The port (PORT-VEH-2's body, `sa-veh/door2.rs`) is now a hook (`std12-veh-1`, 7-byte expect through the `jmp`). The
protection reader `sbb edi, [0x6F4043]` at 0x4072EE reads the entry's rel32 as a key: a keycell reader (value
0xFFD13D8E) keeps it reading the original dword in phase A. The body was re-checked against the bytes (the axis-0/2
rotation, the `sin` for other axes, the chassis `pow`, the clamp and limit parity tests).

### 18.4 Ports

`docs/arch/std12-ports.csv` (regenerate with `tools/native/std12_ports.py`): 20 hooks and one native. Batches:
`std12-veh-1` (`CDoor::Process`, `CAutomobile::SetRandomDamage`), `std12-veh-2` (`CBike::FixHandsToBars`),
`std12-render-1` (`RenderWeaponPedsForPC`, `FxSystem_c::AttachToBone`), `std12-render-2` (`CPostEffects::HeatHazeFX`,
unreversed in gta-reversed), `std12-world-1` (`CPickups::DoPickUpEffects`, unreversed), `std12-world-2`
(`CColSphere::IntersectPoint`, L1-pure, spec in sa-oracle math_specs), `std12-ai-1` (`RemoveMember(CPed*)`,
`CMentalState::IncrementAnger` (.HOODLUM body 0x156FC20, its time read decoded to 0xB7CB84),
`CTaskComplexWalkRoundObject::ComputeRoute`, `CTaskComplexSmartFleePoint::MakeAbortable` / `SetFleePosition` /
`ComputeFleeDir`, `CEventHandlerHistory::StoreActiveEvent`, `GetIsLineOfSightClear`, `CTaskSimpleIKChain::BlendOut`),
`std12-audio-1` (`GetVehicle(Non)DriveWheelSkidValue`: existing bodies, now hooked with `preserve = "edx"`),
`std12-pop-1` (`CCarAI::UpdateCarAI`: the partial port, previously without a batch, now also covers law enforcers
while the wanted level is 0, plus the gang-area police cruise speed 0x420553..0x42065A; marked unverified because its
behaviour changed). Native: `CWorld::ClearScanCodes` 0x563470 (its entry is read as data at 0x4013A2). sa-game: the
CRT `acos` 0x822394 replacement (the `_CIacos` libm model).

Not ported (still exploration stubs or blockers): `CCarAI::UpdateCarAI` for missions > 1 and law enforcers with a
wanted level (fallback, ~3,400 instructions), the five `FrameUpdateCallBack` variants (0x4D2E40, 0x4D1DB0, 0x4D1A50,
0x4D30A0, 0x4D2450: none was reached in any STD-12 run), and the tail in 18.6.

### 18.5 Runs (release, `--start-in-world`, `SA_GAME_SCRIPT_SKIP=1`, i686, the user's install)

sa-game: `SA_GAME_KEYS=KEY:DOWN[:UP],...` scripted key presses for offscreen runs (letters, space, enter, shift, ctrl,
arrows).

STD-10/11's nine STD stubs are gone (`DoPickUpEffects`, `RenderWeaponPedsForPC`, 0x5D9900 via WRITER-P, `HeatHazeFX`,
`CDoor::Process`, `SetRandomDamage`, `UpdateCarAI`'s cruise/law path, WRITER-Q's 0x5F1890 / 0x5EA390 / 0x5F3DD0 /
0x5EFCA0 / 0x602590 / 0x449D10 / 0x6B7280). The real-time window runs reach new code every run (ambient peds, police
bikes, flee / IK / event tasks, vehicle audio, car collisions); each blocker found was ported until the last runs.

| run | stubs | in-game present intervals | blocked iterations |
|---|---|---|---|
| winA (before WRITER-Q's last four picks), 110 s, W + turns | 0x421A50, 0x6A6DC0, 0x6510D0 + Q's then-unlanded five | 931 intervals, mean 45.01 ms (22.2 fps), median 46.63, p95 61.68, p99 66.57, max 228.41 | 1,608 (1,497 `ClearScanCodes`, since ported; 105 drive-wheel skid, since hooked) |
| winC (all picks), 100 s | 0x421A50, 0x6A6DC0, 0x6510D0 | 899 intervals, mean 48.00 ms (20.8 fps), median 45.02, p95 70.88, p99 153.13, max 248.15 | 1,193 (1,181 `CBikeDamage KnockOffRider` 0x6B5A10) |

Exploration stubs still used for the evidence: `0x421A50` (the weave heading test of `WeaveForOtherCar`, ~400
instructions with argument-slot reuse), `CAutomobile::dmgDrawCarCollidingParticles` 0x6A6DC0 (thiscall, 3),
`CCarEnterExit::IsVehicleStealable` 0x6510D0 (needs `CPedGroup::IsAnyoneUsingCar` 0x5F7DB0 and
`IsClearToDriveAway` 0x6509B0). Two of three video runs after WRITER-Q's attractor picks crashed (null +4 in
`CPedAttractorManager::IsPedRegisteredWithEffect` after the unported `CPedAttractor` constructor 0x5EDFB0 blocked a
registration half-way; logs `target/std12-evidence/crash*-attractor*.log`); one earlier crash in
`CPointRoute::operator=` (null route, `crash-point-route-assign.log`).

Evidence (not committed, `target/std12-evidence/`): `winA/frame-times.txt`, `winA/gs-{60,90,110}.png`, `winA/sa-game.log`;
`winC/frame-times.txt`, `winC/sa-game.log`; `winV/cj-walk-window.mp4` (every 4th in-game frame of a window run, 67
frames, until the attractor crash); `door-r1/` (18.3); `car1/` (offscreen F at the parked car: blocked by
`GetNearestCarDoor` 0x6528F0); `glowA/` (18.7).

### 18.6 Driving

Offscreen `SA_GAME_KEYS=w:40:70,f:72:76,w:200:450` next to the parked car in front of CJ's house: F starts
`CTaskComplexEnterCar`, which blocks in `CCarEnterExit::GetNearestCarDoor` 0x6528F0 (~150 instructions behind a
protection chunk 0x4058AC). Not reached further; no driving video.

### 18.7 The orange blobs on the ground

Identified: the **sun halo corona's lens flare**. `CCoronas::DoSunAndMoon` registers corona id 2 (the halo) with flare
type 1 and the time cycle's halo colour; `SA_GAME_DUMP=0xC3E094:0x3C` in game at 08:05 shows id 2 with colour
(236, 121, 2), fade 255, flare 1. `CCoronas::Render` draws the `SunFlareDef` (0x8D4B68) sprites additively (SRC/DEST
blend ONE) at `(sun - centre) * def.position + centre`, so they lie on the line from the sun through the screen
centre (in STD-9's frame: sun (560, 20), blobs at (380, 330) and (240, 580)). The port follows the bytes (flare loop,
`ProcessLineOfSight` with vehicles / peds / shoot-through only, as pushed at 0x6FB3E8). No fix made: whether the
original draws the same flare at this spot is open (the e2e frames of the original do not show the sun; the halo is
drawn only with a clear LOS from the camera, and the original's flare uses the same additive star sprite). Next step:
an original frame with the sun in view at the same time of day (Wine, the corpus state at Grove Street, camera turned
to the sun), then compare the flare sprites (count, colour, raster bound at the sprite-buffer flush).

### 18.8 Next gaps, in order

1. The attractor path WRITER-Q left (crashes the window runs): `CPedAttractor` ctor 0x5EDFB0, `RegisterPed` 0x5EEE30 →
   `SetTaskForPed` 0x5EECA0 (+0x5EE270, 0x5EB310, 0x5EE300, 0x5ED4B0), `DeRegisterPed` 0x5EC5B0 (+0x5EAB80),
   `CGarage::IsEntityTouching3D` 0x448EE0, `CEventKnockOffBike` dtor 0x4AFE70.
2. `CBike::KnockOffRider`-path 0x6B5A10 (1,181 blocks in winC), 0x421A50, `dmgDrawCarCollidingParticles` 0x6A6DC0,
   `IsVehicleStealable` 0x6510D0 (+0x5F7DB0, 0x6509B0), `CTaskComplexWander::SetDir` 0x669D30, the clone / dtor
   vslots seen once (0x682DA0, 0x66CC60, 0x420F30, 0x4B18D0), `FxManager AddSparks` 0x49F040, 0x503CE0.
3. Driving: `CCarEnterExit::GetNearestCarDoor` 0x6528F0, then the enter-car task chain.
4. `CCarAI::UpdateCarAI` complete (missions > 1, law enforcers with a wanted level).
5. The five `FrameUpdateCallBack` variants (not reached yet).
6. 18.7 (original comparison frame).

### 18.9 Checks

`cargo test -p sa-hook --lib` 58 passed at every commit; clippy -D warnings on i686-unknown-linux-gnu for sa-veh,
sa-render, sa-world, sa-ai, sa-pop, sa-audio, sa-core, sa-ports, sa-game; writer gate: see the hand-back.

### 18.10 Decisions needed

1. Whether the remaining exploration stubs (18.5) may stay for the evidence runs while the real-time tail keeps
   growing, or STD-13 should first make the window run deterministic (fixed timestep) so one blocker list holds.
2. 18.7: who produces the original comparison frame (needs a Wine lane and a scripted camera turn).

## 19. STD-13: deterministic runs, the attractor base class, a crash-free window walk

### 19.1 Main-session decisions on STD-12's questions (recorded by STD-13, 2026-11-12)

1. **A deterministic mode** in sa-game produces one reproducible blocker list per scenario; real-time window runs stay
   the acceptance check (19.3).
2. **STD-13 produces the original sun-flare comparison** (Wine lane, offscreen gamescope, scripted camera turn) (19.6).

### 19.2 Base and merges

Base STD-12 (`worktree-agent-a0c5043f7469fc732`, d447cbf). INTEGRATE-5 had not landed on master when STD-13 ended, so
no rebase. Cherry-picked: WRITER-NATIVE 4012bb9 (`Fx_c::AddSparks`, two task clones, PORT-VEH-3's
`dmgDrawCarCollidingParticles`). Mid-task the main session moved the attractor tree to WRITER-ATTR and the native
blockers to WRITER-NATIVE; STD-13 kept entering cars, driving and the flare.

### 19.3 sa-game `--fixed-step MS` (deterministic mode, `det.rs`, `detheap.rs`)

* The Win32 time sources read a virtual clock: +MS at the start of every main-loop iteration, +10 us at every time
  read on the main thread (loops that poll the time end; their read count is itself deterministic), +ms for every
  `Sleep(ms)` on the main thread. The frame limiter does not sleep. With MS = 40.5 every in-game iteration runs one
  game frame (the limiter needs `40 < t`).
* Streaming reads complete at their request: after `CdStreamRead` releases the stream semaphore the main thread waits
  until the queue is empty and the streaming thread idle (and once more at the end of every iteration).
* The game heap (`malloc` / `calloc` / `realloc` / `free`, `CMemoryMgr::MallocAlign` / `FreeAlign`) comes from a
  fixed-address region (0x18000000, 896 MiB, size-class free lists): the game seeds `rand` with entity addresses
  (the timed enex lights, 0x5339F5), and glibc's addresses depended on what other threads allocated.
* The DirectSound play cursors advance by the step in the main loop (the ped speech state reads them).
* Runs use `setarch i686 -R` (no ASLR) and `GLIBC_TUNABLES=glibc.malloc.mmap_max=0` (scratch run script).
* Each iteration prints `det: clock, rand state, rand calls, srand calls (last seed)`; two runs of a scenario are
  compared on these lines and the step results. `SA_GAME_DET_BT=N` prints a backtrace for every time read of
  iteration N (how the remaining sources were found).
* Report and stdout end with the **blocker list** (callee, iterations, first frame). In this mode a repeated failure
  does not stop the loop.
* `SA_GAME_MOUSE=DX:DY:FROM:TO,...` scripted mouse motion; `SA_GAME_STUB` entries may pop up to 10 dwords, and a
  leading `!` replaces a registered port (exploration only; Rust callers that name the port directly still reach it).

Reproducibility: grove-walk scenario (`SA_GAME_KEYS=w:40:600,d:200:260,a:400:430`), two runs of 700 iterations:
identical traces (clock, rand state / counts, every step result) and identical blocker lists
(`target/std13-evidence/drep4-{a,b}/`). Before the heap and DirectSound fixes the runs diverged at iteration 24
(srand seed = an entity address) and then at 294 (ped speech state). Not covered: the audio threads' own timing
(smooth fade, stream decoder).

### 19.4 Ports (`docs/arch/std13-ports.csv`, `tools/native/std13_ports.py`)

53 hooks (`std13-ai-1` 52, `std13-veh-1` 1) and one `#[native]`:
* The **`CPedAttractor` base class**: constructor 0x5EDFB0 (entry `push -1; jmp` into 0x404CB7; keycell reader
  `adc edx, [0x5EDFB2]` at 0x40206A added), destructor 0x5EC410 and the nine derived complete destructors,
  `RegisterPed`, `SetTaskForPed`, `DeRegisterPed`, `BroadcastArrival`, `BroadcastDeparture`, `ComputeAttractPos` /
  `ComputeAttractHeading` (the x87 rounding of the original) and their random deltas, `HasEmptySlot`,
  `GetQueueSlot`, head / tail of the queue; the manager's `BroadcastArrival` / `BroadcastDeparture` dispatch and
  templates; the `CPedTaskPair` vector (push_back, insert, `_Insert_n`, fill, copies) and the `CTask*` vector
  instantiation of the pointer-vector module.
* Attractor tasks: `CTaskComplexGoToAttractor::CreateFirstSubTask` / `MakeAbortable`,
  `CTaskComplexUseAttractor::CreateFirstSubTask` (all ten types) / `CreateNextSubTask` / `ControlSubTask`, the
  `CTaskComplexSitDownThenIdleThenStandUp` constructor.
* Others found by the deterministic runs: `FindRespectedFriendInInformRange`, `CTaskComplexWander::SetDir`,
  `CTaskComplexWalkRoundObject::CreateSubTask`, the `CEventVehicleDamage` ctor, the 0x86C6D0 event's ctor / dtor,
  `CEventPotentialWalkIntoObject`'s dtor, the event dtor 0x420F30, `CBike::PlayCarHorn`;
  `CAEVehicleAudioEntity::GetBaseVolumeForBicycleTyre` 0x4F60B0 `#[native]` (its loop branches back into the 5
  patched bytes).

The CPointRoute null-route crash and the attractor `IsPedRegisteredWithEffect` crash (STD-12) followed a blocked
`CPedAttractor` constructor (a pool slot taken, a null pointer pushed to the manager's vector); neither recurred after
the base class was ported (no further STD-13 run crashed). Not separately root-caused for `CPointRoute::operator=`.

### 19.5 Runs (release, `--start-in-world`, `SA_GAME_SCRIPT_SKIP=1`, i686, the user's install)

Deterministic grove walk (2,000 iterations, `SA_GAME_KEYS=w:40:1900,d:300:340,a:700:760,d:1100:1180,a:1500:1540`):

| run | stubs | blocked iterations | blocker list |
|---|---|---|---|
| walk2 (attractor base ported) | STD-12's three | 1,604 | null vslot (an event half-destroyed after the blocked dtor 0x420F30), FindRespectedFriend, GoToAttractor first sub-task, step-attractor dtor, HasEmptySlot, PlayCarHorn |
| walk5 (use-attractor tasks ported) | same | 1,921 | the sit task's `ControlSubTask` 0x631730 (the sit / idle / stand-up task family follows) |
| walk10 | + `!0x5EFCA0` (no attractors), 0x6B5A10, 0x4B4950, 0x503CE0/10 | 93 | `CEventVehicleDamage` ctor (since ported), AddSparks (since picked), 0x5F6190 (since ported), SpawnFlyingComponent |

Window runs (headless gamescope 800x600, `--auto-input`, W held from iteration 100, scripted mouse turns; the
machine's load average was ~26 from other lanes):

| run | in-game present intervals | blocked | crash |
|---|---|---|---|
| winT, 95 s walking | 1,479 intervals, mean 61.23 ms (16.3 fps), median 55.30, p95 103.68, p99 133.95, max 720.85 | 5 | none |
| 95 s standing (W never pressed) | 2,161 intervals, mean 42.13 ms (23.7 fps), median 41.88, p95 54.46, p99 59.24 | 5 | none |
| winV2 video (PNG of every 3rd in-game frame) | 466 intervals (capture cost) | 439 (a gang ped's hand-signal task `ControlSubTask` 0x61B580) | none |

Exploration stubs of these runs: `0x421a50=0/0,0x6a6dc0=0/3,0x6510d0=0/0,!0x5efca0=0/4,0x6b5a10=0/0,0x4b4950=0/1,
0x503ce0=0/10,0x4b1a00=0/1` (0x6A6DC0 is ported since the WRITER-NATIVE pick). The walk is crash-free but not
stub-free.

Evidence (not committed, `target/std13-evidence/`): `drep4-{a,b}/` (reproducibility), `walk10/`, `walk11/` (blocker
lists), `winT/frame-times.txt`, `winV2/cj-walk-window.mp4` (CJ turns and walks across Grove Street; 157 captured
frames), `winV/cj-walk-window.mp4` (an earlier run into the garage wall), `flare-orig-l2/` (19.6).

### 19.6 The sun flare: the original

Lane 2 (flock), original exe with sa-launcher + master's `sa_hook.dll`, no ports selected, headless gamescope, New
Game, the intro skipped, the camera pitched up and swept a full turn by xdotool relative motion (36 shots), `peek
0xC3E094 3840` (`CCoronas::aCoronas`) and the clock at every shot (scratch `flare-orig.sh`, `corona.py`). At 07:xx
the original registers **corona id 2 (the sun halo) with flare type 1** whenever the sun is above the horizon, colour
(166..218, 125..128, 2..6) rising with the time (the native run had (236, 121, 2) at 08:05), together with id 1 (the
sun core, flare 0). In all 36 shots the halo's fade intensity was 0 (sun outside the view or its line of sight
blocked), so no original frame with the flare drawn exists yet. Conclusion so far: the native blobs are the same
`SunFlareDef` flare the original registers; the pixel comparison (count, colour, raster bound) still needs an
original frame with the sun unoccluded. No code change.

### 19.7 Entering a car (not done)

`CCarEnterExit::GetNearestCarDoor` 0x6528F0 (gr-reversed; ~350 instructions, protection chunk 0x4058AC loads +0x464)
needs `GetPositionToOpenCarDoor` 0x64E740 (gr-unreversed, ~150 instructions, jump table 0x64EC7C over door ids 8..11,
x87) and `IsRoomForPedToLeaveCar` 0x6504C0 (gr-unreversed); both are unported. After them the enter-car task chain
(CTaskComplexEnterCar sub-tasks, door animations) and the player's driving path follow. Not reached in STD-13; no
driving video.

### 19.8 Next gaps, in order

1. Enter car: 0x6504C0, 0x64E740, 0x6528F0, then the CTaskComplexEnterCar chain; driving.
2. The sit / idle / stand-up tasks (WRITER-ATTR): `CTaskComplexSitDownThenIdleThenStandUp` Clone 0x636390,
   MakeAbortable 0x631510, CreateNextSubTask 0x637DB0, CreateSubTask 0x631550, ControlSubTask 0x631730;
   `CTaskSimpleSitDown` / `SitIdle` / `StandUp` (StartAnim, ProcessPed, Clone, MakeAbortable, finish callbacks);
   `CTaskComplexUsePairedAttractor::CreateNextSubTask` 0x639E40; the `CTaskComplexSunbathe` ctor 0x631F80; the script
   brain calls 0x46B390 / 0x635850 / 0x6358B0. Then drop `!0x5EFCA0`.
3. The remaining stubs: `CBike::DamageKnockOffRider` 0x6B5A10 (~280 instructions, x87),
   `CAEWeaponAudioEntity::PlayGunSounds` 0x503CE0, `CEventPotentialWalkIntoObject::AffectsPed` 0x4B4950, 0x4B1A00,
   0x421A50, 0x6510D0.
4. The hand-signal task `ControlSubTask` 0x61B580 / `CreateSubTask` 0x61B2F0, task clone 0x636BE0, 0x664770,
   0x5FACD0, 0x5179E0, 0x605A30, `CAutomobile::SpawnFlyingComponent` 0x6A8580.
5. `CCarAI::UpdateCarAI` complete (missions > 1, law enforcers with a wanted level): not started in STD-13.
6. 19.6's pixel comparison; the black palm and the grey Im3D triangle.

### 19.9 Checks

At every commit: `cargo test -p sa-hook --lib` 58 passed; clippy -D warnings i686-unknown-linux-gnu for the touched
crates (sa-ai, sa-veh, sa-audio, sa-game, sa-core) and host for sa-ai, sa-hook, sa-game, sa-core; i686-pc-windows-gnu
clippy of sa-hook (keycells); writer gate hooks step: 53 new hooks, all unverified, batches std13-ai-1 (52) and
std13-veh-1 (1).

### 19.10 Decisions needed

1. Whether the deterministic mode's fixed-address heap should also serve window runs (repeatable real-time runs up
   to input timing; it reuses freed blocks only within their size class).
2. The flare pixel comparison: accept the state-level confirmation (19.6), or spend a lane run on a scripted camera
   facing the sun (a free camera or a corpus state with the sun in view).

## 20. RENDER-SHADOW: render targets without alpha

User report (window run, 08:07): large dark grey quads on the ground under and in front of CJ (a darker trapezoid and
a lighter rectangle) that move with the camera. Base STD-13 (9e704a6).

**Root cause (seam).** The game's camera-texture rasters are `D3DFMT_X8R8G8B8` render-target textures (the
real-time shadow cameras: 128x128 / 64x64; the post-effect screen copies: 1024x1024); the back buffer is
`A8R8G8B8`. `WgpuDevice` stored every render target as RGBA8 and kept the alpha draws wrote, while D3D9 has no
alpha there: it samples alpha 1, ignores alpha writes and `StretchRect` into it drops the source alpha.

1. **The quads.** `CStencilShadows::RenderStencilShadows` (z-fail volumes of the parked car, then a full-screen
   `DrawRect` with the stencil test) changes mostly the back buffer's *alpha*. `CPostEffects`' colour filter copies
   the back buffer into an X8 texture (`StretchRect`, a GPU copy on the seam that kept the alpha) and adds it back
   with SRCALPHA / ONE: wherever a shadow volume set the stencil, the filter added less, so the volumes' screen
   footprint showed as grey quads. The grey triangle at the bottom right of the first frames (16.8 item 6) is the
   same effect. Found with `SA_GAME_D3D_BBDUMP`: the back buffer is unchanged up to the colour-filter draws.
2. **CJ's real-time shadow** (`CRealTimeShadow::Update`): `CShadowCamera::InvertRaster` leaves alpha 0 inside the
   silhouette (cleared alpha 0, CJ drawn with TFACTOR alpha 1, INVDESTCOLOR / ZERO); the resample / blur passes
   sample it with the alpha test on (ref 0, GREATER), so the silhouette texels were discarded and the shadow texture
   kept only faint edges. In D3D9 the X8 raster samples alpha 1 and every texel passes.

**Fix** (`crates/sa-rw-wgpu/src/d3d9.rs`): render targets whose D3D format has no alpha (X8R8G8B8, R8G8B8, R5G6B5,
X1R5G5B5, X4R4G4B4) start at (0, 0, 0, 1); draws into them mask alpha writes; clears store alpha 1; `StretchRect`
into them resets the copied rectangle's alpha (an alpha-only pass on the GPU path, 255 on the CPU resample path).
`WgpuDevice::set_back_buffer_format` does the same for the back buffer; sa-game passes `BackBufferFormat` at
`CreateDevice` / `Reset` (D3DFMT_UNKNOWN = X8R8G8B8). Test `render_target_without_alpha`.

**Tooling** (`crates/sa-game/src/d3d9.rs`, exploration only): `SA_GAME_D3D_LOG=FROM:TO` logs the render-target
changes, clears and draws of presented frames FROM..=TO with their states; `SA_GAME_D3D_RTDUMP=DIR` writes the
render-target textures of those frames as PNGs; `SA_GAME_D3D_BBDUMP=K,...|stencil` writes the back buffer before
chosen draws.

**Evidence** (not committed, `target/shadow-evidence/`): `window-before-after-{0,1}.png` (headless gamescope window
runs, play-package binary vs the fix), `offscreen-before-after-{0100,0130,0160}.png` (deterministic,
`SA_GAME_KEYS=w:100:160`), `cj-shadow-crop-before-after.png`, `shadow-raster-passes-{before,after}.png` (the shadow
camera / resample / blur / gradient passes, RGB above, alpha below), `colour-filter-draws-before-fix.png`.

Not done: a Wine reference frame of the original at the same spot; the black near palm (not investigated).

## 21. STD-14: the enter-car chain up to the door, lane integration, the window heap

### 21.1 Main-session decisions on STD-13's questions (recorded by STD-14, 2026-11-12)

1. **One heap layout everywhere:** window runs serve the game heap from the deterministic mode's fixed-address
   region too (`det::fixed_heap`, decided at argument parsing, before the first game allocation); `--system-heap`
   turns it off. The region falls back to a kernel-chosen address when 0x18000000 is taken (no `setarch -R` in the
   play package). Freed blocks are reused only within their size class; long sessions were not measured.
2. **The flare:** the state-level match of 19.6 is enough for now; no lane run on it.

### 21.2 Base and merges

Base STD-13 (`worktree-agent-adab932ed2ad46a44`, 9e704a6). Cherry-picked, in order: PORT-AI-8 (f9328ce, 462c48d,
5e4797c, 60ec250, e030fca, later d879c4d), WRITER-NATIVE (404c763; 68ffe50..34e3306 without 6c6d495, a duplicate
of 404c763; f19bf238, 092fa866; 0ab02d5..654c82d without 1a13d09), WRITER-ATTR (3461aa8; dcbe3d2, 53c1f7e,
5ee67d7, 2c9d51a; the rest already came through WRITER-NATIVE), RENDER-SHADOW (7e080de, 86d6e5f, 3984228, its
platform.md section 20), PORT-VEH-3/4 (1e2f939, be17e4f, 08c723d, 49cc1ac, df2eefb). Conflicts in the generated
queue files (`catalog/verify-queue.csv`, `docs/verify/QUEUE.md`) were resolved to ours and not regenerated.

Duplicates removed on the way (each address registered once, `cargo test -p sa-hook --lib` 58 passed):
* PORT-AI-8's generated copies (`gen_*.rs`, `samp_reached_small.rs`, `intel_group_misc.rs`) of 47 functions other
  lanes port by hand; WRITER-NATIVE's f19bf238 then regenerated the AI-GEN modules without them.
* STD-13's `CBike::PlayCarHorn`, STD-14's `IsPlayerToQuitCarEnter` and crane native in favour of PORT-VEH-3/4's.
* The keycell `READERS` of both sides merged (41 readers).

**AI-GEN bug found:** the generated bodies of 0x5F2B80, 0x5F3730 and 0x5F3B40 call
`ComputeEntityBoundingBoxCornersUncached` 0x5F1FA0 as a one-argument thiscall; the original pushes three cdecl
arguments. The enter-car walk crashed on the null entity (SIGSEGV at 0x36). The three addresses are excluded in
`tools/gen/ai_patterns_exclude.txt`; other callees whose entry is `nop; jmp` may be misdeclared the same way.

### 21.3 Ports (`docs/arch/std14-ports.csv`, `tools/native/std14_ports.py`)

* `std14-veh-1`: `CCarEnterExit::GetPositionToOpenCarDoor` 0x64E740, `IsRoomForPedToLeaveCar` 0x6504C0 (both
  unreversed in gta-reversed), `GetNearestCarDoor` 0x6528F0 (entry chunk 0x4058AC; the bytes differ from
  gta-reversed in the bike branch, door 0x12 and the towed-vehicle path test).
* `std14-ai-1`: `CTaskComplexEnterCar` `CreateFirstSubTask` 0x643A60, `ControlSubTask` 0x63A890, `CreateSubTask`
  0x63E040, `CreateNextSubTask` 0x63E990 (0x2C0 and the fall-and-get-up order follow the bytes),
  `CreateNextSubTask_AfterSimpleCarAlign` 0x63F970 (chat reactions with `rand()` at their call sites), the
  `CTaskComplexEnterCarAsDriver` destructor 0x640330; `ComputeTargetDoorOppositeToFlag` 0x63AB70 is called directly
  (not in the catalogue).
* `std14-veh-2`: `CVehicle::ProcessOpenDoor` 0x6D56C0 (the door enum's default passes the time's bits; the default
  animation groups of the last cases read the pushed `this` and argument slots as floats, as the bytes do).
* `std14-ai-2`: `CEventLeaderEnteredCarAsDriver` destructor 0x48E220, `CTaskComplexGoToCarDoorAndStandStill::
  ControlSubTask` 0x64A820, `CTaskSimpleSlideToCoord::ProcessPed` 0x66C4E0.
* Phase-B natives of existing ports: `CTaskSimpleCarSetPedOut::PositionPedOutOfCollision` 0x6479B0 (PORT-AI-3's
  body, unhooked for its split L2 difference) and `CAERadioTrackManager::QueueUpTracksForStation` 0x4EA670
  (unhookable entry).
* Extended partial ports: `CCarAI::UpdateCarAI` 0x41DA30 now takes the ported body for the missions both of its
  mission switches leave to the default (45..=51, 56..=58, 62, 63, > 68: parking, heli landing...); the parking car
  at Grove Street (mission 45) blocked every frame. `SteerAICarWithPhysics_OnlyMission` 0x436A90 dispatches the
  parking missions 0x2D / 0x31 and 0x2E / 0x32 to the ported `SteerAICarParkPerpendicular` / `Parallel`.
* sa-game: `SA_GAME_VEHLIST=N` (vehicles near the player, with mission and law flag) and
  `SA_GAME_TELEPORT=ITER:X:Y:Z[:HEADING]` (the player through its `Teleport` virtual) for exploration runs.

### 21.4 Runs (release, `--start-in-world`, `SA_GAME_SCRIPT_SKIP=1`, i686, no stubs)

Enter-car scenario: deterministic, `SA_GAME_TELEPORT=60:2456.5:-1761.7:13.6` (next to the parked Savanna, model
567, at (2452.88, -1761.75)), `SA_GAME_KEYS=f:100:103`.

| run | first blocker | where the player got |
|---|---|---|
| car1 | `GetNearestCarDoor` 0x6528F0 | standing |
| car4 | `CreateFirstSubTask` 0x643A60, the 0x640330 destructor | standing |
| car5..car9 | the leader event 0x4B0EF0, `UpdateCarAI` (mission 45), the steering of mission 45 | standing |
| car9 | SIGSEGV in the AI-GEN body of 0x5F3730 (21.2) | standing |
| car12 | `GoToCarDoorAndStandStill::ControlSubTask` 0x64A820 | standing |
| car13..car14 | `CTaskUtilityLineUpPedWithCar::ProcessPed` 0x6513A0, `CVehicle::ProcessOpenDoor` 0x6D56C0 | at the door (2454.72, -1761.54) |
| car15 (ProcessOpenDoor ported) | `LineUpPedWithCar::ProcessPed` 0x6513A0 (from frame 118), then `ProcessPlayerVehicleEngine` 0x4FBB10, the excluded 0x5F3730 | at the door |

Window runs (headless gamescope 800x600, `--auto-input`, W held from iteration 150): 60 s, 1,887 frames, 0
blocked, no crash (system heap, before 21.1); with the fixed heap a `CTaskSimpleSlideToCoord::ProcessPed` block
appeared (since ported).

Evidence (not committed, `target/std14-evidence/`): `enter-door/png/` and `enter-door/cj-to-car-door.mp4`
(deterministic run, CJ teleported beside the car turns and walks to its door; the camera faces the wall),
`car*/sa-game.log` (blocker lists), `win3/`, `win4/`, `win5/` (window runs).

### 21.5 Play package

Refreshed at 08:54 (RENDER-SHADOW), 09:04 (WRITER-NATIVE car damage, two stubs left), 09:33 (no stubs,
`SA_GAME_STUB` gone from `play.sh`), 09:47 (fixed heap, SlideToCoord, fists) and 09:54 (ProcessOpenDoor; 90 s
headless window walk: 1,128 frames, 0 blocked, no crash). The README says not to press F next to a car: three identical blocked frames
stop a window run.

### 21.6 Next gaps, in order

1. `CTaskUtilityLineUpPedWithCar::ProcessPed` 0x6513A0 (unreversed; 0x6513A0..beyond 0x652640, more than 1,000
   instructions: the chunk 0x406B07, SEH, a per-anim switch of 0xFA entries, the slide offset and heading blend, then
   for the bike / quad anims 0x167..0x16B two `CMatrix` locals, `RtQuatConvertFromMatrix` 0x7EB5C0, an inlined
   quaternion slerp with polynomial sin / cos, and more) and `GetLocalPositionToOpenCarDoor` 0x64FC10. Read up to
   0x651FFD in STD-14, not ported: the next lane's main item for entering cars.
2. Then the get-in / close-door / set-ped-in tasks run (their bodies are ported) and the driving path follows:
   PORT-VEH-4's `ProcessControlInputs` are now in; camera in car, vehicle audio (`ProcessPlayerVehicleEngine`
   0x4FBB10 blocked in car14).
3. `CCarEnterExit::GetNearestCarPassengerDoor` 0x650BB0 (passengers), `CEventVehicleCollision` ctor 0x4AC840,
   `CPed::KillPedWithCar` 0x5F0360, `ComputeClosestSurfacePoint` 0x5F3B70, `ChoosePolicePedOccupation` 0x610F40.
4. `UpdateCarAI` for law enforcers with a wanted level and the remaining missions.
5. AI-GEN: check the other `nop; jmp` callees for the 0x5F1FA0 argument bug; port 0x5F3730 / 0x5F2B80 / 0x5F3B40 by
   hand (0x5F3730 blocks once excluded).
6. `CAEVehicleAudioEntity::ProcessPlayerVehicleEngine` 0x4FBB10 (blocked every frame after 178 in car15).

### 21.7 Checks

clippy -D warnings, i686-unknown-linux-gnu: sa-ai, sa-veh, sa-core, sa-game, sa-pop, sa-audio, sa-ports; host:
sa-ai, sa-veh, sa-core, sa-pop, sa-audio, sa-hook. `cargo test -p sa-hook --lib`: 58 passed. Every commit builds
(the release sa-game build after each).

### 21.8 Decisions needed

1. Whether to keep the fixed-address heap in the play package before a long-session memory test (it never returns
   memory across size classes).
2. Whether PORT-AI-3's split-L2-failing `PositionPedOutOfCollision` / `SetPedOut::ProcessPed` bodies may stay
   registered for phases B/C until their difference is resolved.

## 22. WEB-1: sa-game in the browser (wasm32)

User decision 2026-11-12: keep the legacy 32-bit layouts and the address arena in wasm32 linear memory. Base: STD-13
(`worktree-agent-adab932ed2ad46a44`, 7da5de4). How to run it: `web/README.md` (`tools/web/build.sh`, then
`tools/web/serve.sh [--game DIR]`, open http://localhost:8642/).

### 22.1 The arena at the original addresses

wasm32 pointers are 32-bit like the exe's, so the arena is plain linear memory: the module is linked with
`--global-base=0x1600000 --no-stack-first -zstack-size=16 MiB` (`tools/web/build.sh`), so its data, stack and heap lie
above 0x1600000 and everything below is free. `arena::map_fixed` replaces `mmap(MAP_FIXED_NOREPLACE)`: on wasm32 it
only checks that the range lies below `WASM_LOW_END` (the memory is already there and zero). The exe's
`.rdata`/`.data`/`.bss`/`_TEXT_HA`/`_rwdseg`/`.rsrc` (0x858000..0xCB1000), the read-only code bytes the no-CD protection
reads (0x401000..0x858000, `.HOODLUM` 0x1556000..0x1577000) and sa-game's COM / D3D9 / DirectSound pages (0x30000..)
sit at their original addresses; import traps stay 0x20000 + 0x10·i. No page protection (`.rdata` is writable),
no fault handler (a stray access below memory size reads the arena or zeros; above it the module traps).
`tools/web/check_layout.mjs` checks the built module (data segments ≥ 0x1600000, initial stack pointer above it).

### 22.2 `orig!` dispatch on wasm32: type-erased entries (`sa_core::wabi`)

wasm checks every indirect call against the callee's exact signature, while declarations and ports may disagree on
types the x86 ABI tolerates (`u8`/`u32`, `f32`/`X87`, struct vs words, variadic CRT calls). So every registry entry
on wasm32 is `extern "C" fn(callee, args: *const u32, ret: *mut Ret)`: the caller (`__orig_dispatch`) pushes its
arguments as the original's stack words (each rounded up to 4 bytes, in declaration order, `this` / register
arguments first), the callee reads them with its own types; results come back as EAX:EDX bytes or ST0 (an f64,
used when the type is `f32`/`f64`/`X87`). `#[hook]` and `#[native]` emit such an entry next to the x86 one; the CRT
replacements, imports, COM methods, RW stand-ins, static-initialiser shapes, lone-`ret` natives and the
`SA_GAME_STUB` exploration stubs register erased entries too (`wabi_wrap!`). `sprintf`/`sscanf` read their variadic
words from the buffer as the x86 entries read the stack. X87 results lose their 80-bit tail across a dispatch
(f64 in `Ret`).

### 22.3 What wasm32 lacks, and the replacements

| need | wasm32 |
|---|---|
| threads (`CdStreamThread`, audio threads, DirectSound mixer) | none: `ResumeThread` of `CdStreamThread` marks it running and each release of the stream semaphore runs its loop body (`cd_stream_thread_step`, split out of the port without behaviour change) once per count on the main thread; audio threads stay suspended; the DirectSound buffers advance in real time per iteration |
| blocking waits | semaphores never block (an unsignalled wait times out at once); `Sleep` and the frame limiter do not sleep |
| time | `web_time` (`performance.now()`) |
| files | `webfs.rs`: the page's `File`s (`FileReaderSync`) or the localhost server's ranged synchronous requests, 64 KiB read-ahead per open file (the text loaders read byte by byte), 256 KiB block cache in JS |
| C heap | Rust allocator with a size header, **zeroed** (native fresh pages are mostly zero; dlmalloc reuses blocks at once and stale pointers in fields the original leaves uninitialised trapped in the FX blueprint code) |
| CPUID | a fixed virtual SSE2 CPU (`abi::game_fpu::cpuid`): the SSE skinning paths of the native runs |
| GPU | wgpu on an `OffscreenCanvas` in the worker: WebGPU, else WebGL2 (`?backend=gl|webgpu`); a positive base vertex becomes a vertex-buffer offset (WebGL2 has no base-vertex draws) |
| fork probe, SIGSEGV reporter, `--present-dir` read-back | none; panics and traps are reported to the page with their wasm stack |

### 22.4 Results (headless Chromium 14x, offscreen, `tools/web/headless.mjs`)

* WebGPU (SwiftShader, `--use-webgpu-adapter=swiftshader`): boot, logos, main menu (real Enter key events move it on),
  the legal splash on the canvas; `CGame::Initialise` then stalls on the GPU process (SwiftShader WebGPU at ~1000 % CPU).
* WebGL2 (ANGLE/SwiftShader): through `CGame::Initialise` into the world: **CJ in Grove Street with the HUD**
  (`target/web1-evidence/gl2/t0107-state9.png`, not committed), about 105 s after start.
* The user's desktop (Brave, AMD RX 7800 XT, WebGPU): through the menu into `CGame::Initialise`, then an
  out-of-bounds trap in `FxManager_c::FindFxSystemBP` (fixed candidate: the zeroed C heap, 21.3).

### 22.5 Network seam for SA:MP (not built)

A page cannot open UDP. The plan: a separate WebSocket↔UDP proxy on localhost (another lane); the samp-rs client's
socket trait gets a web implementation that sends one binary WebSocket message per datagram
(`ws://localhost:PORT/udp?host=…&port=7777`) and queues received ones in JS; the client drains the queue once per
main-loop iteration, so nothing blocks the game loop.
