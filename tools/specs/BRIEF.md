# RE-SPEC brief (shared by all spec writers; RE-SPEC-1 and RE-SPEC-2)

Pipeline (reproducible): `select.py WORK SUBSYS...` -> `decomp.sh WORK/addrs.txt WORK/decomp.c` ->
`bundle.py WORK` -> `assign.py WORK` -> writers (this brief) -> `merge_index.py WORK` (appends the
index fragments to catalog/specs-index.csv and checks coverage). WORK is a scratch directory, `$WORK` below.

Project: /home/ghosty2004/sa-engine, a 1:1 Rust rebuild of gta_sa.exe 1.0 US. Ports must reproduce the
ORIGINAL binary exactly (including its bugs). Implementation agents will port from your specs WITHOUT
access to REA/Ghidra, so a spec must be complete and exact.

Read first (briefly): /home/ghosty2004/sa-engine/CLAUDE.md, docs/PLAN.md §4b (no guessing),
docs/arch/ADR-001-globals-and-layouts.md (globals via `globals!`, calls via `orig!`, x87 via `sa_math::x87`).

## Inputs (read-only)
- Evidence bundles: $WORK/bundles/<addr>.txt
  Each has: catalogue header, callees, callers, rwsets (globals read/written), gr-deviations rows,
  full capstone disassembly (with `.HOODLUM` body when the address is a jmp stub; memory operands annotated
  with the constant's value f32/f64 and global names) and the Ghidra decompile.
  THE DISASSEMBLY IS THE AUTHORITY. The decompile is a reading aid only (it hides x87 order, ST0 returns,
  register args, precision).
- gta-reversed source (pinned): /home/ghosty2004/sa-engine/re/gta-reversed/source/game_sa — use it for
  class layouts (field names/offsets from headers, VALIDATE_OFFSET), global names (`StaticRef<T>(0xADDR)`),
  enum values, and to cross-check. Some "REA-source" functions still have (unhooked or partial) gr code:
  you may cite it, but every behavioural statement must be checked against the bytes; where gr differs from
  the bytes, say so explicitly.
- catalog/layouts.csv (sizes/offsets), catalog/state.csv, catalog/gr-deviations.csv,
  catalog/gr-deviations-notes.csv, docs/re/cat14-gr-deviations.md (finding 1: gr's
  CGeneral::GetRandomNumberInRange differs from the original 0x407180/0x41BD90).
- If you need the raw bytes of a data table / a constant not annotated, read them with:
  `cd /home/ghosty2004/sa-engine && .venv/bin/python -I -c "import pefile,struct;pe=pefile.PE('re/gta/1.0-US/gta_sa.exe');m=pe.get_memory_mapped_image();a=0xADDR;print(m[a-0x400000:a-0x400000+N].hex())"`
  (memory-mapped image; .bss is zero). For a function NOT in the bundles (e.g. a callee you need to
  understand), use `.venv/bin/python -I tools/specs/dis.py ADDR` (catalogue extent; also `LO-HI` raw ranges,
  `dADDR:N` dwords, `bADDR:N` bytes), extents in catalog/extents.csv.
- Do NOT use the REA MCP tools (one agent owns REA). Do not run Ghidra.
- Scope of "full spec": source REA / state unknown / gta-reversed hook with `.Reversed = false` / named
  "called from gta-reversed only" (plugin::Call*). For the gr-unreversed ones gr usually has (unhooked) C++:
  treat it as an unverified reading aid only; state for each where it differs from the bytes.
- Budget: very large functions (> 1500 insns) still need a complete spec, but structure it by block
  (address ranges) and spend the precision on float order, constants and branch conditions; never skip a
  function silently. If you run out of budget, list the unwritten addresses in your final report.

## Output
Write ONLY the files assigned to you (listed in your task) under
/home/ghosty2004/sa-engine/docs/re/specs/<SUBSYSTEM>/<Class>.md (full specs) or
docs/re/specs/<SUBSYSTEM>/<Class>.deviations.md (deviation notes), plus your index fragment
$WORK/index-<AGENT>.csv
with header `address,name,subsystem,spec_file,kind,confidence` (spec_file relative to repo root, kind =
full-spec | deviation-note, confidence = high | medium | low; low = open questions remain that affect behaviour).
English only, no emojis. Do not touch anything else (no commits, no catalog edits, never ~/samp-rs).
Class naming: the class from the catalogue name (`CGarage::Foo` → CGarage.md). Unnamed functions: put them
in the class their evidence points to (callers, `this` layout, name_hint) and give a best evidence-backed
name like `CGarage::sub_448330 (proposed: ...)` with the evidence; if unclear use `_unattributed.md`.
Static initialisers / atexit destructors (`__init_*`, `__dtor_*`) may go together into `_static-init.md`
of their subsystem with one short spec each (what object is constructed/destroyed, which ctor/dtor is
called, with what count/stride, atexit registration).

### Full-spec format (one `##` section per function, ordered by address)
```
## 0xADDR ClassName::Method  (catalogue: <state>/<source>, port level N)
- Convention / signature: `__thiscall` (this = ECX: CFoo*) / `__cdecl` / `__stdcall`, args with stack
  offsets and inferred types, return (EAX / ST0 / AL / void), `ret N`. Note register-only args, non-standard
  conventions, callers that rely on preserved registers.
- Callers: (count, notable) — Callees: list with addresses and names (via `orig!`).
- Globals: read / written (address, name, type).
- Fields of `this`/args used: offset, type, meaning (gr name if known).
- Behaviour (step by step, in the original's order): branch structure, loop direction and bounds,
  every float op in the exact order the x87 code does it (e.g. `t = (a.x - b.x); t *= k[0x858b50]=0.5f`),
  constants with their address and value (f32/f64; note when a double constant is used with an f32
  operand), int/float conversions (ftol = `_ftol` truncation vs `fistp` with current rounding), compares
  (which operand order, how NaN falls through), short-circuit order.
- Edge cases / oddities: uninitialised reads, overflow, signedness, dead code, original bugs, things gr
  "fixes".
- gta-reversed: provided? (unhooked code / plugin::Call / absent) and any difference vs bytes.
- Open questions: each with the address of the instruction that is unclear. (No guessing: if you cannot
  determine something from the bytes, write it here.)
```
Keep trivial functions short (a 3-instruction getter needs 4 lines). Spend the words on the non-trivial
ones. Be exact about x87: values loaded from f32 memory, computed in x87 registers (precision control is
24-bit in-game, so intermediates round to f32 significand but keep extended exponent), stored as f32.

### Deviation-note format (one `##` per function)
```
## 0xADDR Class::Method  (gr_deviations = N)
For each gr-deviations.csv row of this function that has deviates yes/maybe (or that the CAT-14 rule
classified), quote the gr file:line, then:
- Original bytes: what the original does at the corresponding place (instruction addresses).
- Verdict: CONFIRMED (gr really differs; port keeps original) | REFUTED (gr matches the original /
  the marker is not a behavioural difference) | UNCLEAR (open question + address).
- Class: keep-original | harmless.
```
Count your confirmed / refuted / unclear per function in the index `confidence` column? No: put the
verdict counts in your final report; the index confidence reflects how sure you are of the note.

## Final report (to the caller)
Counts: full specs written, deviation notes written (confirmed / refuted / unclear rows), list of open
questions with addresses, files written. Keep it compact.

## File ownership when a subsystem is split between writers (RE-SPEC-2)
Each writer group owns the class files of the classes in its assign file. CAM is split (A: CCam, CIdleCam,
CMBlur, CHandShaker; B: CCamera, unnamed, plus INPUT) and VEH is split (C, D, E). An unnamed function or a
helper that belongs to a class owned by another group goes into `docs/re/specs/<SUB>/_unattributed-<G>.md`
(heading names the proposed class), never into another group's file. Static initialisers: `_static-init-<G>.md`
in split subsystems, `_static-init.md` otherwise.

## Spot checks (RE-SPEC-2)
The lead re-checks a sample of every writer's claims against the bytes (constants, operation order, branch
conditions, verdicts). Write every claim with the instruction address that proves it, so it can be checked.
