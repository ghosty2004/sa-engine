# CAT-6: names for the functions nothing else named (+ CAT-13 typed virtual calls)

Generator: `tools/catalog/names_extra.py` (in the `run_all.sh` fixpoint loop, after `data_refs.py`;
exit 3 when `catalog/names-extra.csv` changed, so the loop reruns once and `subsystems.py`,
`callgraph.py` and `progress.py` pick the names up). CAT-13 type sources: `tools/catalog/cat13_types.py`,
used by `callgraph.py build`.

## Where names come from now

`subsystems.py` writes the final name of every row into `procedures.csv` (new columns `name`,
`name_source`, `name_confidence`, `role`, `name_hint`). Precedence:

1. gta-reversed hook name (`gr_name`);
2. `catalog/rw-names.csv` (RW-RE agent), confidence high/medium. It wins over plugin-sdk, so
   plugin-sdk's wrong `RwTextureRead` at 0x4c7510 (really `CVehicleModelInfo::FindTextureCB`, which
   is also the gta-reversed name) cannot come back; the real `RwTextureRead` 0x7f3ac0 is named by
   rw-names;
3. CAT-5 name (`classification.csv`: plugin-sdk meta, validated gta-reversed vtable slot);
4. CAT-6 `names-extra.csv`, confidence high/medium;
5. a bare CAT-5 `Class::vfunc_N`.

Low-confidence names (237 rw-names rows, 354 CAT-6 hints) never become names. They appear as
`low-confidence name hint: …` in the `state.csv` evidence and in `procedures.csv` `name_hint`.
`progress.py` copies `name` into `state.csv` and the new `state.csv` column `role` (`init` / `dtor`).
A named `unknown` row becomes `specified` (source REA), as before.

`subsystems.py` also moves a `CALLED-ONLY` / `UNK-game` row to the subsystem of its new name:
rw-names rows of RenderWare modules go to `RW` (the 853 RW thunks that gta-reversed only calls), and
CAT-6 rows take the subsystem written in `names-extra.csv` (owner class; for an initialiser, the
global's owner, or the single subsystem of the gta-reversed functions that use the global).

`callgraph.py` (CAT-7) reads `procedures.csv` `name`, so `callgraph.csv`, `port-order.csv`,
`extents.csv` carry the new names too.

## Counts

Scope: non-excluded catalogue rows whose `state.csv` name is empty or a bare `vfunc_N`.
"RW" = subsystem `RW` or an address in `rw-names.csv`.

| | all | non-RW | RW |
|---|---:|---:|---:|
| unnamed before (state.csv of commit d939150) | 5,862 | 4,354 | 1,508 |
| unnamed after | **1,656** | **1,418** | **238** |
| named by this step | 4,206 | 2,936 | 1,270 |

State totals: `specified` 9,727 → 13,919, `unknown` 5,751 → 1,560. `CALLED-ONLY` subsystem 944 → 37
rows. One function was added to the catalogue (below): live functions 15,485 → 15,486.

### Non-RW names by source (2,936)

| source (`names-extra.csv` `source`) | high | medium | what |
|---|---:|---:|---|
| `init` | 1,666 | | every `__xc` static initialiser: `__init_<global>` |
| `dtor` | 138 | | every `atexit` callback that an initialiser registers: `__dtor_<global>` |
| `src-addr-comment` | 421 | 38 | gta-reversed `// 0xADDR` on a declaration/definition line or just above a definition |
| `src-wrapper` | 108 | 2 | one-statement wrapper calling the address (gta-reversed / plugin-sdk) |
| `src-define` | | 1 | plugin-sdk `#define FUNC_Class__Name 0xADDR` |
| `vtable-class` | 322 | 240 | slots of 141 of the 164 unknown vtables |
| total | 2,655 | 281 | |

How good the names are:
- **Initialisers.** 1,490 of the 1,666 store one `fstp` into a file-static float (`static const float
  k = a / b;` that MSVC could not fold). Those globals have no name in gta-reversed or plugin-sdk, so the
  name is `__init_g_<address>` (1,571 `g_`/address names among inits and dtors). 128 initialisers and
  105 destructor callbacks name a declared global (for example `__init_CAnimManager__ms_aAnimBlocks`,
  `__dtor_TheText`, `__dtor_CReplay__PlayerInfo`). Object found by: first store 1,522 (`fstp` 1,490,
  `mov` 27, `rep stos` 5), `this` of the first constructor call or tail jump 81, array passed to
  `eh_vector_constructor_iterator` 43, object of the registered destructor callback 7, none (calls only)
  13. 202 initialisers have a subsystem. The nearest named global below an unnamed one is listed in the
  evidence and never used as its owner: the extent of a global is not known.
- **Vtables.** The class is identified for 101 vtables: the constructor that installs it (it stores its
  own vtable last; 300 slot rows), the scalar deleting destructor in slot 0 calling `Class::~Class` and
  installing nothing itself (27), `RH_ScopedVirtualClass` in a gta-reversed header (3), or (medium)
  the owner of the named slots that only this vtable has (15). A lone destructor store proves nothing
  (MSVC drops the destructor's own vtable store when the body calls no virtual). The Fx vtables
  0x85a7dc and 0x85a968 proved this by giving 21 and 4 conflicting classes before the rule was
  tightened. Slot names come from the class's header layout when its length equals the table length,
  else from the base class (below). 40 vtables have no class but a proven base: their slots are named
  `vt_<vtable>::<slot name of the base>` (225 rows, medium). The base is the deepest validated
  gta-reversed class whose *own* slot code (code that differs from its parent's) the table shares at
  the same index. Sharing only inherited slots does not count, because sibling classes share those too.
  Only 8 rows are still `Class::vfunc_N`.
- **Conflicts.** A validated layout slot name beats an address comment. gta-reversed's
  `TaskComplexClimb.h` has the `Clone`/`GetTaskType` comments swapped: the bytes show 0x46a6b0 =
  `mov eax, 0x10b; ret` (GetTaskType), 0x46a6c0 = scalar deleting destructor, and Clone = 0x46a650.
  Both rows are medium with the other candidate in the evidence (9 such rows).
  `catalog/raw/names-extra-rejected.csv` lists 4 dropped candidates: 3 names that rw-names gives to
  another address (gta-reversed comments call 0x5a47e0 and 0x5d72a0 `RpClumpForAllAtomics`, rw-names has
  it at 0x749b70; 0x7322f0 `CVisibilityPlugins::SetClumpForAllAtomicsFlag` vs rw-names 0x732307), and one gta-reversed wrapper whose name is already the hook of another address
  (`CTaskSimpleFight::~CTaskSimpleFight` 0x61c530 calls 0x6239d0).
- **plugin-sdk alone is medium.** Measured on the 14,000 addresses that already have a
  gta-reversed/CAT-5 name, the same extractors agree with that name (after normalising
  ctor/dtor/overload suffixes) at these rates: gta-reversed address comments 7,917/8,040 (98.5 %),
  plugin-sdk wrappers 3,200/3,278 (97.6 %), plugin-sdk `Converted from` 2,426/2,488, plugin-sdk
  `#define FUNC_` 45/62. Most of the misses are naming variants, but there are real swaps:
  `CRestart::FindClosestHospital/PoliceRestartPoint`, `CPostEffects::FilterFX_Store…/Restore…`,
  `CMessages::Init/ClearAllMessagesDisplayedByGame`.

Unnamed non-RW rows after (1,418) by subsystem: UNK-game 1,036, AI 138, VEH 57, STREAM 30, WORLD 29,
CORE 27, SCRIPT 21, RENDER 19, PED 13, AUDIO 10, POP 9, CAM 7, WEAPON 6, CALLED-ONLY 4, ANIM 4, COLL 2,
HUD 2, PHYS 2, SAVE 2. Before (4,354): UNK-game 3,666, AI 208, CALLED-ONLY 91, VEH 84, RENDER 65,
WORLD 42, STREAM 41, CORE 39, SCRIPT 27, PED 20, AUDIO 18, HUD 13, POP 10, CAM 9, ANIM 5, WEAPON 5,
INPUT 4, SAVE 3, COLL 2, PHYS 2. Newly named rows by their subsystem after: UNK-game 1,706 (mostly
initialisers of unnamed globals), AI 697, AUDIO 83, RENDER 82, WORLD 66, VEH 60, STREAM 35,
CALLED-ONLY 33, HUD 26, SCRIPT 24, CAM 18, WEAPON 18, others ≤ 13.

Of the 1,418 still unnamed, 354 have a low hint (`sub_<addr>_from_<only caller>` 342,
`sub_<addr>_<the one string it references>` 12). These hints are not names.

### RW (1,270)

rw-names.csv high 1,074 + medium 196 named rows that were unnamed. The other rw-names rows are
gta-reversed hooks; the gta-reversed name stays there (10 differ, e.g. `RwCore::RwCameraClear` vs
`RwCameraClear`, `CClothesBuilder::nullptr` vs `CClothesBuilder::unnamed_0x5a42b0`). 238 RW rows stay
unnamed: their rw-names name is low (hint only), except 0x7cf9a0 and 0x80ffb0 (subsystem RW, code
immediates), which have no rw-names row (for the RW-RE agent).

- **0x750480** `_rpWorldObjCameraSync` was the rw-names address missing from the catalogue (0x7fff00 is
  already a row, through the code immediate at 0x7f73a5). It is added through
  `catalog/extra-code-addresses.csv` (rule `rw-names`). The `data_refs.py` entry check rejected it
  because the byte before it ends `jmp [ecx+0x14]`, not padding or `ret`.
- The `[catalogue: …]` tags in `rw-names.csv` evidence were refreshed once with
  `names_extra.py refresh-rw-tags` (1,318 of 1,829). Nothing else in that file changed.

## CAT-13: virtual calls on typed objects

`callgraph.py build` extends its `this` dataflow with the tokens OBJ C (pointer to an object of static
class C), VTC C (its vptr) and SLOTC C n. Sources (`cat13_types.py`):

- **members:** class layouts computed from the gta-reversed headers (vptr, primary base, members in
  order with MSVC alignment, bit-fields, arrays / `std::array` with constant or enum bounds, anonymous
  and named unions/structs, `notsa::WEnum*`/`EntityRef` as their original sizes). A layout is used only
  when the computed size equals `VALIDATE_SIZE` and every `VALIDATE_OFFSET` matches: **604** classes
  (including CTask/CTaskComplex/CTaskSimple, CEvent, CEntity, CPhysical, CPed, CPlayerPed, CVehicle,
  CBike, CBoat, CObject, CPedIntelligence, CTaskManager). CAutomobile and its subclasses are 8 bytes
  off and are not used. `mov r, [this/obj + off]` on a `C*` member gives OBJ C, and `lea` on an
  embedded polymorphic member gives OBJ C;
- **globals:** 38 typed globals (`StaticRef<C*>`, `C*& x = *(C**)…`, objects/arrays of C);
- **returns:** 1,528 functions whose gta-reversed declaration returns `C*` (one class over all
  overloads).

A `call [VTC C + 4n]` resolves to C's slot n (edge `virtual-typed`). Edges `virtual-typed-override`
go to the overrides in derived gta-reversed vtables, at most 16. Above that, or when C's known vtable
lacks the slot (CTask: `RH_ScopedVirtualClass(CTask, 0x86D48C, 6)`, abstract), the site is
`virtual-typed-wide`: the type is known but no edges are added. Both kinds are in the port order.

| `indirect-calls.csv` | before | after |
|---|---:|---:|
| sites with the virtual pattern `mov r,[obj]; call [r+n]` | 3,090 | 3,092 |
| … typed, edges added (`virtual-typed`) | 0 | **134** |
| … typed, too many/abstract targets (`virtual-typed-wide`) | 0 | **462** |
| … still untyped | 3,090 | 2,496 |
| all `unresolved` indirect sites | 5,566 | 4,971 |
| `this-vtable` sites (gta-reversed vtables in headers now included) | 318 | 321 |

So **596 / 3,092 = 19.3 %** of the non-`this` virtual sites now have a static receiver type, and 134 of
them (4.3 %) have concrete targets. Receivers by class: CTask 466 (mostly `m_pSubTask` → wide),
CEvent 45, CVehicle 29, CEntity 8, CBaseModelInfo 8, CTaskAllocator 7, others ≤ 5. New callgraph rows:
`virtual-typed` 82, `virtual-typed-override` 237. Port order: still 43 levels, 15 SCCs / 64
functions; leaves 6,685 → 6,671.

Not covered (next steps): stack arguments (`mov ecx, [esp+n]` with a typed parameter), pool slots,
CAutomobile-family layouts, and fan-out over 16.

Also fixed in `callgraph.py`: capstone sets no access flags on x87 memory operands, so
`fstp/fst/fistp/fnstsw/…` stores were recorded as reads in `rwsets.csv`. They are now `W`. This
matters for the static initialisers and every float global write.

## Verification

- `run_all.sh` twice: the second run gives sha256-identical `catalog/*.csv|md|json` and `catalog/raw/*.csv|json`
  (`rw-names.csv` excluded; `catalog/samp-usage.csv` appeared during the runs, written by another
  agent, not by the pipeline).
- Spot check, 15 random newly named rows (seed 6, all sources), checked against the bytes and
  gta-reversed: 15/15 consistent. Items: 8 initialisers (the `fstp`/store target at the named
  address; `__init_CAnimManager__ms_aAnimBlocks` loops 0xb4 × 0x20 bytes from 0xb5d4a0);
  `CEventPedToFlee::AffectsPed` (slot 5 of the CEvent layout; the body calls 0x5e0170 =
  `ped->IsAlive()` as in the header); `CQuadBike::~CQuadBike` 0x6cdc30 (installs 0x871ae8, the
  CQuadBike vtable); `FxInfoTrail_c::GetValue` and `FxInfoEmDir_c::GetValue` (slot 2 of FxInfo_c);
  `CPointRoute::operator delete` 0x41b7d0 (gta-reversed comment); `CPedScriptedAttractor::~…`
  (slot 1, after `GetType`, per CPedAttractor); `D3DResourceSystem::Shutdown` 0x730a00 (gta-reversed
  wrapper `plugin::Call<0x730A00>`).
- A second sample of 15 without initialisers (seed 21) was also consistent:
  `CEventGunAimedAt::GetLifeTime` = `xor eax, eax; ret` (header returns 0); `CBike::IsDoorFullyOpenU32`
  = `xor al, al; ret 4` (returns false); `CVehicleModelInfo::GetWheelSize` selects
  `[ecx+0x40]` for the front wheel; `CPlaneBanners::Init` / `CPad::~CPad` are a bare `ret` (NOP in
  gta-reversed); `CTaskComplexEvasiveStep::ControlSubTask`, `CEventDraggedOutCar` destructor,
  `vt_*` rows named from the proven base. Caveat: one-instruction bodies (`ret`, `xor eax,eax; ret`)
  can be shared by several functions through identical-code folding. Their name is the one the source
  attaches to that address.

## Commands

```sh
tools/catalog/run_all.sh                                   # whole pipeline (names_extra in the loop)
.venv/bin/python -I tools/catalog/names_extra.py catalog   # CAT-6 alone (exit 3 = names changed)
.venv/bin/python -I tools/catalog/names_extra.py refresh-rw-tags catalog   # one-off rw-names tag refresh
.venv/bin/python -I tools/catalog/cat13_types.py           # layout validation report
```
