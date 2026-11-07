# PORT-CORE: the rest of subsystem CORE (sa-core)

Follows `docs/arch/pilot-ports.md` (ARCH-1 pilots) and ADR-001. Every row of `catalog/state.csv` with
`subsystem == CORE` and state `specified`/`unknown` (298 rows) has a decision below. Agent states (the main
session sets `catalog/state.csv`):

| state | rows | meaning here |
|---|---|---|
| `L1` | 56 | Rust port, bit-exact against the original bytes under `sa-oracle l1 <key> --cases 1000000` (0 mismatches; SNaN-input cases decided by the CPU where floats are involved). Hooked unless the row says otherwise (3 rows: entry too short or a protection branch into the entry) |
| `implemented-needs-L2` | 191 | Rust port registered with `#[hook]`; reads/writes game globals or calls other functions, so it needs CORE-L2 in game. The row gives the L2 signature (conv, `this` type, return kind) |
| `ported-unhookable` | 23 | Rust port without a hook: the entry cannot be patched (a lone `ret`; a branch back into the first 5 bytes; an argument in EAX/ESI that no supported convention carries). Phase B uses the Rust function; phase A keeps the original |
| `deferred` | 25 | orchestrators (CGame init/shutdown/process, Game::Idle/Render*, WinMain, psInitialize, …), `CTimer::Update` (sa-hook's frame-boundary hook), and the mis-split row 0x7487CF |
| `blocked` | 3 | `App::DoRWStuffStartOfFrame(_Horizon)` and `App::AppEventHandler` use FPTAN, for which sa-math has no CPU-checked model (Unicorn is unfaithful for it, docs/re/l1-harness.md) |

Ported = 270 (L1 + needs-L2 + unhookable). Sources: the 1.0 US disassembly (capstone over the exe, extents
from `catalog/extents.csv`, `.HOODLUM` bodies and protection chunks followed) and the Ghidra decompilation of
all 298 rows; gta-reversed @ 20d8d2a as reading aid. Where they differ, the original is followed; the
differences are in the deviations column (`gr-deviations.csv` rows cited, *new* ones listed at the end).

## Layout of the code

| module | contents |
|---|---|
| `pool.rs` (pilot) | generic `CPool<T>` logic; `pool_delete` fixed to re-read `m_nFirstFree` after the map write and store only when lower (the 0x536DD0 spec caught the pilot's order: 113/20,000 mismatches) |
| `pools.rs` | every other `CPool` instantiation (New, ctors, operator new/delete), `CPools::*` incl. Save/Load |
| `quadtree.rs`, `references.rs` | `CQuadTreeNode<T>`, `CReferences` |
| `directory.rs`, `keygen.rs`, `filemgr.rs`, `inifile.rs`, `memory.rs`, `loadmonitor.rs`, `common.rs` | CDirectory, CKeyGen, CFileMgr, CIniFile, CMemoryMgr/CMemoryHeap, CLoadMonitor, CDebug and `Common::` helpers |
| `platform.rs`, `video.rs` | Rs*/Ps*/Win/App/Securom glue; VideoMode/VideoPlayer. Win32/DirectX/DirectShow through `orig!` (`site` = `call [IAT]` instruction, `cell` = IAT slot or function pointer, `vslot` = COM method, `table` = RwEngineInstance function table, `dyn` = GetProcAddress result) |
| `input.rs`, `light.rs` | Input (DirectInput through `vslot`), CPad key helpers, GTATranslateKey, DefinedState; Light:: setters (RpLight* through `orig!`) |
| `game.rs` | CGame / Game:: leaves and short sequences |

Infrastructure changes: each module has its own `HOOKS` slice and one `globals!` list `GLOBALS`;
`hooks::ALL` is their const concatenation (`hooks::MODULES`), `hooks::GLOBALS` feeds the initial-value test
(every global's initial value is checked against the exe image) and a new test that every global address is
declared once. `orig!` gained the `cell`, `vslot`, `table` and `dyn` forms. sa-hook's
`matches_catalogue_and_exe` accepts port hooks of functions gta-reversed does not hook when their name is the
catalogue's `name` column (or Ghidra's name while that is empty). L1 specs: `sa-oracle/src/specs_core.rs`
(pools, quadtree, references, the pilot's CPed operator new/delete), `specs_core_util.rs` (+ `specs_core_c1.rs`,
`specs_core_c2.rs`).

Hook entries worth knowing for L2: inline `expect`s longer than 5 bytes where the entry is a `jmp` that is not a
`.HOODLUM` stub (CFileMgr::OpenFile/CloseFile, CMemoryMgr::Free, Rs `jmp ps*` wrappers,
Light::DeActivateDirectional); entries that start with a `call` (CPools::Save/Load, kept-return trampoline);
four functions enter through protection chunks that compute an address from a `.HOODLUM` dword and a `.text`
dword (RsKeyboardEventHandler, DoRWStuffEndOfFrame, GetVideoModeList, InitUserDirectories) and the ports
compute it the same way at run time; `__init_g_c87bd0` runs during CRT start-up, before sa-hook attaches.

Original uninitialised values that the ports pass as 0 (L2 may show them): the stack colour alpha in
LightsCreate / WorldReplaceNormalLightsWithScorched / AddAnExtraDirectionalLight; matrix flags/pos in
SetLightsWithTimeOfDayColour; GetMouseSetUp's padding byte and caps after a failed GetCapabilities; diPadInit's
pad-1 property header; LoadVehiclePool/LoadPedPool's count when the load does not write it; SaveObjectPool's
stack buffer; ValidateVersion's local buffer.

## Functions (all 298 CORE rows that were `specified`/`unknown`)

| address | name | conv | port (sa-core) | source | deviations followed (original kept); *new* = not in gr-deviations.csv | state | notes |
|---|---|---|---|---|---|---|---|
| 0x403FA0 | CBuildingPool::New | thiscall | pools::building_pool_new (pool::pool_new) | bytes | as CPedPool::New (search from first_free+1, no CLEANLAND fill); slot 0x38 | L1 (1,000,148 cases, 0 mismatches) | |
| 0x405900 | CIplDefPool::Constructor | thiscall, ret 8 | pools::ctor_ipl_def | bytes (.HOODLUM 0x1561B20) | *new*: CPool ctor (see pools.rs header): no NOMANSLAND fill, +0x11 not written; slot 0x34 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | operator new[] stubbed in L1 (allocator queue); real allocation L2 |
| 0x40FB80 | CColModelPool::New | thiscall | pools::col_model_pool_new | bytes | as CPedPool::New; slot 0x30 | L1 (1,000,148 cases, 0 mismatches) | |
| 0x41B5B0 | CPointRoutePool::New | thiscall | pools::point_route_pool_new | bytes | slot 0x64 | L1 (1,000,148 cases, 0 mismatches) | |
| 0x41B660 | CPatrolRoutePool::New | thiscall | pools::patrol_route_pool_new | bytes | slot 0x1A4 | L1 (1,000,148 cases, 0 mismatches) | |
| 0x41B710 | CNodeRoutePool::New | thiscall | pools::node_route_pool_new | bytes | slot 0x24 | L1 (1,000,148 cases, 0 mismatches) | |
| 0x41B860 | CNodeRoute::operator new | cdecl | pools::node_route_operator_new (no #[hook]) | bytes | `mov ecx, ms_pNodeRoutePool; jmp 0x41B710` | ported-unhookable (L1 (1,000,148 cases, 0 mismatches)) | protection bytes after the next stub branch into the stolen range (`jmp short` 0x41B87A → 0x41B865): installer refuses |
| 0x41B870 | CNodeRoute::operator delete | cdecl | pools::node_route_operator_delete | bytes (.HOODLUM 0x156E0C0) | *new*: first_free written only when lower; no null check (gr FIX_BUGS) / no DEADLAND fill | L1 (1,000,030 cases, 0 mismatches) | L1 with ms_pNodeRoutePool patched to scratch |
| 0x44D0E0 | FUN_0044d0e0 | cdecl | pools::ret_44d0e0 (no #[hook]) | bytes | lone `ret` | ported-unhookable | lone `ret` + padding: relocate TooShort(1) |
| 0x4D5980 | CPad::IsEnterJustPressed | cdecl, AL | `input::is_enter_just_pressed` | bytes | none (enter or extenter just pressed) | L1 (1,002,000 cases, 0 mismatches; key-state operands patched into scratch) | static `NewKeyState`/`OldKeyState` reads |
| 0x4D59B0 | CPad::IsStandardKeyJustPressed | thiscall (`this` unused), `ret 4`, EAX | `input::is_standard_key_just_pressed` | bytes | returns EAX 0/1 (gr: bool) | L1 (1,002,000, 0) | caller 0x748B89 passes `GetPad(0)` in ECX |
| 0x532240 | FUN_00532240 (gr `CDebug::DebugInitTextBuffer`) | cdecl | `common::debug_init_text_buffer` | bytes | *new*: original clears the `uint16` count at 0xB71508 (gr reserves a `std::vector`) | implemented-needs-L2 | L2: Void; write set = 0xB71508 (2 bytes) |
| 0x532260 | CDebug::DebugDisplayTextBuffer | cdecl | `common::debug_display_text_buffer` (no hook) | bytes | *new*: empty in 1.0 US (gr draws debug strings) | ported-unhookable | lone `ret` |
| 0x532290 | CDirectory::Constructor-Empty | thiscall | `directory::constructor_empty` | gr + bytes | *new*: +0x0C (`m_bOwnsEntries`) not written (gr default-initialises it) | implemented-needs-L2 | L2: this=CDirectory, ret EAX=this |
| 0x5322a0 | CDirectory::Constructor-Capacity | thiscall | `directory::constructor_capacity` | gr + bytes | `operator new` 0x82119A of `capacity << 5` (wrapping) | implemented-needs-L2 | L2: this=CDirectory, 1 arg, EAX=this |
| 0x5322d0 | CDirectory::Destructor | thiscall | `directory::destructor` | gr + bytes | frees with CRT `free` 0x8214BD | implemented-needs-L2 | L2: this=CDirectory, Void |
| 0x5322f0 | CDirectory::Init | thiscall | `directory::init` | gr + bytes | none | implemented-needs-L2 | L2: this=CDirectory, 2 args |
| 0x532310 | CDirectory::AddItem- | thiscall | `directory::add_item` | gr + bytes | gr-deviations `Directory.cpp:52` (FIX_BUGS duplicate check absent, kept); overflow message via `printf` 0x821982 | implemented-needs-L2 | L2: this=CDirectory, 1 arg |
| 0x532350 | CDirectory::ReadDirFile | thiscall | `directory::read_dir_file` | gr + bytes | AddItem inlined; *new*: the entry count is read into the `filename` argument slot (a failed read leaves the pointer value as the count) | implemented-needs-L2 | L2: file I/O (CFileMgr calls) |
| 0x532410 | CDirectory::WriteDirFile | thiscall | `directory::write_dir_file` | gr + bytes | none (no header written, as gr) | implemented-needs-L2 | L2: ret AL |
| 0x532450 | CDirectory::FindItem- | thiscall | `directory::find_item` | gr + bytes | `_stricmp` 0x8229B6 (locale state: L2) | implemented-needs-L2 | L2: ret EAX |
| 0x5324a0 | CDirectory::FindItem-ByName | thiscall | `directory::find_item_by_name` | gr + bytes | *new*: stores `*size` before `*pos` (gr: pos first) | implemented-needs-L2 | L2: ret AL, 3 args |
| 0x5324d0 | CDirectory::FindItem-ByHash | thiscall | `directory::find_item_by_hash` | gr + bytes | *new*: `*size` before `*pos`, entry re-read for each | implemented-needs-L2 | L2: ret AL, 3 args; calls GetUppercaseKey (toupper) |
| 0x536D10 | CEntryInfoNodePool::GetAt | thiscall | pools::entry_info_node_pool_new | bytes | *new*: catalogue mislabel, the body is CPool<CEntryInfoNode>::New (slot 0x14) | L1 (1,000,148 cases, 0 mismatches) | |
| 0x536DC0 | CEntryInfoNode::operator new | cdecl | pools::entry_info_node_operator_new | bytes | `jmp 0x536D10` with ms_pEntryInfoNodePool | L1 (1,000,148 cases, 0 mismatches) | L1 with the pool global patched to scratch |
| 0x536DD0 | CEntryInfoNode::operator delete | cdecl | pools::entry_info_node_operator_delete | bytes | *new*: conditional first_free write, no fill/null check | L1 (1,000,030 cases, 0 mismatches) | |
| 0x536E10 | CEntryInfoList::Flush | thiscall | pools::entry_info_list_flush | bytes | inlined node delete as above | implemented-needs-L2 | L2 sig: thiscall this=CEntryInfoList* (no layouts type), 0 stack args, ret Void; rwset: list head, node +0xC/+0x10 links, entry-info pool map/first_free |
| 0x5386f0 | CFileMgr::Initialise | cdecl | `filemgr::initialise` | gr + bytes | `_getcwd(root,128)` + `"\\"`; gr wide-char path not followed | implemented-needs-L2 | L2: write set ms_rootDirName |
| 0x538730 | CFileMgr::ChangeDir | cdecl | `filemgr::change_dir` | gr + bytes | *new*: no `memset(ms_dirName)`: a path without leading `\` is appended to the current dir; no length check | implemented-needs-L2 | L2: ret EAX (`_chdir`), chunk 0x402353 |
| 0x5387d0 | CFileMgr::SetDir | cdecl | `filemgr::set_dir` (no hook) | gr + bytes | *new*: no length check/EINVAL; plain `_chdir` | ported-unhookable | the inlined strcpy loop branches back to 0x5387D2, inside the stolen entry bytes |
| 0x538860 | CFileMgr::SetDirMyDocuments | cdecl | `filemgr::set_dir_my_documents` | gr + bytes | *new*: no length check | implemented-needs-L2 | L2: ret EAX |
| 0x538890 | CFileMgr::LoadFile | cdecl | `filemgr::load_file` | gr + bytes | gr-deviations `FileMgr.cpp:238` (original kept: ignores size, 0x4000 chunks, NUL terminator); *new*: -1 without fclose when fread < 0 | implemented-needs-L2 | L2: ret EAX |
| 0x538900 | CFileMgr::OpenFile | cdecl | `filemgr::open_file` | gr + bytes | `jmp fopen` 0x8232D8 (stub-like, hooked inline) | implemented-needs-L2 | L2: ret EAX |
| 0x538910 | CFileMgr::OpenFileForWriting | cdecl | `filemgr::open_file_for_writing` | gr + bytes | none ("wb") | implemented-needs-L2 | |
| 0x538930 | CFileMgr::OpenFileForAppending | cdecl | `filemgr::open_file_for_appending` | gr + bytes | none ("a") | implemented-needs-L2 | |
| 0x538950 | CFileMgr::Read | cdecl | `filemgr::read` | gr + bytes | none | implemented-needs-L2 | |
| 0x538970 | CFileMgr::Write | cdecl | `filemgr::write` | gr + bytes | none | implemented-needs-L2 | |
| 0x538990 | CFileMgr::Seek | cdecl | `filemgr::seek` | gr + bytes | returns `fseek != 0` (as gr) | implemented-needs-L2 | ret AL |
| 0x5389b0 | CFileMgr::ReadLine | cdecl | `filemgr::read_line` | gr + bytes | none | implemented-needs-L2 | ret AL |
| 0x5389d0 | CFileMgr::CloseFile | cdecl | `filemgr::close_file` | gr + bytes | `jmp fclose` | implemented-needs-L2 | |
| 0x5389e0 | CFileMgr::GetTotalSize | cdecl | `filemgr::get_total_size` | bytes (gr comment) | original uses the RW file interface (`RwEngineInstance+0xC4`, rwftell/rwfseek); gr uses ftell/fseek (its own comment says so) | implemented-needs-L2 | |
| 0x538a20 | CFileMgr::Tell | cdecl | `filemgr::tell` | bytes | as GetTotalSize (RW rwftell) | implemented-needs-L2 | |
| 0x538a50 | CFileMgr::GetErrorReadWrite | cdecl | `filemgr::get_error_read_write` | bytes | *new*: returns `_flag & 0x20` in EAX (gr: bool) | implemented-needs-L2 | reads FILE (heap) |
| 0x53BB50 | CGame::InitialiseOnceBeforeRW | cdecl, AL | `game::initialise_once_before_rw` | gr + bytes | none | implemented-needs-L2 (ret AL; callees CMemoryMgr::Init, CLocalisation::Initialise, CFileMgr::Initialise, CdStreamInit(5), CPad::Initialise; runs once at start-up) | |
| 0x53BB80 | CGame::ShutdownRenderWare | — | — | — | — | deferred: orchestrator (14 calls into RW, CLoadingScreen, CHud, CFont, CPlayerSkin, CVisibilityPlugins, D3DResourceSystem, CPostEffects) | |
| 0x53BC30 | CGame::FinalShutdown | cdecl | `game::final_shutdown` | gr + bytes | none | implemented-needs-L2 (void; shutdown, record at exit) | tail jump into CdStreamShutdown |
| 0x53BC60 | CGame::InitAfterLostFocus | cdecl | `game::init_after_lost_focus` | bytes (REA row) | n/a (not in gr) | implemented-needs-L2 (void; writes 0xBA677B) | reads menu +0xE9 and +0x5C |
| 0x53BC80 | CGame::Initialise | — | — | — | — | deferred: orchestrator, depends on other subsystems (Init1/2/3, CFileLoader::LoadLevel, CColAccel, CTheScripts, CCamera) | |
| 0x53BCF0 | CGame::ReInitGameObjectVariables | — | — | — | — | deferred: orchestrator (59 calls) | |
| 0x53BED0 | CGame::ReloadIPLs | cdecl | `game::reload_ipls` (no-op) | bytes | none | ported-unhookable (body is `ret`; relocation needs 5 bytes) | |
| 0x53BEE0 | CGame::Process | — | — | — | — | deferred: orchestrator (83 calls) | |
| 0x53C4A0 | CGame::CanSeeOutSideFromCurrArea | cdecl, EAX | `game::can_see_outside_from_curr_area` | gr + bytes | none (EAX fully 0/1: `xor eax,eax; sete al`, port returns u32) | implemented-needs-L2 (reads CGame::currArea 0xB72914) | |
| 0x53C4B0 | CGame::CanSeeWaterFromCurrArea | cdecl, EAX | `game::can_see_water_from_curr_area` | gr + bytes | original tests `currArea == 0 \|\| == 5` inline (gr calls CanSeeOutSideFromCurrArea; same result, no call) | implemented-needs-L2 | |
| 0x53C500 | CGame::TidyUpMemory | cdecl | `game::tidy_up_memory` | gr + bytes | *new* (call structure): original calls TidyUpD3DTextures(100)/TidyUpD3DIndexBuffers(200) itself, gr calls DrasticTidyUpMemory (matters for hook toggles only) | implemented-needs-L2 | FindPlayerPed(0) first |
| 0x53C550 | CGame::ShutDownForRestart | — | — | — | — | deferred: orchestrator (35 calls) | |
| 0x53C680 | CGame::InitialiseWhenRestarting | — | — | — | — | deferred: orchestrator (29 calls) | |
| 0x53C810 | CGame::DrasticTidyUpMemory | cdecl | `game::drastic_tidy_up_memory` | gr + bytes | none | implemented-needs-L2 | second call is a tail jump reusing the arg slot |
| 0x53C900 | CGame::Shutdown | — | — | — | — | deferred: orchestrator (68 calls) | |
| 0x53ced0 | CKeyGen::GetKey-sized | cdecl | `keygen::get_key_sized` | gr + bytes | none | L1 (1,000,075 cases, 0 mismatches) | table read 0x8CD068 allowed |
| 0x53cf00 | CKeyGen::GetKey-null-terminated | cdecl | `keygen::get_key` | gr + bytes | none | L1 (1,000,075 cases, 0 mismatches) | |
| 0x53cf30 | CKeyGen::GetUppercaseKey- | cdecl | `keygen::get_uppercase_key` | gr + bytes | CRT `toupper` 0x823A0E per byte (locale-aware) | implemented-needs-L2 | L2: ret EAX |
| 0x53cf70 | CKeyGen::AppendStringToKey | cdecl | `keygen::append_string_to_key` | gr + bytes | none (chunk 0x404785) | L1 (1,000,075 cases, 0 mismatches) | |
| 0x53cfa0 | CLoadMonitor::Constructor | thiscall | `loadmonitor::constructor` | gr + bytes | *new*: zeroes m_iCyclesHistory; leaves +0x08/+0x18/graph untouched | implemented-needs-L2 | this=CLoadMonitor, EAX=this |
| 0x53d030 | CLoadMonitor::BeginFrame | thiscall | `loadmonitor::begin_frame` | gr + bytes | none | implemented-needs-L2 | |
| 0x53d050 | CLoadMonitor::StartTimer | thiscall | `loadmonitor::start_timer` | gr + bytes | none | implemented-needs-L2 | 1 arg |
| 0x53d070 | CLoadMonitor::EndTimer | thiscall | `loadmonitor::end_timer` | gr + bytes | none | implemented-needs-L2 | 1 arg |
| 0x53d0b0 | CLoadMonitor::EndFrame | thiscall | `loadmonitor::end_frame` | bytes | *new*: thresholds 0.66666 (0x3F2AAA3B) / 0.33333 (0x3EAAAA3B) from .rdata, gr uses 2/3, 1/3; *new*: speed = sqrt of squares of each component*50 (gr: magnitude*50) | implemented-needs-L2 | reads ped/vehicle via FindPlayerPed, CTimer |
| 0x53d480 | CLoadMonitor::Render | thiscall | `loadmonitor::render` (no hook) | bytes | none | ported-unhookable | lone `ret` |
| 0x53D690 | App::DoRWStuffStartOfFrame | cdecl | — | bytes | — | blocked: computes `tan(FOV * k)` with FPTAN (0x53D6BD) for CameraSize; sa-math has no CPU-checked FPTAN model (Unicorn's is unfaithful, L2 route) | port the rest once an `X87::fptan` exists and is checked by `sa-oracle fidelity` |
| 0x53D7A0 | App::DoRWStuffStartOfFrame_Horizon | cdecl | — | bytes | — | blocked: FPTAN (0x53D7C1), as above | |
| 0x53D840 | App::DoRWStuffEndOfFrame | cdecl | platform::do_rw_stuff_end_of_frame | bytes | entry calls DebugDisplayTextBuffer, FlushObrsPrintfs, then reads `Scene.camera` through the protection chunk 0x4037CB (`[[0x1560754] + [0x58287B] + 1]` = 0xC1703C), computed at run time | implemented-needs-L2 | L2: cdecl, 0 args, ret Void |
| 0x53D870 | App::PluginAttach | cdecl | platform::plugin_attach | bytes | RpAnisotPluginAttach result ignored; byte results (`test al,al`) for CTxdStore/CCollisionPlugin/C2dEffect; last result normalised to 0/1 | implemented-needs-L2 | L2: ret EAX; runs once at start-up |
| 0x53D910 | Game::RwTerminate | cdecl | `game::rw_terminate` | gr + bytes | gr's `ResetGammaWhenExiting()` is inline in the original: if 0xC920C8, `IDirect3DDevice9::SetGammaRamp(dev, 0, 1, 0xC8D4C8)` (COM slot 0x54) when a device exists, then clear the flag | implemented-needs-L2 | |
| 0x53DF10 | Game::TheGame | cdecl | `game::the_game` | gr + bytes | gr NOTSA_LOG_DEBUG = original `printf("Into TheGame!!!\n")` (CRT 0x821982) | implemented-needs-L2 (unused by the game: never called) | |
| 0x53DF40 | Game::RenderScene | — | — | — | — | deferred: orchestrator (25 calls + 12 indirect, renderer) | |
| 0x53E160 | Game::RenderDebugShit | cdecl | `game::render_debug_shit` (no-op) | bytes | *new*: original is an empty `ret`; gr calls CTheScripts::RenderTheScriptDebugLines + DefinedState (NOTSA/!FINAL) | ported-unhookable (body is `ret`) | |
| 0x53E170 | Game::RenderEffects | — | — | — | — | deferred: orchestrator (24 render calls) | |
| 0x53E230 | Game::Render2dStuff | — | — | — | — | deferred: orchestrator (18 calls + 7 indirect, HUD/renderer) | |
| 0x53E530 | Game::RenderMenus | cdecl | `game::render_menus` | gr + bytes | none | implemented-needs-L2 | |
| 0x53E580 | Game::InitialiseGame | cdecl | `game::initialise_game` | gr + bytes | none | implemented-needs-L2 | |
| 0x53E600 | DoFade | cdecl | `game::do_fade` (+ `fade_colour`, `fade_rect`) | gr + bytes | gr-deviations `Draw.cpp:69` (FIX_BUGS rect): original = #else, memory order {-5, -5, W+5, H+5}; *new*: `JustLoadedDontFadeInYet`/`StillToFadeOut` compared `== 1` (byte), elapsed time compared unsigned | implemented-needs-L2 | x87: fild/fstp of W+5/H+5 (PC 24) |
| 0x53E770 | Game::FrontendIdle | — | — | — | — | deferred: orchestrator (25 calls) | |
| 0x53E920 | Game::Idle | — | — | — | — | deferred: orchestrator (46 calls, the frame loop) | |
| 0x53EC10 | App::AppEventHandler | cdecl | — | bytes | gr FIX_BUGS 4:3 → SCREEN_ASPECT_RATIO (keep 4/3 = 0x3FAAAAAB) | blocked: rsCAMERASIZE case uses FPTAN (0x53EC58); the other cases are calls into orchestrators (InitialiseOnceBeforeRW, RwInitialize, Idle, FrontendIdle, ...) | jump table 0x53ECDC/0x53ED08 read; entry jumps through chunk 0x4018CE |
| 0x53ED30 | TestModelIndices | cdecl | `game::test_model_indices` (no-op) | bytes | none | ported-unhookable (body is `ret`) | |
| 0x53F2D0 | Input::GetMouseSetUp | stdcall-like (hidden result pointer, `ret 4`), EAX = out | `input::get_mouse_set_up` | bytes + gr | button-count jump table 0x53F3A0 (1 -> lmb; 2 -> +rmb; 3..8 -> +mmb), 3 axes -> wheel | implemented-needs-L2 (stdcall, 1 word, EAX) | original UB: struct padding byte (+7) and DIDEVCAPS fields after a failed GetCapabilities are uninitialised stack (port 0) |
| 0x541330 | Common::LittleTest | cdecl | `common::little_test` | gr + bytes | none (increment in chunk 0x409B56) | implemented-needs-L2 | write g_nNumIm3dDrawCalls |
| 0x550080 | CPools::MakeSureSlotInObjectPoolIsEmpty | cdecl | pools::make_sure_slot_in_object_pool_is_empty | bytes | *new*: gr NOTSA_UNREACHABLE; original body (gr comment has the projectile test inverted) | implemented-needs-L2 | sig: cdecl 1 arg, Void; calls CWorld::Remove/Add, CProjectileInfo, CObject new/ctor, vtable[0] delete |
| 0x550170 | CPools::CheckBuildingAtomics | cdecl | pools::check_building_atomics (no #[hook]) | bytes | none (`or eax,-1; ret`) | ported-unhookable (L1 1,000,001 cases, 0 mismatches) | 4 bytes + padding: too short for the entry jump |
| 0x550180 | FUN_00550180 (CPool<CPtrNodeSingleLink> ctor) | thiscall, ret 8 | pools::ctor_ptr_node_single | bytes | *new*: ctor (no fill, +0x11 untouched, name unused); slot 8 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | name proposal `CPtrNodeSingleLinkPool::CPtrNodeSingleLinkPool` |
| 0x550250 | FUN_00550250 | thiscall, ret 8 | pools::ctor_ptr_node_double | bytes | ctor; slot 0xC | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CPtrNodeDoubleLinkPool ctor |
| 0x550320 | FUN_00550320 | thiscall, ret 8 | pools::ctor_entry_info_node | bytes | ctor; slot 0x14 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CEntryInfoNodePool ctor |
| 0x5503F0 | FUN_005503f0 | thiscall, ret 8 | pools::ctor_ped | bytes | ctor; slot 0x7C4 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CPedPool ctor |
| 0x5504C0 | FUN_005504c0 | thiscall, ret 8 | pools::ctor_vehicle | bytes | ctor; slot 0xA18 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CVehiclePool ctor |
| 0x550570 | FUN_00550570 | thiscall, ret 8 | pools::ctor_building | bytes | ctor; slot 0x38 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CBuildingPool ctor |
| 0x550640 | FUN_00550640 | thiscall, ret 8 | pools::ctor_object | bytes | ctor; slot 0x19C | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CObjectPool ctor |
| 0x5506F0 | FUN_005506f0 | thiscall, ret 8 | pools::ctor_dummy | bytes | ctor; slot 0x38 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CDummyPool ctor |
| 0x5507C0 | FUN_005507c0 | thiscall, ret 8 | pools::ctor_col_model | bytes | ctor; slot 0x30 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CColModelPool ctor |
| 0x550890 | FUN_00550890 | thiscall, ret 8 | pools::ctor_task | bytes | ctor; slot 0x80 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CTaskPool ctor |
| 0x550960 | FUN_00550960 | thiscall, ret 8 | pools::ctor_event | bytes | ctor; slot 0x44 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CEventPool ctor |
| 0x550A30 | FUN_00550a30 | thiscall, ret 8 | pools::ctor_point_route | bytes | ctor; slot 0x64 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CPointRoutePool ctor |
| 0x550B00 | FUN_00550b00 | thiscall, ret 8 | pools::ctor_patrol_route | bytes | ctor; slot 0x1A4 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CPatrolRoutePool ctor |
| 0x550BD0 | FUN_00550bd0 | thiscall, ret 8 | pools::ctor_node_route | bytes | ctor; slot 0x24 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CNodeRoutePool ctor |
| 0x550CA0 | FUN_00550ca0 | thiscall, ret 8 | pools::ctor_task_allocator | bytes | ctor; slot 0x20 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CTaskAllocatorPool ctor |
| 0x550D70 | FUN_00550d70 | thiscall, ret 8 | pools::ctor_ped_intelligence | bytes | ctor; slot 0x294 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CPedIntelligencePool ctor |
| 0x550E40 | FUN_00550e40 | thiscall, ret 8 | pools::ctor_ped_attractor | bytes | ctor; slot 0xC4 | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | CPedAttractorPool ctor |
| 0x550F10 | CPools::Initialise | cdecl | pools::initialise | bytes | sizes/names as pushed; null stored when new fails | implemented-needs-L2 | runs once at boot: needs a record from process start; calls the 17 ctors through orig! |
| 0x551950 | CPools::CheckPoolsEmpty | cdecl | pools::check_pools_empty | bytes | backwards walk (gr upwards, `maybe` row confirmed); CRT printf | implemented-needs-L2 | printf through orig! (0x821982) |
| 0x5519F0 | CPools::ShutDown | cdecl | pools::shut_down | bytes | *new*: conditional clear (size > 0), first_free = 0, globals not nulled, printf | implemented-needs-L2 | |
| 0x552240 | CPtrNodeSingleLinkPool::New | thiscall | pools::ptr_node_single_pool_new | bytes | slot 8 | L1 (1,000,148 cases, 0 mismatches) | |
| 0x5522E0 | CPtrNodeDoubleLinkPool::New | thiscall | pools::ptr_node_double_pool_new | bytes | slot 0xC | L1 (1,000,148 cases, 0 mismatches) | |
| 0x5525A0 | CQuadTreeNode<T>::FindSector-rect | thiscall, ret 4 | quadtree::find_sector_rect (find_sector_rect_of) | bytes | x87 compare branches as compiled | L1 (1,060,000 cases, 0 mismatches; SNaN cases CPU-decided, Unicorn = CPU) | |
| 0x552640 | CQuadTreeNode<T>::FindSector-vec | thiscall, ret 4 | quadtree::find_sector_vec | bytes | mids kept in ST (unrounded) | L1 (1,060,000 cases, 0 mismatches; SNaN cases CPU-decided, Unicorn = CPU) | |
| 0x5526A0 | CQuadTreeNode<T>::InSector | thiscall, ret 8, AL | quadtree::in_sector | bytes | *new*: sectors 1/3 keep mid x in ST0; out-of-range sector compares the whole rect | L1 (1,060,000 cases, 0 mismatches; SNaN cases CPU-decided, Unicorn = CPU) | |
| 0x552870 | CQuadTreeNode<T>::GetAll | thiscall, ret 4 | quadtree::get_all | bytes | none | implemented-needs-L2 | calls CPtrListSingleLink::AddItem |
| 0x5528C0 | CQuadTreeNode<T>::GetAllMatching-rect | thiscall, ret 8 | quadtree::get_all_matching_rect | bytes | none | implemented-needs-L2 | |
| 0x552930 | CQuadTreeNode<T>::GetAllMatching-vec | thiscall, ret 8 | quadtree::get_all_matching_vec | bytes | iterative descent | implemented-needs-L2 | |
| 0x552980 | CQuadTreeNode<T>::ForAllMatching-rect | thiscall, ret 8 | quadtree::for_all_matching_rect | bytes | callback cdecl(rect, item) via orig! `dyn` | implemented-needs-L2 | |
| 0x5529F0 | CQuadTreeNode<T>::ForAllMatching-vec | thiscall, ret 8 | quadtree::for_all_matching_vec | bytes | iterative | implemented-needs-L2 | |
| 0x552A40 | CQuadTreeNode<T>::DeleteItem- | thiscall, ret 4 | quadtree::delete_item | bytes | *new*: list DeleteItem only after finding the item | implemented-needs-L2 | |
| 0x552A90 | CQuadTreeNode<T>::DeleteItem-rect | thiscall, ret 8 | quadtree::delete_item_rect | bytes | *new*: InSector for every sector, no child null check, recursion with rect | implemented-needs-L2 | |
| 0x552AF0 | FUN_00552af0 (CQuadTreeNode pool ctor) | thiscall, ret 8 | pools::ctor_quad_tree_node | bytes | ctor; slot 0x28; plain `operator new` 0x82119A | L1 (1,000,015 cases, 0 mismatches; new[] stubbed) | |
| 0x552C00 | CQuadTreeNode<T>::InitPool | cdecl | quadtree::init_pool | bytes | none | implemented-needs-L2 | |
| 0x552CD0 | CQuadTreeNode<T>::AddItem | thiscall, ret 8 | quadtree::add_item | bytes | leaf insertion inlined; child rect jump table 0x552E50 | implemented-needs-L2 | |
| 0x561B10 | CTimer::Update | cdecl | — | — | — | deferred: frame boundary hook owned by sa-hook | |
| 0x56d070 | CIniFile::LoadIniFile | cdecl | `inifile::load_ini_file` | gr + bytes | *new*: NaN multiplier kept (gr turns it into 0.5) | implemented-needs-L2 | |
| 0x5719B0 | CReferences::Init | cdecl | references::init | bytes | last entry's next first set one past the array, then 0 | implemented-needs-L2 | writes 0xB9B9A8..0xBA176C |
| 0x5719E0 | CReferences::ListSize | cdecl | references::list_size (count_links) | bytes | none | L1 (1,000,002 cases, 0 mismatches) | |
| 0x571AD0 | CReferences::RemoveReferencesToPlayer | cdecl | references::remove_references_to_player | bytes (+ chunk 0x403768) | FindPlayerPed re-called for every use as compiled | implemented-needs-L2 | |
| 0x571CC0 | CReferences::PruneAllReferencesInWorld | cdecl | references::prune_all_references_in_world | bytes | backwards walks (gr `maybe` rows confirmed) | implemented-needs-L2 | |
| 0x57C5E0 | CMenuManager::NoDiskInDriveMessage | thiscall | `game::no_disk_in_drive_message` | bytes (REA row) | n/a | implemented-needs-L2 (this = CMenuManager, void) | GetPad(0) result unused; pad words 0xB733A8/0xB73138 |
| 0x5a4150 | Common::WriteRaster | cdecl | `common::write_raster` | gr + bytes | none | implemented-needs-L2 | RW calls |
| 0x5BA060 | CGame::ValidateVersion | cdecl | `game::validate_version` | gr + bytes | gr-deviations rows for 0x5BA060 (NOTSA_UNREACHABLE: original = LoadingScreen("Invalid version") + `jmp $`); *new*: open check is `FILE* == -1`, not null; `strncpy` (gr strncpy_s) | implemented-needs-L2 | local buffer zeroed (original uninitialised; only visible for unterminated text) |
| 0x5BA120 | CGame::D3DDeviceRestoreCallback | cdecl | `game::d3d_device_restore_callback` | gr + bytes | none | implemented-needs-L2 | previous callback through `orig! cell 0xBD0114` |
| 0x5BA160 | CGame::InitialiseEssentialsAfterRW | cdecl, AL | `game::initialise_essentials_after_rw` | gr + bytes | none (CCarFX tested as EAX, CGrass as AL, CCustomBuilding as EAX) | implemented-needs-L2 | |
| 0x5BA1A0 | CGame::Init2 | — | — | — | — | deferred: orchestrator (67 calls) | |
| 0x5BA400 | CGame::Init3 | — | — | — | — | deferred: orchestrator (11 calls: loading screen, pads, interior/proc-object/water-creature/shadow managers) | |
| 0x5BA520 | Light::LightsCreate | cdecl, EAX = world | `light::lights_create` | bytes + gr | returns `world` (gr void) | implemented-needs-L2 (cdecl, 1, EAX) | original UB: colour alpha uninitialised (port 0) |
| 0x5BD600 | CGame::InitialiseRenderWare | — | — | — | — | deferred: orchestrator (28 calls, RW setup) | |
| 0x5BF390 | Game::RwInitialize | cdecl, EAX | `game::rw_initialize` | gr + bytes | EAX = `movzx eax, al` of InitialiseRenderWare | implemented-needs-L2 | |
| 0x5BF3B0 | Game::GameInit | cdecl | `game::game_init` | gr + bytes | gr VERIFY(RwInitialize(nullptr)): original inlines it and ignores the result | implemented-needs-L2 | srand(RsTimer()) via `crt::srand` |
| 0x5BF840 | CGame::Init1 | — | — | — | — | deferred: orchestrator (54 calls) | |
| 0x5BFA90 | CGame::InitialiseCoreDataAfterRW | — | — | — | — | deferred: orchestrator (11 calls into handling, surfaces, ped stats, time/pop cycle, audio, bone/IK managers) | |
| 0x5D0880 | CPools::Save | cdecl, AL | pools::save | bytes | returns the callee's AL | implemented-needs-L2 | |
| 0x5D0890 | CPools::Load | cdecl, AL | pools::load | bytes | as Save | implemented-needs-L2 | |
| 0x5D2A20 | CPools::LoadVehiclePool | cdecl, AL | pools::load_vehicle_pool | bytes (+ chunk 0x403AC1) | *new*: NumRandomCars via protection-chunk address; bike flag on null | implemented-needs-L2 | count local uninitialised in the original if LoadDataFromWorkBuffer does not write it (port: 0) |
| 0x5D2D70 | CPools::LoadPedPool | cdecl, AL | pools::load_ped_pool | bytes | *new*: previous ped reused for non-player types | implemented-needs-L2 | same uninitialised-count note |
| 0x5D4800 | CPools::SaveVehiclePool | cdecl, AL | pools::save_vehicle_pool | bytes (gr unreversed) | — | implemented-needs-L2 | |
| 0x5D4940 | CPools::SaveObjectPool | cdecl, AL | pools::save_object_pool | bytes (gr unreversed) | — | implemented-needs-L2 | the 0x30-byte save buffer is uninitialised stack before Construct in the original (port: zeroed) |
| 0x5D4A40 | CPools::LoadObjectPool | cdecl, AL | pools::load_object_pool | bytes | GetAtRefNoChecks = sign test only | implemented-needs-L2 | |
| 0x5D4B40 | CPools::SavePedPool | cdecl, AL | pools::save_ped_pool | bytes (gr unreversed) | — | implemented-needs-L2 | |
| 0x605EC0 | CPedIntelligencePool::New | thiscall | pools::ped_intelligence_pool_new | bytes | slot 0x294 | L1 (1,000,148 cases, 0 mismatches) | |
| 0x60F8C0 | FUN_0060f8c0 | cdecl | pools::ret_60f8c0 (no #[hook]) | bytes | lone `ret` | ported-unhookable | |
| 0x6193F0 | Rs::RsKeyFromScanCode | cdecl, AL | platform::rs_key_from_scan_code | bytes | — | L1 (--cases 1000000, 0 mismatches, Unicorn) | tables KeysNormal 0x8D2C00 / KeysShifted 0x8D2D00 embedded as globals' initial values (checked against the exe) |
| 0x619410 | Rs::RsTimer | cdecl | platform::rs_timer | bytes | `jmp psTimer` (expect 6 bytes so the stub is installed inline, not as a .HOODLUM stub) | implemented-needs-L2 | ret EAX |
| 0x619420 | Rs::RsWindowSetText | cdecl | platform::rs_window_set_text | bytes | jmp psWindowSetText | implemented-needs-L2 | 1 arg, EAX |
| 0x619430 | Rs::RsPathGetSeparator | cdecl, AL | platform::rs_path_get_separator | bytes | jmp psPathGetSeparator | implemented-needs-L2 | |
| 0x619450 | Rs::RsCameraBeginUpdate | cdecl | platform::rs_camera_begin_update | bytes | jmp | implemented-needs-L2 | |
| 0x619460 | Rs::RsAlwaysOnTop | cdecl | platform::rs_always_on_top | bytes | jmp | implemented-needs-L2 | |
| 0x619470 | Rs::RsRegisterImageLoader | cdecl | platform::rs_register_image_loader | bytes | — | L1 (--cases 1000000, 0 mismatches, Unicorn) | returns 1 |
| 0x619480 | Rs::RsSetDebug | cdecl | platform::rs_set_debug | bytes | — | L1 (--cases 1000000, 0 mismatches, Unicorn) | returns 1 |
| 0x619490 | Rs::RsMouseSetVisibility | cdecl | platform::rs_mouse_set_visibility | bytes | jmp | implemented-needs-L2 | |
| 0x6194A0 | Rs::RsMouseSetPos | cdecl | platform::rs_mouse_set_pos | bytes | jmp | implemented-needs-L2 | |
| 0x6194B0 | Rs::RsSelectDevice | cdecl | platform::rs_select_device | bytes | jmp | implemented-needs-L2 | |
| 0x6194C0 | Rs::RsInputDeviceAttach | cdecl | platform::rs_input_device_attach | bytes | — | implemented-needs-L2 | 2 args, EAX; writes RsGlobal device handler then used |
| 0x619510 | Rs::rsCommandLine | EAX arg | platform::rs_command_line | bytes | — | ported-unhookable: argument in EAX (whole-program-optimised convention) | |
| 0x619530 | Rs::rsPreInitCommandLine | EAX arg | platform::rs_pre_init_command_line | bytes | `repe cmpsb` over 5 bytes of "-vms" | ported-unhookable: argument in EAX | |
| 0x619560 | Rs::RsKeyboardEventHandler | cdecl | platform::rs_keyboard_event_handler | bytes | entry `jmp` into protection chunk 0x40591D: reads `[[0x15607D4] + [0x648111] + 1]` = keyboard.used (0xC1705C), computed at run time; tail-jumps to the handler | implemented-needs-L2 | 2 args, EAX |
| 0x619580 | Rs::RsMouseEventHandler | cdecl | platform::rs_mouse_event_handler | bytes | — | implemented-needs-L2 | |
| 0x6195A0 | Rs::RsPadEventHandler | cdecl | platform::rs_pad_event_handler | bytes | — | implemented-needs-L2 | |
| 0x6195C0 | Rs::RsPathnameDestroy | cdecl | platform::rs_pathname_destroy | bytes | jmp | implemented-needs-L2 | |
| 0x6195D0 | Rs::RsPathnameCreate | cdecl | platform::rs_pathname_create | bytes | jmp | implemented-needs-L2 | |
| 0x6195E0 | Rs::RsRwTerminate | cdecl | platform::rs_rw_terminate | bytes | — | implemented-needs-L2 | |
| 0x6195F0 | Rs::RsTerminate | cdecl | platform::rs_terminate | bytes | jmp psTerminate | implemented-needs-L2 | |
| 0x619600 | Rs::RsInitialize | cdecl | platform::rs_initialize | bytes | RsGlobal.ps not touched; store order of the bytes | implemented-needs-L2 | tail-calls psInitialize (deferred, original in phase A) |
| 0x619AF0 | Rs::RsGrabScreen | cdecl | platform::rs_grab_screen | bytes | jmp | implemented-needs-L2 | |
| 0x619B00 | Rs::RsErrorMessage | cdecl | platform::rs_error_message | bytes | keyboard event 0x1D with a local 0x420 first | implemented-needs-L2 | |
| 0x619B30 | Rs::RsWarningMessage | cdecl | platform::rs_warning_message | bytes | as above | implemented-needs-L2 | |
| 0x619B60 | Rs::RsEventHandler | cdecl | platform::rs_event_handler | bytes | jump table 0x619C40/0x619C64; quit set on 0x1E regardless of the app result | implemented-needs-L2 | 2 args, EAX |
| 0x619C90 | Rs::RsRwInitialize | cdecl | — | bytes | — | deferred: orchestrator (RwEngineInit/Open/Start, psSelectDevice, 4 AppEventHandler events, mipmapping) | |
| 0x6FAB30 | Light::GetDayNightBalance | cdecl, ST0 | `light::get_day_night_balance` | bytes | *new*: multiplies by the f32 constant 1/60 (0x859044) where gr divides by 60; GetMinutesToday inlined | L1 (1,002,000, 0; ST0 compared exactly; clock operands patched into scratch) | |
| 0x718600 | Common::AsciiToGxtChar | cdecl | `common::ascii_to_gxt_char` | gr + bytes | none | L1 (1,000,064 cases, 0 mismatches) | |
| 0x7186e0 | Common::MakeUpperCase- | cdecl | `common::make_upper_case` | bytes | *new*: returns the start of dst (gr: end); ASCII-only uppercase | L1 (1,000,064 cases, 0 mismatches) | |
| 0x718710 | Common::MakeUpperCase-in | cdecl | `common::make_upper_case_in` | bytes | *new*: EAX not set (gr returns dest); void port | L1 (1,000,064 cases, 0 mismatches) | |
| 0x71D720 | CMBlur::MotionBlurClose | cdecl | platform::motion_blur_close | bytes | — | L1 (--cases 1000000, 0 mismatches, Unicorn) | returns 1 |
| 0x71da00 | Common::CalcScreenCoors-VVff | cdecl | `common::calc_screen_coors_scaled` | bytes | *new*: `sx = W*(1/z)` stored then `/FOV*70` (gr: `W*(1/z/FOV*70)`) | implemented-needs-L2 | reads TheCamera view matrix, RsGlobal, CDraw::ms_fFOV |
| 0x71dab0 | Common::CalcScreenCoors-VV | cdecl | `common::calc_screen_coors` | gr + bytes | none beyond x87 order | implemented-needs-L2 | |
| 0x71db30 | FUN_0071db30 (gr `DoesInfiniteLineCrossFiniteLine`) | cdecl | `common::does_infinite_line_cross_finite_line` | bytes | returns EAX 0/1 | L1 (10^6 generated + 100,000 edge, 0 mismatches; 116,280 SNaN cases CPU-decided) | |
| 0x71db80 | Common::DoesInfiniteLineTouchScreen | cdecl | `common::does_infinite_line_touch_screen` | bytes | *new*: strict on-screen test, edges inlined with f32-stored intermediates | L1 (10^6 generated + edges per screen size, RsGlobal patched to 640x480 and 1366x768, 0 mismatches; 36,320 SNaN cases CPU-decided each) | the global read itself is L2 territory |
| 0x71e050 | Common::IsPointInsideLine | cdecl | `common::is_point_inside_line` | gr + bytes | *new*: NaN → true (gr `>=` → false) | L1 (10^6 generated + 100,000 edge, 0 mismatches; 102,592 SNaN cases CPU-decided) | |
| 0x72f180 | CMemoryHeap::Init | thiscall | `memory::heap_init` | bytes | *new*: FreeListBegin.prev/FreeListEnd.next not written; m_paCommonSizes cleared only if bInitSizes | implemented-needs-L2 | this = CMemoryHeap (not in layouts.csv) |
| 0x72f3b0 | CMemoryMgr::Init | cdecl | `memory::init` (no hook) | bytes | none | ported-unhookable | lone `ret` |
| 0x72f3c0 | (missed by ghidra: cat7-call) | cdecl | `memory::nullsub_72f3c0` (no hook) | bytes | none | ported-unhookable | lone `ret` |
| 0x72f420 | CMemoryMgr::Malloc- | cdecl | `memory::mgr_malloc` | bytes | none | implemented-needs-L2 | |
| 0x72f430 | CMemoryMgr::Free | cdecl | `memory::mgr_free` | bytes | none (`jmp free`) | implemented-needs-L2 | |
| 0x72f440 | CMemoryMgr::Realloc- | cdecl | `memory::mgr_realloc` | bytes | none | implemented-needs-L2 | |
| 0x72f460 | CMemoryMgr::Calloc- | cdecl | `memory::mgr_calloc` | bytes | none | implemented-needs-L2 | |
| 0x72f480 | CMemoryMgr::InitScratchPad | cdecl | `memory::init_scratch_pad` | bytes | none | implemented-needs-L2 | |
| 0x72f4c0 | CMemoryMgr::MallocAlign- | cdecl | `memory::malloc_align` | bytes | no null check | implemented-needs-L2 | |
| 0x72f4f0 | CMemoryMgr::FreeAlign | cdecl | `memory::free_align` | bytes | none | implemented-needs-L2 | |
| 0x72FD90 | CGame::CameraDestroy | cdecl | `game::camera_destroy` | gr + bytes | none | implemented-needs-L2 | RwCamera +0x04 parent, +0x60 frameBuffer, +0x64 zBuffer |
| 0x731650 | CTxdStore::PluginAttach | cdecl, AL | platform::txd_store_plugin_attach | bytes | — | implemented-needs-L2 | writes ms_txdPluginOffset 0xC88018 |
| 0x734610 | CreateDebugFont | cdecl | platform::create_debug_font | bytes | — | ported-unhookable: lone `ret` | |
| 0x734620 | DestroyDebugFont | cdecl | platform::destroy_debug_font | bytes | — | ported-unhookable: lone `ret` | |
| 0x734640 | FlushObrsPrintfs | cdecl | platform::flush_obrs_printfs | bytes | — | ported-unhookable: lone `ret` | |
| 0x734650 | App::DefinedState | cdecl | `input::defined_state` | bytes | none; fog colour `((((r \| 0xFFFFFF00) << 8) \| g) << 8) \| b` from words 0xB7C4CA/CC/CE | implemented-needs-L2 (void) | RwRenderStateSet via `RwEngineInstance+0x20` (`orig! table`) |
| 0x734750 | App::DefinedState2d | cdecl | `input::defined_state_2d` | bytes | none | implemented-needs-L2 (void) | |
| 0x7354E0 | Light::SetLightsWithTimeOfDayColour | cdecl | `light::set_lights_with_time_of_day_colour` | bytes + gr | LightsMult read before each CTimeCycle getter call; directional matrix right=(Z x sun).N x sun, up=(Z x sun).N, at=-sun | implemented-needs-L2 (void) | original UB: RwMatrix flags/pads/pos uninitialised stack passed to RwFrameTransform(REPLACE) (port 0) |
| 0x735720 | Light::LightsEnable | cdecl | `light::lights_enable` (no hook) | bytes | — | ported-unhookable (lone `ret`) | |
| 0x735730 | Light::LightsDestroy | cdecl, EAX = world | `light::lights_destroy` | bytes | returns `world` | implemented-needs-L2 (cdecl, 1, EAX) | |
| 0x7357E0 | Light::WorldReplaceNormalLightsWithScorched | cdecl | `light::world_replace_normal_lights_with_scorched` | bytes | `DeActivateDirectional` inlined | implemented-needs-L2 (cdecl, 2, void) | original UB: colour alpha uninitialised (port 0) |
| 0x735820 | Light::WorldReplaceScorchedLightsWithNormal | cdecl | `light::world_replace_scorched_lights_with_normal` | bytes | ActivateDirectional inlined | implemented-needs-L2 (cdecl, 1, void) | |
| 0x735840 | Light::AddAnExtraDirectionalLight | cdecl | `light::add_an_extra_directional_light` | bytes | *new*: weakest-slot search starts at -1, so a light weaker than all six is dropped (gr app_light.cpp: msIdx starts at 0 -> slot 0); CanSeeOutSideFromCurrArea inlined | implemented-needs-L2 (cdecl, 7, void) | original UB: colour alpha uninitialised (port 0) |
| 0x7359E0 | Light::RemoveExtraDirectionalLights | cdecl | `light::remove_extra_directional_lights` | bytes | none | implemented-needs-L2 (cdecl, 1, void) | |
| 0x735A20 | Light::SetAmbientAndDirectionalColours | cdecl | `light::set_ambient_and_directional_colours` | bytes | x87 `forFrame * mult` | implemented-needs-L2 (cdecl, 1, void) | |
| 0x735AB0 | Light::SetFlashyColours | cdecl | `light::set_flashy_colours` | bytes | *new* (minor): copies only rgb of DirectionalLightColourForFrame (gr copies the struct incl. alpha) | implemented-needs-L2 (cdecl, 1, void) | |
| 0x735B40 | Light::SetFlashyColours_Mild | cdecl | `light::set_flashy_colours_mild` | bytes | as SetFlashyColours (0.65 / 0.9) | implemented-needs-L2 (cdecl, 1, void) | |
| 0x735BD0 | Light::SetBrightMarkerColours | cdecl | `light::set_bright_marker_colours` | bytes | none | implemented-needs-L2 (cdecl, 1, void) | |
| 0x735C40 | Light::ReSetAmbientAndDirectionalColours | cdecl | `light::reset_ambient_and_directional_colours` | bytes | none | implemented-needs-L2 (void) | |
| 0x735C70 | Light::DeActivateDirectional | cdecl | `light::deactivate_directional` | bytes | entry `jmp` into obfuscation chunk 0x405BCF that computes `*0xC886EC` (= pDirect) | implemented-needs-L2 (void) | hooked inline with `expect` = the 10 bytes up to `ret` |
| 0x735C80 | Light::ActivateDirectional | cdecl | `light::activate_directional` | bytes | none | implemented-needs-L2 (void) | |
| 0x735C90 | Light::SetAmbientColoursToIndicateRoadGroup | cdecl | `light::set_ambient_colours_to_indicate_road_group` | bytes | *new*: table index is `idx % 7` (signed idiv remainder; negative idx reads into FullLight) — gr uses `idx & 8` | implemented-needs-L2 (cdecl, 1, void) | |
| 0x735D10 | Light::SetFullAmbient | cdecl | `light::set_full_ambient` | bytes | none | implemented-needs-L2 (void) | |
| 0x735D30 | Light::SetAmbientColours-void | cdecl | `light::set_ambient_colours` | bytes | none | implemented-needs-L2 (void) | |
| 0x735D50 | Light::SetAmbientColours-color | cdecl | `light::set_ambient_colours_color` | bytes | gr assert(color) not in the original (gr-deviations: harmless) | implemented-needs-L2 (cdecl, 1, void) | |
| 0x735D70 | Light::SetDirectionalColours | cdecl | `light::set_directional_colours` | bytes | gr assert (harmless) | implemented-needs-L2 (cdecl, 1, void) | |
| 0x735D90 | Light::SetLightColoursForPedsCarsAndObjects | cdecl | `light::set_light_colours_for_peds_cars_and_objects` | bytes | *new*: adds m_BrightnessAddedToAmbientRed/Green/Blue (0xB79E38/34/30) per channel, red/green to the unrounded x87 products (gr adds Red to all three) | implemented-needs-L2 (cdecl, 1, void) | arithmetic helper host-tested |
| 0x735E40 | Light::SetLightsForInfraredVisionHeatObjects | cdecl | `light::set_lights_for_infrared_vision_heat_objects` | bytes | none | implemented-needs-L2 (void) | |
| 0x735E70 | Light::StoreAndSetLightsForInfraredVisionHeatObjects | cdecl | `light::store_and_set_lights_for_infrared_vision_heat_objects` | bytes | heat-object setter inlined | implemented-needs-L2 (void) | |
| 0x735EF0 | Light::RestoreLightsForInfraredVisionHeatObjects | cdecl | `light::restore_lights_for_infrared_vision_heat_objects` | bytes | none | implemented-needs-L2 (void) | |
| 0x735F20 | Light::SetLightsForInfraredVisionDefaultObjects | cdecl | `light::set_lights_for_infrared_vision_default_objects` | bytes | none | implemented-needs-L2 (void) | |
| 0x735F70 | Light::SetLightsForNightVision | cdecl | `light::set_lights_for_night_vision` | bytes | none | implemented-needs-L2 (void) | |
| 0x743DF0 | Input::HandleKeyDown | cdecl, EAX=1 | `input::handle_key_down` | bytes (jump table 0x7442E0 extracted) | `GetPad(0)` called and discarded; rsNULL returns before the pad part; plain keys `code < 255` signed (negative codes write before `standardKeys`) | implemented-needs-L2 (cdecl, 1, EAX) | writes `CPad::TempKeyState` and pad 1's `PCTempKeyState` |
| 0x7443C0 | Input::HandleKeyUp | cdecl, EAX=1 | `input::handle_key_up` | bytes (jump table 0x744798, identical mapping) | as HandleKeyDown; padNumber test in chunk 0x407F85 | implemented-needs-L2 (cdecl, 1, EAX) | |
| 0x744880 | Input::KeyboardHandler | cdecl | `input::keyboard_handler` | bytes | none | implemented-needs-L2 (cdecl, 2, EAX) | callees through `orig!` |
| 0x7448B0 | Input::HandlePadButtonDown | custom: argument in EAX | `input::handle_pad_button_down` (no hook) | bytes | *new*: pad number = `CPad::padNumber != 0 ? 1 : 0` (gr: `padID`); every button 1..16 is reported, with button 0 when not down (gr: only pressed buttons) | ported-unhookable (EAX argument: no supported convention) | covered through PadHandler's L2 |
| 0x744930 | Input::HandlePadButtonUp | custom: argument in EAX | `input::handle_pad_button_up` (no hook) | bytes | *new*: as Down (pad 0/1, button 0 reported for buttons still down; buttons 2..16) | ported-unhookable (EAX argument) | camera modes 0x11/0xF of `TheCamera.m_aCams[m_nActiveCam]` |
| 0x7449F0 | Input::PadHandler | cdecl | `input::pad_handler` | bytes | tail-jumps with EAX = param | implemented-needs-L2 (cdecl, 2, EAX) | calls the two EAX-convention handlers' Rust ports directly (they have no hook, so no toggle is bypassed) |
| 0x744A20 | Input::AttachInputDevices | cdecl, EAX=1 | `input::attach_input_devices` | bytes | none | implemented-needs-L2 (cdecl, 0, EAX) | handlers passed as original addresses 0x744880/0x7449F0 |
| 0x744A70 | Securom::SecuromDisplayInit | cdecl | platform::securom_display_init | bytes | — | implemented-needs-L2 | both 32-int arrays = 1000 |
| 0x744AE0 | Securom::SecuromStateDisplay | cdecl | platform::securom_state_display | bytes | gr notsa::format_to_sz: original CRT sprintf + AsciiToGxtChar | implemented-needs-L2 | |
| 0x744D50 | CPad::IsMenuKeyJustPressed | cdecl, EAX | `input::is_menu_key_just_pressed` | bytes | *new*: `lalt` **or `ralt`** just pressed (gr Pad.h:266: lalt only) | L1 (1,002,000, 0; gr's version caught by `c2_specs_pass_and_discriminate`) | |
| 0x744D90 | CPad::IsTabJustPressed | cdecl, EAX | `input::is_tab_just_pressed` | bytes | none | L1 (1,002,000, 0) | |
| 0x744FB0 | CFileMgr::InitUserDirectories | cdecl | platform::init_user_directories | bytes | gr NOTSA wide-char version not followed; samDesired = KEY_READ pushed through protection chunk 0x406807 (`[0x156081C] - [0x53D84A]`) | implemented-needs-L2 | ret EAX = 0xC92368 |
| 0x7451B0 | Ps::psWindowSetText | cdecl | platform::ps_window_set_text | bytes | — | implemented-needs-L2 | SetWindowTextA site 0x7451BE |
| 0x7451D0 | Ps::psErrorMessage | cdecl | platform::ps_error_message | bytes | — | implemented-needs-L2 | MessageBoxA site 0x7451E2 |
| 0x7451F0 | Ps::psWarningMessage | cdecl | platform::ps_warning_message | bytes | — | implemented-needs-L2 | MessageBoxA site 0x745202 |
| 0x745210 | Ps::psCameraBeginUpdate | cdecl | platform::ps_camera_begin_update | bytes | uses Scene.camera, not the argument (as gr) | implemented-needs-L2 | |
| 0x745240 | Ps::psCameraShowRaster | cdecl | platform::ps_camera_show_raster | bytes | — | implemented-needs-L2 | ret EAX (RwCamera*) |
| 0x745270 | Ps::psTimer | cdecl | platform::ps_timer | bytes | *new*: original = timeGetDevCaps / timeBeginPeriod / timeGetTime / timeEndPeriod (gr `OS_TimeMS()`) | implemented-needs-L2 | timeGetTime through site 0x74528C (sa-hook determinism redirects it) |
| 0x7452B0 | Ps::psGrabScreen | cdecl | platform::ps_grab_screen | bytes | gr FIX_BUGS blocks not followed: surface never unlocked/released; rows written contiguously (image stride ignored) | implemented-needs-L2 | |
| 0x7453E0 | Ps::psMouseSetVisibility | cdecl | platform::ps_mouse_set_visibility | bytes | — | implemented-needs-L2 | |
| 0x7453F0 | Ps::psMouseSetPos | cdecl | platform::ps_mouse_set_pos | bytes | `_ftol` x4 (sa_math X87::ftol), `fild` back to lastMousePos | implemented-needs-L2 | |
| 0x745470 | Ps::psPathnameCreate | cdecl | platform::ps_pathname_create | bytes | *new*: original uses RwEngineInstance strlen/strcpy/strchr (+0x120/+0xF8/+0x10C) and calls CMemoryMgr::Malloc(len+1, 0) with RwMalloc's hint left on the stack; writes '\\' directly (gr calls psPathGetSeparator) | implemented-needs-L2 | |
| 0x7454E0 | Ps::psPathnameDestroy | cdecl | platform::ps_pathname_destroy | bytes | RwFree through RwEngineInstance+0x138 | implemented-needs-L2 | |
| 0x745500 | Ps::psPathGetSeparator | cdecl, AL | platform::ps_path_get_separator | bytes | — | L1 (--cases 1000000, 0 mismatches, Unicorn); unhookable (`mov al,5Ch; ret` = 3 bytes, relocate TooShort(3)) | |
| 0x745510 | psGetMemoryFunctions | cdecl | platform::ps_get_memory_functions | bytes | — | implemented-needs-L2 | returns &RwMemoryFunctions 0x8D6228 |
| 0x745520 | Ps::psInstallFileSystem | cdecl | platform::ps_install_file_system | bytes | — | L1 (--cases 1000000, 0 mismatches, Unicorn) | |
| 0x745530 | Ps::psNativeTextureSupport | cdecl | platform::ps_native_texture_support | bytes | jmp RwD3D9DeviceSupportsDXTTexture | implemented-needs-L2 | |
| 0x745540 | Ps::psDebugMessageHandler | cdecl | platform::ps_debug_message_handler | bytes | OutputDebugStringA via the IAT slot in ESI (cell 0x858118) | implemented-needs-L2 | ret Void |
| 0x745560 | Win::Win32_InitInstance | cdecl | platform::win32_init_instance | bytes | — | implemented-needs-L2 | ret EAX (HWND) |
| 0x7455E0 | Ps::GetVideoMemInfo | cdecl | platform::get_video_mem_info | bytes | — | implemented-needs-L2 | |
| 0x745650 | FUN_00745650 (DirectX version check) | cdecl | platform::check_dx_version | bytes + decomp | gr has it only as an original comment in psInitialize | implemented-needs-L2 | returns 0/0x700/0x800/0x801/0x900; DirectDrawCreateEx through the GetProcAddress pointer (`orig!` `dyn`) |
| 0x745840 | Ps::CheckDirectSound | cdecl | platform::check_direct_sound | bytes | only caps.dwSize initialised | implemented-needs-L2 | |
| 0x7458A0 | Ps::psTerminate | cdecl | platform::ps_terminate | bytes | — | ported-unhookable: lone `ret` | |
| 0x7458B0 | Ps::psAlwaysOnTop | cdecl | platform::ps_always_on_top | bytes | — | implemented-needs-L2 | |
| 0x745A80 | VideoMode::FreeVideoModeList | cdecl | video::free_video_mode_list | bytes | signed count | implemented-needs-L2 | |
| 0x745AF0 | VideoMode::GetVideoModeList | cdecl | video::get_video_mode_list | bytes | entry jmp into protection chunk 0x402012 (`[0x156070C] - [0x6ECF06]` = gVideoModes); *new*: signed loop count (gr unsigned), skipped entries stored 0 explicitly; aspect via x87 fild/fidiv PC=24 | implemented-needs-L2 | |
| 0x745C70 | VideoMode::SetVideoMode | cdecl | video::set_video_mode | bytes | — | implemented-needs-L2 | ret Void (EAX holds the raster pointer, unused by callers) |
| 0x745CA0 | VideoMode::IsVideoModeExclusive | cdecl, AL | video::is_video_mode_exclusive | bytes | — | implemented-needs-L2 | |
| 0x745F90 | VideoPlayer::WaitState | cdecl | video::wait_state | bytes | — | implemented-needs-L2 | |
| 0x745FB0 | VideoPlayer::OnGraphNotify | cdecl | video::on_graph_notify | bytes | — | implemented-needs-L2 | |
| 0x746060 | Win::IsForegroundApp | cdecl, AL | platform::is_foreground_app | bytes | *new*: ForegroundApp read as a dword (gr: bool) | implemented-needs-L2 | |
| 0x7460A0 | GetBestRefreshRate | cdecl | video::get_best_refresh_rate | bytes | — | implemented-needs-L2 | 3 args, EAX |
| 0x746190 | Ps::psSelectDevice | cdecl | — | bytes | — | deferred: orchestrator (subsystem dialog DialogBoxParamA, video mode selection, refresh rate, multisampling) | |
| 0x746480 | FUN_00746480 (command-line splitter) | EAX arg | — | decomp | — | deferred: argument in EAX (not hookable), only caller is WinMain (deferred) | |
| 0x7465B0 | Ps::InitialiseLanguage | cdecl | platform::initialise_language | bytes | *new*: GetSystemDefaultLCID, GetUserDefaultLCID and GetKeyboardLayout are each called twice | implemented-needs-L2 | |
| 0x7466D0 | VideoPlayer::UpdateWindow | cdecl | video::update_window | bytes | — | implemented-needs-L2 | |
| 0x746740 | VideoPlayer::Release | cdecl | video::release | bytes | — | implemented-needs-L2 | |
| 0x7467B0 | VideoPlayer::Shutdown | cdecl | video::shutdown | bytes | — | implemented-needs-L2 | |
| 0x746870 | Win::MessageLoop | cdecl | `input::message_loop` | bytes | none (WM_QUIT sets `RsGlobal.quit` and keeps peeking) | implemented-needs-L2 (void) | PeekMessageA/TranslateMessage/DispatchMessageA through IAT cells |
| 0x7468E0 | Win::IsAlreadyRunning | cdecl | platform::is_already_running | bytes | — | implemented-needs-L2 | |
| 0x7469A0 | Input::diMouseInit | cdecl, EAX=HRESULT | `input::di_mouse_init` | bytes | returns 0 even when Acquire fails; flags 5/6 | implemented-needs-L2 (cdecl, 1, EAX) | COM slots 0x0C/0x2C/0x34/0x1C |
| 0x746D80 | Input::diPadSetRanges | cdecl, EAX | `input::di_pad_set_ranges` | bytes | *new*: bZAxisPresent/bZRotPresent set only after a successful SetProperty (gr: before), RZ uses DIJOFS_RZ 0x14 (gr passes DIJOFS_Z twice) | implemented-needs-L2 (cdecl, 2, EAX) | |
| 0x747000 | FUN_00747000 | cdecl | `input::release_di_interface` | bytes | — (unreversed: `ps->diInterface->Release(); ps->diInterface = 0`) | implemented-needs-L2 (void) | name for the catalogue: `WinInput::ReleaseInput`-like (called from __MainWndProc) |
| 0x747020 | Input::EnumDevicesCallback | stdcall, `ret 8` | `input::enum_devices_callback` | bytes | a third device gets a null out-pointer (CreateDevice fails, continue) | implemented-needs-L2 (stdcall, 2, EAX) | counter 0xC92468 |
| 0x747420 | Ps::psInitialize | cdecl | — | bytes | — | deferred: orchestrator (CFileMgr, InitUserDirectories, save dir, language, OS/DirectX/memory checks with MessageBoxes, CText) | |
| 0x747660 | VideoPlayer::Play | cdecl | video::play | bytes | gr FIX_BUGS blocks not followed (no cleanup on failure) | implemented-needs-L2 | 2 args, Void |
| 0x7477C0 | FUN_007477c0 (pad enumeration) | cdecl | platform::enum_pads | bytes | — | implemented-needs-L2 | |
| 0x747820 | Win::GTATranslateKey | cdecl, AL | `input::gta_translate_key` | bytes (two-level jump table 0x747C38/0x747B68 extracted) | *new*: MapVirtualKeyA char stored only if `< 0xFF` (gr `<= 0xFF`) | L1 (1,002,000, 0; OS version operand patched, MapVirtualKeyA stubbed via 0x747000) | |
| 0x747CD0 | Win::GTATranslateShiftKey | cdecl | `input::gta_translate_shift_key` | bytes | EAX not set by the original (gr returns BOOL) | implemented-needs-L2 (void) | GetKeyState through IAT cell |
| 0x747EB0 | Win::__MainWndProc | stdcall | — | — | — | deferred: orchestrator (1458-byte message dispatcher, 51 direct + 8 indirect calls into menu, pad, audio, video, RW, CTimer) | port with the window/platform layer |
| 0x7485C0 | Input::diPadInit | cdecl | `input::di_pad_init` | bytes | *new*: gr's diPadSetPIDVID also writes `wDeviceID = dwData`; the original writes only vendor (low word) / product (high word) | implemented-needs-L2 (void) | original UB: pad 1 reuses pad 0's DIPROPDWORD whose header is written only on the pad-0 path (port 0) |
| 0x7486A0 | Win::Win32_InitApplication | ESI arg | platform::win32_init_application | bytes | — | ported-unhookable: HINSTANCE in ESI | |
| 0x748710 | Win::NOTSA_WinMain | — | — | bytes | — | deferred: orchestrator (WinMain: IsAlreadyRunning, Win32_InitApplication, command line, RsEventHandler sequence) | |
| 0x7487CF | Input::Initialise | — | — | — | — | deferred: not a function entry — 0x7487CF is inside WinMain (0x748710, after `mov [eax+4], esi`), where gta-reversed's inlined `WinInput::Initialise` starts; the row's extent 0x7487CF-0x748EF7 is the rest of WinMain's body. Port with WinMain; the catalogue row should be merged into 0x748710 | |
| 0x855690 | __init_g_c87bd0 | cdecl | pools::init_g_c87bd0 | bytes | `CMemoryHeap::CMemoryHeap()` (0x72E860, uncatalogued) on 0xC87BD0 | implemented-needs-L2 | runs in CRT start-up before sa-hook attaches: in practice never entered through the hook |
| 0x856430 | CLoadMonitor::Destructor | thiscall | `loadmonitor::destructor` (no hook) | bytes | none | ported-unhookable | lone `ret` |

## Deviations found while porting, not yet in `catalog/gr-deviations-notes.csv`

All keep-original (the ports follow the bytes). Candidates for CAT-14 notes.

| address | gta-reversed file:line | description | evidence (1.0 US bytes) |
|---|---|---|---|
| 0x550180..0x550E40, 0x552AF0, 0x405900 (CPool ctors) | source/game_sa/Core/Pool.h `CPool(size_t, const char*)` | gr fills the storage with 0xFD (`DoFill(NOMANSLAND_FILL)`) and value-initialises the slot states; the original allocates `new[](size*slot)` + `new[](size)` (0x552AF0: plain `new` 0x82119A), sets owns=1, size, first_free=-1, and per slot does `map[i] |= 0x80; map[i] &= 0x80` (two RMWs). No fill; `+0x11` (`m_DealWithNoMemory`) is never written; the name argument is unused | e.g. 0x550180-0x5501DA (`or dl,80h` / `and dl,80h` loop, `ret 8`) |
| 0x5519F0 CPools::ShutDown | source/game_sa/Pools.cpp (ShutDown, `delete std::exchange(ms_p*, nullptr)`), Pool.h `Flush` | original: per pool, only when `size > 0`: free both arrays if owned, then objects=map=size=first_free=0 (gr Flush: first_free=-1, owns/locked cleared, always); header freed with `operator delete`; the `ms_p*Pool` globals are NOT nulled (gr nulls them); `printf` of "Shutdown pool started\n"/"Shutdown pool done\n" (0x863D78/0x863D64) | 0x551A07-0x551A40 pattern ×17, no store to 0xB74484.. |
| 0x536DD0, 0x41B870 (operator delete), 0x536E10 (CEntryInfoList::Flush) | Pool.h `Delete` | `first_free` is written only when the slot index is lower (`cmp; jge` skip); no DEADLAND fill | 0x536DF7-0x536DFC |
| 0x551950 CPools::CheckPoolsEmpty | Pools.cpp:121 (listed as `maybe` pool-iteration) | confirmed: the object pool is walked from the last slot down; prints with CRT `printf` (0x821982), format at 0x863D3C | 0x551970-0x5519DC |
| 0x571CC0 CReferences::PruneAllReferencesInWorld | References.cpp:55-57 (listed as `maybe`) | confirmed: each pool walked from the last slot down | 0x571CD8-0x571CF7 |
| 0x550080 CPools::MakeSureSlotInObjectPoolIsEmpty | Pools.cpp:276 (`NOTSA_UNREACHABLE`) | the original has a body; gr's commented-out version inverts the projectile test: the original returns when `RemoveIfThisIsAProjectile` is TRUE and does the copy/replace when false | 0x5500E8-0x5500F2 (`test al,al; jne exit`) |
| 0x5D2D70 CPools::LoadPedPool | Pools.cpp:204 | non-player ped types reuse the previous iteration's ped pointer (initially null) for `Load`/`CWorld::Add`; gr uses a fresh nullptr + assert | esi kept across iterations (0x5D2D93 `xor esi,esi` before the loop only) |
| 0x5D2A20 CPools::LoadVehiclePool | Pools.cpp LoadVehiclePool | `--NumRandomCars` goes through a protection chunk (0x403AC1: address = `[0x156079C] - [0x59921E]` = 0x969094); bike/BMX `+0x614 |= 0x10` is applied even on a null vehicle; vehicle types 7/8 (and out of range) build a CAutomobile | 0x5D2CFB → 0x403AC1 → 0x5D2D00; jump table 0x5D2D40 |
| 0x552A90 CQuadTreeNode::DeleteItem(item, rect) | Core/QuadTreeNode.h DeleteItem(T, const CRect&) | original calls `InSector` for every sector first and then `child->DeleteItem(item, rect)` without a null check (gr: null check first, then the rect-less `DeleteItem(item)`) | 0x552AC0-0x552AD5 |
| 0x552A40 / 0x552A90 | QuadTreeNode.h DeleteItem | `CPtrListSingleLink::DeleteItem` is called only after a scan finds the item | 0x552A53-0x552A61 |
| 0x5526A0 CQuadTreeNode::InSector | QuadTreeNode.h InSector/GetSectorRect | sectors 1 and 3 keep the new left bound (mid x) in ST0 without rounding to f32 (sectors 0/2 store mid x as f32); mids use `(x1 + x0)` register form vs `(x0 + x1)` memory form (NaN payload order only) | 0x5526F1-0x5526F9, 0x552729-0x552731 |
| 0x536D10 "CEntryInfoNodePool::GetAt" | catalogue name | mislabel: the bytes are `CPool<CEntryInfoNode>::New` (slot 0x14), the target of `CEntryInfoNode::operator new` (0x536DC0 `jmp 0x536D10`) | byte-identical to CPedPool::New up to the stride |
| 0x5E4760 CPed::operator delete (ARCH-1 pilot) | Pool.h Delete | the pilot port read `m_nFirstFree` before the map write and wrote it unconditionally; the original re-reads it after `or byte [map+idx],80h` (a negative index can hit the header) and writes only when lower. pool::pool_delete now delegates to pools::pool_delete_exact; new L1 specs CPedOperatorNew/CPedOperatorDelete pass 10^6 | 0x5E4784-0x5E478C; found by the 0x536DD0 spec (113/20000 mismatches before the fix) |
| 0x53D0B0 | source/game_sa/LoadMonitor.cpp (EndFrame, `2.f / 3.f`, `1.f / 3.f`) | processing-level thresholds are 0.66666 / 0.33333, not 2/3, 1/3 | `fcom [0x863AD8]` = 0x3F2AAA3B, `fcom [0x863AD4]` = 0x3EAAAA3B |
| 0x53D0B0 | LoadMonitor.cpp (EndFrame move speed) | `sqrt((x*50)^2+(y*50)^2+(z*50)^2)`, each component scaled first (gr `Magnitude()*50`) | 0x53D0FD–0x53D133 |
| 0x53CFA0 | LoadMonitor.cpp (ctor) | original also zeroes m_iCyclesHistory[4][8]; does not write m_bForceProcLevel/+0x18 | 0x53CFD8 loop `rep stosd` 8 dwords x4 |
| 0x538730 | source/game_sa/FileMgr.cpp:ChangeDir | no `memset(ms_dirName)`: relative paths are appended to the current dir; no length check | 0x538730–0x5387C1 (strcpy only for leading `\`, then strcat) |
| 0x5387D0 / 0x538860 | FileMgr.cpp:SetDir / SetDirMyDocuments | no length checks / EINVAL / ENOMEM paths | bytes |
| 0x538890 | FileMgr.cpp:LoadFile | returns -1 without fclose when fread result is negative (`jl`) | 0x5388CC |
| 0x538A50 | FileMgr.cpp:GetErrorReadWrite | returns `_flag & 0x20` in EAX (not a bool) | `mov eax,[eax+0Ch]; and eax,20h` |
| 0x5324A0 / 0x5324D0 | source/game_sa/Directory.cpp FindItem(name/hash, pos, size) | `*size` stored before `*pos` | 0x5324B6 / 0x53251A |
| 0x532290 | Directory.h (default member init) | empty ctor leaves m_bOwnsEntries (+0x0C) untouched | 3 stores only |
| 0x532350 | Directory.cpp:ReadDirFile | count read into the filename argument slot (stale pointer as count on short read) | `lea edx,[esp+4Ch]` = [esp+4] at entry |
| 0x56D070 | source/game_sa/IniFile.cpp | NaN multiplier is kept (gr: `>= 0.5` else 0.5) | compare/branch at 0x56D0BB–0x56D0F0 |
| 0x7186E0 | source/game_sa/common.cpp:73 MakeUpperCase(dst,src) | returns start of dst (gr returns the advanced pointer); ASCII-only | EAX = arg0, never changed |
| 0x718710 | common.cpp:81 MakeUpperCase(dst) | EAX not set (gr returns dest) | no write to eax except `mov al,[ecx]` |
| 0x71DA00 | common.cpp:118 CalcScreenCoors(VVff) | `sx = W*(1/z)` stored as f32, reloaded, `/FOV*70` (gr: `W*(1/z/FOV*70)`) | 0x71DA6D–0x71DAA3 |
| 0x71DB80 | common.cpp DoesInfiniteLineTouchScreen | strict `0 < x < W`, `0 < y < H`; edge tests 1–2 inlined with `-y*dx`, `-x*dy` and f32-stored intermediates | 0x71DB80–0x71DC3B |
| 0x71E050 | common.cpp IsPointInsideLine | NaN → true (`!(d < r)`) | `test ah,5; jp` → 1 |
| 0x72F180 | source/game_sa/Memory/MemoryHeap.cpp:32 Init | FreeListBegin.m_Prev and FreeListEnd.m_Next not written; m_paCommonSizes cleared only when bInitSizes | 0x72F1CF–0x72F207 |
| 0x532260 | source/game_sa/Debug.cpp DebugDisplayTextBuffer | empty in 1.0 US (gr draws strings) | `C3` |
| 0x532240 | Debug.cpp DebugInitTextBuffer | original clears a uint16 at 0xB71508 (gr reserves a vector) | `mov word [0xB71508], 0` |
| 0x745270 | source/app/platform/win/WinPs.cpp:246 (psTimer) | gr returns `OS_TimeMS()`; the original brackets `timeGetTime` with `timeGetDevCaps` + `timeBeginPeriod(wPeriodMin)` / `timeEndPeriod` (changes the system timer resolution around the call) | 0x74527B/0x745286/0x74528C/0x745299 `call [IAT]` WINMM imports |
| 0x745470 | WinPs.cpp:338 (psPathnameCreate) | gr uses CRT strlen/strcpy_s/strchr and `psPathGetSeparator()`; the original uses RwEngineInstance string functions (+0x120 strlen, +0xF8 strcpy, +0x10C strchr), passes `CMemoryMgr::Malloc` a second argument 0 (RwMalloc hint) and stores the literal '\\' | 0x74547E, 0x745489, 0x74549F, 0x7454AE, 0x7454C0 |
| 0x7465B0 | WinPs.cpp:73 (InitialiseLanguage) | the original calls GetSystemDefaultLCID, GetUserDefaultLCID and GetKeyboardLayout(0) twice each (second results only feed the prefs-language computation, which equals gr's); harmless | 0x7465BA/0x7465E2 `call edi`, 0x7465C4/0x7465E4, 0x7465D4/0x7465F2 |
| 0x746060 | source/app/app.h:24 / WinMain.cpp:140 | `ForegroundApp` is a 32-bit RwBool in the original (`mov eax, [8D621Ch]; test eax, eax`), gr declares it `bool` | 0x746060 bytes `A1 1C 62 8D 00 85 C0` |
| 0x745AF0 | source/app/platform/win/VideoMode.cpp:31 (GetVideoModeList) | loop count signed (`jle`, `jl`) in the original, unsigned in gr; the original stores 0 for skipped modes after the checks (gr stores nullptr first) | 0x745B3B, 0x745C38/0x745C46 |
| 0x53D840 / 0x619560 / 0x745AF0 / 0x744FB0 | — | entry protection chunks (`stc/clc; adc/sbb` of a .HOODLUM dword and a .text dword) compute the address of the global read (Scene.camera, RsGlobal.keyboard.used, gVideoModes) or the pushed constant (KEY_READ); not in gr | chunks 0x4037CB, 0x40591D, 0x402012, 0x406807 |
| 0x744D50 | source/game_sa/Pad.h:266 | IsMenuKeyJustPressed: original = lalt OR ralt just pressed; gr = lalt only | 0x744D50-0x744D7E compares NewKeyState+0x266/+0x268 against OldKeyState; L1 spec catches gr's version |
| 0x747820 | source/app/platform/win/WinPlatform.cpp (GTATranslateKey default) | MapVirtualKeyA char stored only if `< 0xFF`; gr `<= 0xFF` | 0x747B4F `cmp eax, 0xFF; jge` |
| 0x735840 | source/app/app_light.cpp (AddAnExtraDirectionalLight) | weakest-slot search starts at -1 (`or esi, -1`): a light weaker than all six is dropped; gr starts at 0 and replaces slot 0 | 0x73587E, 0x735943 `test esi, esi; jl` return |
| 0x735C90 | source/app/app_light.cpp (SetAmbientColoursToIndicateRoadGroup) | index = signed `idx % 7` (cdq; idiv 7); gr `idx & 8` | 0x735C94-0x735C9A |
| 0x735D90 | source/app/app_light.cpp (SetLightColoursForPedsCarsAndObjects) | per-channel brightness 0xB79E38 (R) / 0xB79E34 (G) / 0xB79E30 (B), R/G added to unrounded x87 products; gr adds m_BrightnessAddedToAmbientRed to all three | 0x735DEF-0x735E1B |
| 0x735AB0, 0x735B40 | source/app/app_light.cpp (SetFlashyColours[_Mild]) | DirectionalLightColour = ForFrame copies only r,g,b; gr copies alpha too | 0x735ABA-0x735AFF (three dword moves) |
| 0x6FAB30 | source/app/app_light.cpp (GetDayNightBalance) | `x * (1/60)` with the f32 constant at 0x859044 where gr writes `/ 60.0f`; GetMinutesToday = h*60+m + s*(1/60) inlined | 0x6FAB5B, 0x6FAB8F, 0x6FABCB `fmul [0x859044]` |
| 0x7448B0, 0x744930 | source/app/app_input.cpp (HandlePadButtonDown/Up) | pad number = padNumber != 0 ? 1 : 0 (gr keeps padID); the ControlsManager update is called for every button index, with button 0 for buttons not in the wanted state (gr skips them) | 0x7448C7/0x74490C, 0x7448E6-0x74491C; 0x744996-0x7449D3 |
| 0x746D80 | source/app/platform/win/WinInput.cpp:93-104 | bZAxisPresent/bZRotPresent written only after a successful SetProperty (gr: before, whenever GetObjectInfo succeeds); RZ uses DIJOFS_RZ 0x14 (gr's lambda passes DIJOFS_Z for both) | 0x746E71-0x746E76, 0x746E8D/0x746EAD |
| 0x7485C0 | source/app/platform/win/WinInput.cpp:115 (diPadSetPIDVID, NOTSA) | original stores only vendor (LOWORD) at +8 and product (HIWORD) at +0xC; gr also writes wDeviceID = dwData | 0x74863E-0x748655, 0x748679-0x748689 |
| 0x5BA520, 0x735730 | source/app/app_light.cpp | LightsCreate/LightsDestroy return the world pointer in EAX (gr void); harmless | 0x5BA681, 0x7357D9 |
| 0x747CD0 | WinPlatform.cpp:33 | GTATranslateShiftKey leaves EAX unset (gr returns TRUE/FALSE); harmless unless a caller tests it | whole body |
| 0x7487CF | source/app/platform/win/WinInput.cpp:26 | catalogue row "Input::Initialise" is not a function entry: it is the inlined WinInput::Initialise inside WinMain (0x748710) | 0x7487CC `mov [eax+4], esi` falls through into 0x7487CF |
| 0x5BA060 | source/game_sa/Game.cpp:120 (ValidateVersion) | the open check is `FILE* == -1` (`cmp ebx, -1`), not `!file`: a missing peds.col reaches `Seek(NULL)`; the version text is copied with `strncpy` (no forced terminator; gr `strncpy_s`); failure = `LoadingScreen("Invalid version", 0, 0)` + `jmp $` | 0x5BA07E `cmp ebx,-1; je`; 0x5BA0E3 `call _strncpy`; 0x5BA103/0x5BA110 |
| 0x53E160 | source/app/app_game.cpp:277 (RenderDebugShit) | empty `ret` in 1.0 US; gr calls CTheScripts::RenderTheScriptDebugLines and DefinedState | 0x53E160 `C3` |
| 0x53C500 | source/game_sa/Game.cpp:192 (TidyUpMemory) | the original calls TidyUpD3DTextures(100)/TidyUpD3DIndexBuffers(200) itself; gr calls DrasticTidyUpMemory (same effect, different call graph for hook toggles) | 0x53C516-0x53C527 |
| 0x53E600 | source/game_sa/Draw.cpp:39,43 (DoFade) | `JustLoadedDontFadeInYet` / `StillToFadeOut` are tested with `cmp byte, 1` (exactly 1), not as bools | 0x53E629 `cmp al, cl` (cl = 1), 0x53E63D `cmp byte [0xC16EDD], cl` |
| 0x5BF3B0 | source/app/app_game.cpp:51 (GameInit) | `RwInitialize(nullptr)` is inlined and its result ignored (gr wraps it in VERIFY) | 0x5BF3B2-0x5BF3BE |

## CORE-L2b: in-game L2/L3 of the 191 `implemented-needs-L2` rows (lane 1)

`tools/e2e/port-l2.sh` on the INF-4 grove walk (5,200 frames) and the pause-restart recording (3,700
frames), heap regions in `tools/e2e/l2-extras.txt` (CORE-L2b block). No Rust port had to change: every
mismatch found traces back to the harness (state outside game memory that the two runs share) or to
original uninitialised stack data. Runs (all from process start, `SA_HOOK_PORTS=orig`):

| run | what | result |
|---|---|---|
| grove `l2`, 138 hooks (L2-safe set), `L2_PER_FRAME=4` | per-function L2 | (b) l2 == plain on 5,200 frames; 0 mismatches except Ps::InitialiseLanguage (below) |
| grove `l2`, 14 burst hooks, `L2_PER_FRAME=1000`; psTimer + InitUserDirectories alone | every call compared | (b) identical; 0 mismatches, 0 undeclared |
| grove `rust`, all 191 on Rust | L3 | 5,200/5,200 frames IDENTICAL on all 17 subsystems, `randcalls` identical, no crash |
| pause-restart `plain` + `l2` (same 138), burst re-run (17 hooks, `L2_PER_FRAME=1000`), psTimer/InitUserDirectories | L2 | (b) identical on 3,700 frames; 0 mismatches except InitialiseLanguage |
| `l2all` (window all) on LightsCreate; file functions | heap arena restored and watched | see open items |

Not runnable in L2 (the case runs the function twice; effects outside process memory are not undone):
file position / descriptor / handle state (CFileMgr::Read, Write, Seek, ReadLine, Tell; OpenFile and
CloseFile: the second `fopen` gets the next descriptor, the second `fclose` returns -1 on the handle the
first one closed; ChangeDir appends a relative path to the OS current directory twice), one-time
initialisers (RwInitialize, GameInit, InitialiseOnceBeforeRW, InitialiseEssentialsAfterRW, InitialiseGame,
the two PluginAttach, RsInitialize, RsEventHandler, Win32_InitInstance, IsAlreadyRunning (the second
`CreateEventA` reports ERROR_ALREADY_EXISTS and the port then reads RsGlobal.ps = NULL, exactly as the
original would: 0x746924), diMouseInit, diPadInit, EnumDevicesCallback, AttachInputDevices,
RsInputDeviceAttach, FUN_007477c0, FUN_00747000), OS counters/queues (ShowCursor in
Rs/psMouseSetVisibility, PeekMessage in MessageLoop, DirectShow events in VideoPlayer::*), D3D scene and
device state (Rs/psCameraBeginUpdate, psCameraShowRaster, DoRWStuffEndOfFrame, SetVideoMode,
TidyUpMemory, DrasticTidyUpMemory, CameraDestroy, D3DDeviceRestoreCallback) and shutdown paths
(RwTerminate, RsRwTerminate, RsTerminate, FinalShutdown) plus the MessageBox wrappers. These have L3
evidence only (the all-191 Rust run); an L2 for them needs a harness mode that does not run the
original and the port on the same OS state.

Open items (evidence in `target/e2e/lane-1/port-l2-*` of the CORE-L2b worktree):
- Light::LightsCreate (1 call, frame 0): with `SA_L2_WINDOW=data` the RW free lists and the world's
  light list (heap) are not restored, so the Rust run gets the next RpLights and the extra lights stay in
  the world (the l2 trace then diverges at frame 203, `player`). With `l2all` the pointers match and the
  only differences are original uninitialised stack bytes: the colour `alpha` of each RpLight (+0x24;
  the original pushes an `RwRGBAReal` whose alpha is never written, 0x5BA54E-0x5BA55E; the port passes 0)
  and the three `pad` words of the RwMatrix that `RwFrameRotate` writes for pDirect's frame (orig 1 /
  0x7BCDAEA0 / 0x00460000, Rust 4 / 0x0036F294 / 0x03219A28: stack residue inside RW). Not portable
  bit for bit; a divergence-ledger decision.
- Ps::InitialiseLanguage (and the first CKeyGen::GetUppercaseKey case of a run): the original run makes
  the thread's first locale-aware CRT call; `__updatetlocinfo` (0x825456 → 0x82524D) sees the thread's
  ptd (TLS index 0x8E3110, OS state) without `ptlocinfo`, increments `__ptlocinfo`'s refcount at
  0x8E2EC0 (1→2) and 0x8E377C (0→1); the Rust run finds the ptd already updated. The only differing
  bytes; GetUppercaseKey's other 999 cases (and 1,000 on pause-restart) match.
