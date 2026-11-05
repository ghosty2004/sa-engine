# CAT-14: gta-reversed fidelity audit

gta-reversed (`re/gta-reversed`, pinned `20d8d2a`) is our main code source, but it is not a 1:1 copy of
`gta_sa.exe` 1.0 US. It fixes bugs, replaces loops with helpers, adds checks and asserts, and leaves some
paths unimplemented. This audit lists every place in its source that shows such a difference, maps each one
to a catalogue function, and classifies what a port has to do there.

- Output: `catalog/gr-deviations.csv` (one row per marker), `catalog/gr-deviations-summary.json` (counts).
- Per function: `catalog/state.csv` column `gr_deviations` = rows with `deviates` yes or maybe. Check those
  rows before porting the function.
- Generator: `tools/catalog/gr_deviations.py`, run by `run_all.sh` after `script_commands.py` and before
  `progress.py`.
- Hand-checked input: `catalog/gr-deviations-notes.csv` (key file + line + a substring of the line). It
  overrides the rule-based fields and can add rows that no marker shows.
- Ghidra helper: `tools/catalog/ghidra/DecompileList.java` (read-only decompile of an address list, follows
  `.HOODLUM` stubs).

## Method

1. **Scope.** `source/game_sa` and `source/app` (`.cpp/.h/.hpp`). `source/toolsmenu` (debug UI),
   `source/extensions` (helpers, attributed at their use sites), `reversiblehooks` and `libs` have no
   original counterpart and are not scanned.
2. **Lexing and scopes.** Comments and strings are blanked, keeping lines. Only the first branch of each
   `#if` counts for brace matching (`#if 0` takes the `#else`). A small scope parser finds namespaces,
   classes and function definitions with their line extents, including constructor initialiser braces and
   `template<>` headers. Check: 97.7 % of the 7,904 reversed hook registrations have a parsed definition
   with the same name in their file or its header. The rest are mostly defined in another file (the
   `MenuManager_*.cpp` split) and are still matched by owner and name.
3. **Mapping** (`mapping` column):
   - `hook`: the definition matched to an `RH_Scoped*Install` registration (by owner and name;
     constructors and destructors map to `Constructor`/`Destructor`), or to an address table such as the
     cheat table.
   - `hook-overload`: one of several overloads, chosen by the parameter types in the signature argument of
     `RH_Scoped*OverloadedInstall`.
   - `addr-comment`: no hook, but a `// 0xADDR` comment on or just above the definition (or in the marker's
     own comment). Mapped to the catalogue function containing that address.
   - `script-handler`: a `REGISTER_COMMAND_HANDLER` function or lambda. Mapped to its opcode's
     `ProcessCommandsXToY`.
   - `script-opcode`: from `catalog/script-commands.csv`.
   - `hand`: set in the notes file.
   - `gr-only`: a gta-reversed function with no original counterpart.
   - `none`: the line is outside any function, for example in a declaration.
   - Suffix `not-in-catalogue`: the address is real, but the catalogue has no row for it (see findings).
4. **Markers** (`kind`, one row per line; `pool-iteration` and `rng-helper` are added next to any other
   marker on the same line):

| kind | what it detects | rule-based classification |
|---|---|---|
| `rbf` | `notsa::bugfixes::X` runtime switch | keep-original / yes |
| `fix-bugs-ifdef` | `#if/#ifdef/#ifndef FIX_BUGS` block (FIX_BUGS is defined in `source/CMakeLists.txt`) | keep-original / yes (`#if 0` → harmless) |
| `fix-bugs-runtime` | `notsa::IsFixBugs()` (constexpr true) | keep-original / yes |
| `fix-bugs-comment` | `FIX_BUGS` in a comment (fix applied with no macro) | keep-original / yes; "todo/maybe/?/should" → keep-original / no |
| `bugfix-comment` | BUGFIX, "bug fix", "fixed a bug/crash", "added check", null check, sanity check, ... | same as above |
| `notsa-unreachable` | `NOTSA_UNREACHABLE(_CASE)`, `NOTSA_UNUSED_FUNCTION` (debug build: exit(3); release: `__assume(false)`) | unclear / maybe; keep-original / yes when the comment says what the original does |
| `original-comment` | `Original:`, `originally`, `R*`, `vanilla`, `SA:`, Rockstar, PS2/mobile | phrase rules (below) |
| `bug-note` | a comment noting a (possible) original bug | keep-original / no (gr keeps the bug) unless a change is stated |
| `notsa` | `NOTSA` inside a function | phrase rules |
| `notsa-function`, `notsa-decl` | `NOTSA` on a gr-only definition or declaration | harmless / no, unless it names an original address |
| `todo` | TODO/FIXME about original/inline/vanilla/different/wrong | phrase rules |
| `assert` | `assert(` | harmless / no (debug only) |
| `notsa-helper` | `notsa::` helper on the line | harmless for pure utilities and tooling (`contains`, casts, `mdarray`, `ui::`, `script::`, logging); unclear for `EntityRef`, `step_to/up/down`, `format_to_sz`, `File`, `string_copy`, `Route`, `SpatialQuery`, `shapes::AngledRect`, `try_ston`, `wrap`, `remove_first` |
| `inlined` | "inlined" note | harmless / no |
| `define-branch` | `#if` on other gr build defines (`NOTSA_USE_SDL3` 46, `NOTSA_DEBUG` 8, `SA_SKINNED_PEDS` 5, `APPLY_CD_STREAM_DEADLOCK_FIX` 5, `USE_D3D9` 4, `NOTSA_WINDOWED_MODE` 3) | unclear / maybe |
| `pool-iteration` | `GetAllValid()` / `GetAllValidWithIndex()` (NOTSA, ascending slot order) | unclear / maybe |
| `rng-helper` | `CGeneral::GetRandomNumberInRange` call (formula differs from the original, see finding 1) | keep-original / yes |
| `script-default` | gr has a handler, the exe sends the opcode to the switch default | keep-original / yes |
| `script-unimpl-case` | gr registers the opcode as unimplemented (logs, returns `OR_INTERRUPT`), the exe has a case | keep-original / yes; evidence = the case's first instructions |

   Phrase rules for comments, applied in this order:
   1. "too / as well / kept / as-is / like original" → keep-original / no.
   2. "same result / equivalent / cleaner / readability / log / debug / naming / helper / unused /
      PS2 / mobile" → harmless / no.
   3. "instead / we use / changed / removed / added / fix / prevent / missing / uninit / OOB / crash /
      originally … was|used|did|returned|…" → keep-original / yes.
   4. Anything else → unclear / maybe.

   Classes: **keep-original** means the port must reproduce the original (and gr differs when `deviates`
   is yes). **harmless** means no observable difference. **unclear** means the item needs a measurement
   (PLAN §4b; nothing is guessed). The rules have limited precision. Only rows with an `evidence` value
   were checked against the original bytes. Every other row is a pointer to where to look, not a verdict.

## Counts (pin 20d8d2a)

3,997 rows. 2,764 of them map to 1,569 addresses; 106 of those rows point at addresses the catalogue does
not have. 978 catalogue functions have `gr_deviations` > 0 (1,546 rows with deviates yes or maybe).

| class / deviates | rows |
|---|---|
| keep-original / yes | 840 |
| keep-original / no (gr keeps the original; note only) | 184 |
| unclear / maybe | 1,238 |
| harmless / no | 1,735 |

| kind | rows | | kind | rows |
|---|---|---|---|---|
| assert | 732 | | notsa-function | 260 |
| notsa-helper | 562 | | notsa | 261 |
| original-comment | 514 | | bug-note | 145 |
| notsa-unreachable | 417 | | inlined | 115 |
| rng-helper | 375 | | pool-iteration | 98 |
| bugfix-comment | 82 | | define-branch | 71 |
| script-unimpl-case | 70 | | fix-bugs-ifdef | 60 |
| rbf | 53 | | fix-bugs-comment | 53 |
| fix-bugs-runtime | 49 | | notsa-decl | 41 |
| todo | 24 | | script-default | 15 |

Mapping: hook 2,500, hook-overload 41, addr-comment 111, script-handler 25, script-opcode 85, hand 2,
gr-only 834, none 399.

Rows with deviates yes/maybe per subsystem: AI 386, WORLD 160, RENDER 145, VEH 124, AUDIO 121, POP 98,
(not in catalogue) 92, STREAM 80, HUD 64, COLL 56, WEAPON 53, CORE 42, PED 42, SCRIPT 37, CAM 30, ANIM 24,
RW 24, INPUT 21, others < 20. Keep-original/yes rows: AI 172, RENDER 86, (not in catalogue) 82, VEH 77,
WORLD 75, POP 44, AUDIO 37, STREAM 34, WEAPON 28.

## Findings

1. **`CGeneral::GetRandomNumberInRange` differs from the original, and nothing in gr marks it.**
   - Integer version: gr computes `(T)lerp(min, max-1, rand()/32767.0f)`. The original, at 0x407180,
     computes `min + ftol((rand() & 0xFFFF) * (1/32768) * (max-min))`. The same `rand()` value gives a
     different result. For example, range 0..10 with rand() = 3277 gives 1 in the original and 0 in gr.
     In gr the top value only comes out when rand() = 32767.
   - Float version: gr's `lerp` is `max*t + min*(1-t)`. The original, at 0x41BD90, uses
     `(max-min)*t + min`. These are equal in exact arithmetic, but the rounding differs.
   - All 375 call sites are tagged `rng-helper`. Any port that takes gr's helper breaks RNG parity at every
     caller. Highest-priority item for MATH and for every subsystem that uses randomness.
2. **Pool iteration order.** gr's NOTSA `CPool::GetAllValid()` walks slots upwards. Original loops often
   walk downwards, for example `CCarCtrl::RemoveDistantCars` at 0x42CD10. The order changes removal order,
   RNG consumption and which entity wins a tie. 98 sites are tagged `pool-iteration` (unclear: check the
   direction of each original loop).
3. **`NOTSA_UNREACHABLE`** (417 rows, 350 of them mapped): on those paths gr exits or has undefined
   behaviour, while the original does something specific, usually returning 0 or falling out of the
   switch. Spot check 0x64E550: the default case returns 0. Every such path has to be read from the
   original.
4. **Reversible bug fixes are on by default** (`ReversibleBugFix::Enabled{ true }`), and `FIX_BUGS` is a
   compile definition. gr's default build therefore runs every fix listed below.
5. **Script commands.** 15 opcodes have gr handlers, but the PC exe sends them to the switch default (no
   behaviour, such as `COMMAND_RETURN_TRUE/FALSE`, `PRINT_WITH_*_NUMBERS*`, `IS_FIRST/SECOND_CAR_COLOUR`,
   `SET_CHAR_ARMOUR`). 70 opcodes are unimplemented in gr, but the exe has a case. Many of these cases are
   shared stubs (for example 0x480EF1 `xor al, al; ...`, shared by 4 opcodes), so reading them is cheap.
6. **Catalogue gaps found on the way** (CAT-12 follow-up; data-referenced code is not closed):
   - 24 of the 27 `ProcessCommandsXToY` range functions (table 0x8A6168) are missing from the catalogue.
   - 58 of the 65 cheat functions (table in `Cheat.cpp`, for example 0x439880) are missing.
   - Rows that point at them carry `not-in-catalogue`.
7. **Catalogue naming bug fixed.** `RH_ScopedClass` is block-scoped. `build_catalog.py` treated it as
   file-scoped, so the 75 `CVehicleModelInfo` hooks after the nested `CLinkedUpgradeList` block in
   `VehicleModelInfo.cpp` were named `CLinkedUpgradeList::…`. Both scanners now restore the outer class at
   the end of the block. Only those 75 `functions.csv` rows changed; no state changed.
8. `notsa::bugfixes::CTagManager_MissingTagCountResetOnShutdown` is used in `TagManager.cpp:46`, but it is
   not defined in `Bugs.hpp` at this pin.

## The reversible bug fix mechanism

`source/reversiblebugfixes/ReversibleBugFix.hpp` defines `struct ReversibleBugFix { Name, Description,
Credit, Enabled = true, Locked = false, RequiresRestart = false, Prev }`. Each fix is an `inline const`
global in `Bugs.hpp` (namespace `notsa::bugfixes`). Its constructor links it into a list headed by
`ReversibleBugFix::Tail`, which the debug tool (`toolsmenu/DebugModules/BugsDebugModule.cpp`) walks to show
toggles. Code tests a fix through `operator bool` (`if (notsa::bugfixes::X)`), so with the default
`Enabled = true` the fixed path runs. **A port must take the `!X` branch** (original behaviour). If it
offers the fix at all, it must be an option that is off by default (PLAN §4 step 3). Separate from this:
`FIX_BUGS` (compile-time, defined) and `notsa::IsFixBugs()` (constexpr true).

26 fixes are defined, plus 1 used but undefined; 53 use sites:

| fix | defined | use sites (catalogue address, function, file:line) |
|---|---|---|
| `GenericCrashing` | yes | 0x722390 C3dMarker::DeleteMarkerObject (3dMarker.cpp:82); 0x49cf90 CTagManager::GetAlpha (TagManager.cpp:128); 0x49cec0 CTagManager::SetAlpha (TagManager.cpp:150); gr-only CTagManager::ResetAlpha (TagManager.cpp:177); 0x72b850 CWeather::Update (Weather.cpp:549) |
| `GenericUB` | yes | 0x6d0a10 CAEVehicleAudioEntity::Destructor (:70); 0x6b0690 CAutomobile::DoHeliDustEffect (:5959, :5961); 0x5f3b70 / 0x5f36f0 CPedGeometryAnalyser::ComputeClosestSurfacePoint (:229, :236); 0x5fc4a0 CPedGroupIntelligence::Process (:192); 0x5f7410 CPedGroupIntelligence::SetPrimaryTaskAllocator (:228) |
| `GenericOOB` | yes | 0x699fa0 CFormation::ReturnDestinationForPed (Formation.cpp:21) |
| `GenericFrameRate` | yes | 0x4f5eb0 UpdateGasPedalAudio; 0x4fca10 ProcessDummyStateTransition (×4); 0x4fb070 ProcessNitro; gr-only UpdateRotorFreq (called from 5 aircraft functions); 0x5003a0 ProcessBoatEngine (×2) (all CAEVehicleAudioEntity); 0x5773d0 CMenuManager::AdditionalOptionInput (×2) |
| `PS2CoronaRendering` | yes | 0x6fb630 CCoronas::RenderReflections (Coronas.cpp:422) |
| `AnimBlendSequence_SetName_SetBoneTagFlag` | yes | 0x4d0c50 CAnimBlendSequence::SetName **(checked)** |
| `AESound_UpdatePlayTime_DivisionByZero` | yes | 0x4ef2e0 CAESound::UpdatePlayTime **(checked)** |
| `CCarCtrl_RemoveDistantCars_UseAfterFree` | yes | 0x42cd10 CCarCtrl::RemoveDistantCars (:802, :812) **(checked)** |
| `CAEVehicleAudioEntity_PlayBicycleSound_VolumeFix` | yes | 0x4f9710 PlayBicycleSound |
| `CAEVehicleAudioEntity_PlayAircraftSound_VolumeFix` | yes | 0x4f93c0 PlayAircraftSound |
| `CPathFind_SwitchRoadsOffInArea_StrayAreas` | yes | 0x452c80 CPathFind::SwitchRoadsOffInArea (:1298, :1318) |
| `CCustomCarPlateMgr_GeneratePlateText_MissingLettersAndDigits` | yes | 0x6fd5b0 GeneratePlateText (:404, :410) **(checked)** |
| `CTaskComplexLeaveCarAndFlee_MissingNullCheckForVehicleOnFlee` | yes | 0x63c0a0 CTaskComplexLeaveCarAndFlee::CreateSubTask |
| `CAECollisionAudioEntity_PlayLoopingCollisionSound_InvalidSurfaceType` | yes | 0x4db450 PlayLoopingCollisionSound |
| `CPool_DestructOnClear` | yes | gr-only CPool::Flush / Clear (Pool.h:115, :136); inlined into pool users in the original |
| `CTaskSimpleClimb_ProcessPed_SettleSpeedFrameRate` | yes | 0x680dc0 CTaskSimpleClimb::ProcessPed |
| `CDamageManager_GetLightStatus_IncorrectStatusCheckForLightRR` | yes | 0x6e1a60 CVehicle::DoVehicleLights **(checked)** |
| `CTaskSimpleSlideToCoord_Constructor_UninitializedTimer` | yes | 0x66c3e0 CTaskSimpleSlideToCoord ctor (no anim) **(checked)** |
| `CPedGeometryAnalyser_ComputeRouteRoundSphere_IncorrectDetourPosition` | yes | 0x5f1890 ComputeRouteRoundSphere (:1024) |
| `CPedGeometryAnalyser_ComputeRouteRoundSphere_AmbigousDetourTarget` | yes | 0x5f1890 ComputeRouteRoundSphere (:1041) |
| `CPedGeometryAnalyser_IsEntityBlockingTarget_IncorrectRadiusCheck` | yes | 0x5f3970 IsEntityBlockingTarget **(checked)** |
| `CObjectData_SetObjectData_UninitializedPhysics` | yes | 0x5a2d00 CObjectData::SetObjectData |
| `CObject_Render_DetachedPartRendering` | yes | 0x59f180 CObject::Render |
| `CVehicleModelInfo_SetClump_DirtMaterialsNeverFound` | yes | gr-only FindDirtMaterials, called from 0x4c95c0 CVehicleModelInfo::SetClump (VehicleModelInfo.cpp:242) |
| `CVehicle_ProcessWheel_and_ProcessBikeWheel_AlreadySkiddingNotReset` | yes | 0x6d6c00 CVehicle::ProcessWheel; 0x6d73b0 CVehicle::ProcessBikeWheel |
| `CVehicle_ScanAndMarkTargetForHeatSeekingMissile_NotProperlyStashedLOSTimestamps` | yes | 0x6e0400 (:5129, :5146) |
| `CTagManager_MissingTagCountResetOnShutdown` | **no** | 0x49cc60 CTagManager::ShutdownForRestart (TagManager.cpp:46) |

## Spot checks against the original (11, all consistent with gr's description of the original)

Method: headless Ghidra decompile (`DecompileList.java`, read-only) plus capstone disassembly where Ghidra
lost displaced code. The findings below are in our own words; addresses refer to the 1.0 US exe.

1. 0x6FD5B0 GeneratePlateText: letters `'A' - ftol(rand*(1/32768)*-23)` give A..W; digits use -9 and give
   0..8. The fix changes the original. **Confirmed.**
2. 0x4EF2E0 CAESound::UpdatePlayTime: `playTime % length` with no zero test. **Confirmed.**
3. 0x42CD10 RemoveDistantCars: one descending pass; it calls PossiblyRemoveVehicle and then reads
   `+0x42A & 0x10` of the same slot (the use-after-free gr fixes). **Confirmed**, plus a new difference:
   gr's fix-off path iterates in ascending order.
4. 0x64E550 ComputeDoorFlag: the default case returns 0 (gr: `NOTSA_UNREACHABLE`). **Confirmed.**
5. 0x7189B0 CFont::Shutdown: `RemoveTxdSlot(FindTxdSlot("fonts"/"ps2btns"))` with no existence check
   (gr: `SafeRemoveTxdSlot`). **Confirmed.**
6. 0x5F3970 IsEntityBlockingTarget: `bound² + radius²` compared with the 2D distance from `fsqrt`. gr's
   fix-off branch matches. **Confirmed.**
7. 0x66C3E0 CTaskSimpleSlideToCoord ctor: writes up to the flag byte +0x78; +0x7C (`m_EndTime`) is never
   written. **Confirmed.**
8. 0x7295E0 CWaterCannon::PushPeds: box initialised to ±10000.0 (gr: ±FLT_MAX). **Confirmed.**
9. 0x6E1A60 DoVehicleLights: four GetLightStatus (0x6C2130) calls with arguments 1, ebp, 3, 3; none passes
   2 (REAR_RIGHT). **Confirmed.**
10. 0x4D0C50 CAnimBlendSequence::SetName: stores only `CKeyGen::GetUppercaseKey(name)`. **Confirmed.**
11. 0x407180 / 0x41BD90 GetRandomNumberInRange (finding 1): the formulas above, read from the
    instructions. **Confirmed the deviation.**

## Limits

- The classification is rule-based except for the 16 hand-checked rows (`evidence` set). Rules can
  misfire in both directions. Read the gr comment and the original before relying on a row.
- Deviations with no marker are not found by the scan. Findings 1 and 2 were found by hand and turned into
  rules. Others certainly exist: helpers such as `CGeneral::RandomBool`, `DoCoinFlip` and `lerp` users,
  plus code reordering without a comment.
- Line-level markers only. A single `NOTSA` comment can cover a whole block.
