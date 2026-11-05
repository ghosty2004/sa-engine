# CEntryExitManager: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x43efd0 CEntryExitManager::GetEntryExitIndex  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/EntryExitManager.cpp:333` (pool-iteration): `for (auto&& [i, enex] : mp_poolEntryExits->GetAllValidWithIndex())`.
  - Original bytes: pool pointer read via obfuscated load in the stub at 0x40156b (EDI = mp_poolEntryExits). `esi = pool->m_nSize` (0x43efda); loop body decrements first: `flags = byFlags[esi-1]` (0x43efe9), `dec esi` (0x43efed), slot skipped if flag byte bit 7 set (`js`, 0x43eff3) or if `objects + esi*0x3c == 0` (0x43eff9). Loop runs while `esi != 0` (0x43f030). So slots are visited from size-1 down to 0. On a match it returns ESI = slot index (0x43f03d); none: -1 (0x43f037).
  - Flag test (matches gr): `(m_nFlags & (uint8)enabled) == (uint8)enabled` in 16 bit with zero-extended low byte of the arg (0x43effb-0x43f00b), then `(uint8)m_nFlags & (uint8)disabled` must be 0 (`test byte [esp+0x1c], dl`, 0x43f010). Name compare `_strnicmp(enex->m_szName (+0), name, 8)` (0x43f01e, CRT 0x835ac7). After a failed compare, EDI is reloaded from 0x96a7d8 directly (0x43f02a).
  - Verdict: CONFIRMED. Original scans descending; gr ascending. If several enexes match, the original returns the highest slot index, gr the lowest.
  - Class: keep-original.

## 0x43f0a0 CEntryExitManager::PostEntryExitsCreation  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/EntryExitManager.cpp:302` (pool-iteration): `for (auto& enex : mp_poolEntryExits->GetAllValid()) if (enex.bCreateLinkedPair && !enex.m_pLink) LinkEntryExit(&enex);`
  - Original bytes: body 0x15631c0. Descending loop from `edi = m_nSize` (`dec edi` first, 0x15631e7) to 0, pool pointer reloaded from 0x96a7d8 every iteration (0x15631d6); skip free / null. Condition `byte [enex+0x30] & 4` and `[enex+0x38] (m_pLink) == 0` (0x15631f5-0x1563200). LinkEntryExit (0x43f050) is inlined: `idx = GetEntryExitIndex(enex /*name at +0*/, 0, 4)` (0x1563207); if `idx != -1`: `linked = (flags[idx] & 0x80) ? null : objects + idx*0x3c` (0x156321d-0x156322c); `enex->m_pLink = linked` (0x156322e); `if (linked->m_pLink == null) linked->m_pLink = enex` (0x1563231-0x1563238); `linked->m_nTimeOn (+0x34) = 0; linked->m_nTimeOff (+0x35) = 24` (0x156323b-0x156323f). No null check on `linked` (cannot be null in practice because GetEntryExitIndex only returns live slots).
  - Verdict: CONFIRMED. Order is descending. It matters: when several linked-pair enexes share a partner, the partner's back link is set by the first one processed (highest slot in the original, lowest in gr).
  - Class: keep-original.
- Found in passing (gr LinkEntryExit, EntryExitManager.cpp:322, used by gr's version of this function): gr writes the back link `if (linkedEnEx->m_pLink)` (when it is already set); the original writes it when it is null (`test ecx,ecx; jne skip`, 0x1563234). CONFIRMED, keep-original.

## 0x43f180 CEntryExitManager::EnableBurglaryHouses  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/EntryExitManager.cpp:288` (pool-iteration): `for (auto& enex : mp_poolEntryExits->GetAllValid()) enex.bBurglaryAccess = enable;`
  - Original bytes: body 0x156c6d0. `ms_bBurglaryHousesEnabled (0x96a730) = enable` (0x156c6da). Loop from `esi = m_nSize` down to 0 (dec at 0x156c6fc), skip free slots (`js`, 0x156c702) and null object address (0x156c708). The loop reloads the pool pointer from 0x96a7d8 each iteration (0x156c6f0).
  - Verdict on the iteration row: CONFIRMED that the order differs (descending in the original), but each iteration touches only its own enex, so order has no effect. Class: harmless.
- Found in passing (real difference, keep-original): the original does NOT write bBurglaryAccess. It reads `m_nFlags` (+0x30, u16) and only if bit 0x1000 (gr name `bBurglaryAccess`) is set (`test ah,0x10`, 0x156c710) it sets (`or eax,0x4000`, 0x156c719) or clears (`and eax,0xbfff`, 0x156c720) bit 0x4000 (gr name `bEnableAccess`) according to `enable`, then stores the word back (0x156c725). Enexes without 0x1000 are untouched. gr sets bit 0x1000 on every enex. Port: `for slot in (0..size).rev() { if valid && flags & 0x1000 != 0 { flags = if enable { flags | 0x4000 } else { flags & 0xbfff } } }`. (If gr's bit names are swapped, the gr code is still wrong: it writes the tested bit, not 0x4000.)

## 0x43fa00 CEntryExitManager::AddOne  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
Args (cdecl, bytes): +4 cx, +8 cy, +0xc cz, +0x10 entranceAngle (degrees), +0x14 rangeX, +0x18 rangeY, +0x1c rangeZ, +0x20..+0x28 exit pos, +0x2c exitAngle, +0x30 area, +0x34 flags, +0x38 skyColor, +0x3c timeOn, +0x40 timeOff, +0x44 numPeds, +0x48 name.
- gr `source/game_sa/EntryExitManager.cpp:194` (fix-bugs-ifdef, 194-198): FIX_BUGS returns -1 when the pool is full; the `#else` branch returns 0.
  - Original bytes: `ptr = CPool::New` (0x43f7d0); if null: `xor eax,eax` and return (0x43fa30-0x43fa48).
  - Verdict: CONFIRMED. The original returns 0 (a valid-looking index) on a full pool.
  - Class: keep-original.
- gr `EntryExitManager.cpp:375` (notsa-function): `AddEnExToWorld` "NOTSA (Code somewhat based on 0x43FC00)", which rotates the two rect corners by `m_fEntranceAngleRad` and adds the min/max rect to the quad tree.
  - Original bytes (inline in AddOne): `hx = rangeX * 0.5f`, `hy = rangeY * 0.5f` [0x858b8c]; corners `v0 = (rangeX * -0.5f [0x858f40], rangeY * -0.5f, 0)` and `v1 = (hx, hy, 0)`; `CMatrix m; m.SetRotateZ(entranceAngle)` (0x59b0e0) where the argument is the RAW `+0x10` argument (degrees), not the radians value stored in the enex (`+0x1c = angle * 0.01745329238f [0x8595ec]`, 0x43fb6a-0x43fb77); for each corner `v = m * v` (0x59c890), `v.x += cx`, `v.y += cy` (0x43fc10-0x43fc52). Rect (CRect: left +0, top +4, right +8, bottom +0xc): left/right = min/max of v0.x, v1.x and bottom/top = min/max of v0.y, v1.y, chosen with `if (v0.c < v1.c)` (ordered, 0x43fc54-0x43fca2); `mp_QuadTree (0x96a7d0)->AddItem(enex, &rect)` (0x552cd0).
  - Verdict: CONFIRMED. The original rotates the quad-tree rect by the angle in degrees interpreted as radians (an original bug); gr uses the correct radians. The quad-tree rect therefore differs for any non-zero angle (affects which enexes the quad tree returns near the entrance).
  - Class: keep-original.
- Other inline parts (match gr's LinkEntryExit except the back-link test, see PostEntryExitsCreation 0x43f0a0): `if (flags & 4)` link via `GetEntryExitIndex(enex, 0, 4)`; back link set only if `linked->m_pLink == null` (0x43fcec-0x43fcf3). Return index = `(enex - pool->objects) / 0x3c` via the 0x88888889 multiply (0x43fd03-0x43fd16).

## 0x43fd50 CEntryExitManager::DeleteOne  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/EntryExitManager.cpp:225` (fix-bugs-ifdef, 225-227): `std::destroy_at(enex)` before `mp_poolEntryExits->Delete(enex)`.
  - Original bytes: body 0x1560ed0. If `flags[index] & 0x80` (free) return (0x1560ee3). Else `enex = objects + index*0x3c`; `mp_QuadTree->DeleteItem(enex)` (0x552a40); inline CPool::Delete: recompute the index (`/0x3c` via 0x88888889), `flags[idx] |= 0x80`, `if (idx < pool->m_nFirstFree (+0xc)) m_nFirstFree = idx` (0x1560f14-0x1560f27). No destructor call.
  - Verdict: CONFIRMED (no destructor in the original). gr's CEntryExit has no user-declared destructor, so `destroy_at` is a no-op.
  - Class: harmless.

## 0x440b90 CEntryExitManager::Shutdown  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/EntryExitManager.cpp:51` (pool-iteration): `for (auto& enex : mp_poolEntryExits->GetAllValid()) mp_QuadTree->DeleteItem(&enex);`
  - Original bytes: body 0x1563940. Descending loop (`dec esi` after reading `flags[esi-1]`, 0x1563959-0x156395d), skip free/null, `CQuadTreeNode::DeleteItem(enex)` (0x552a40), pool pointer reloaded after each call. Then the pool is destroyed (inline: if `m_nSize > 0` and the "owns memory" byte +0x10 is set, frees objects/flags through an obfuscated call at 0x1563998; then zeroes the fields; `operator delete(pool)` 0x8214bd), `mp_poolEntryExits = 0`, quad tree destroyed (`CQuadTreeNode` dtor 0x552520 + delete 0x552c90), `mp_QuadTree = 0`, `ms_entryExitStackPosn = 0`.
  - Verdict: CONFIRMED order differs (descending), but every item is removed and the tree is deleted right after; no observable difference.
  - Class: harmless.

## 0x440c40 CEntryExitManager::ShutdownForRestart  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/EntryExitManager.cpp:66` (pool-iteration): `for (auto&& [i, enex] : GetAllValidWithIndex()) { enex.bEnableAccess = true; if (enex.bDeleteEnex) DeleteOne(i); }`
  - Original bytes: body 0x1567550. Descending loop over slots (0x1567570-0x15675e7), skip free/null; `byte [enex+0x31] |= 0x40` (flag 0x4000, 0x1567585); `if (m_nFlags & 0x8000)` (`test ah,ah; jns`, 0x156758d) DeleteOne inlined (slot re-check, `mp_QuadTree->DeleteItem`, flag |= 0x80, m_nFirstFree = min). Then `EnableBurglaryHouses(0)` (0x43f180), `if (mp_Active) { ResetAreaCodeForVisibleObjects(); ms_exitEnterState = 0; mp_Active = 0; }`, `ms_entryExitStackPosn = 0`, `ms_bDisabled = 0`.
  - Verdict: CONFIRMED order differs, but the per-slot work is independent and m_nFirstFree ends at the minimum deleted index either way. No observable difference.
  - Class: harmless. (Note: the original EnableBurglaryHouses(false) called here clears 0x4000 again on every enex with flag 0x1000; see 0x43f180.)

## 0x5d5970 CEntryExitManager::Save  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/EntryExitManager.cpp:453` (pool-iteration): `for (auto&& [i, enex] : mp_poolEntryExits->GetAllValidWithIndex())` writing (int16 index, u16 flags, int16 link index).
  - Original bytes: ASCENDING loop `ebx = 0, 1, ...` with the bound hard-coded as `ebp < 0x5dc0` (= 400 * 0x3c, 0x5d5a5b-0x5d5a65), not the pool's m_nSize; skip free/null. Per enex: link index = `(link - objects) / 0x3c` (0x88888889 multiply), kept only if `0 <= idx < m_nSize` and the slot is not free, else -1 (0x5d59f3-0x5d5a27); then `Save(&i, 2)`, `Save(&enex->m_nFlags (+0x30), 2)`, `Save(&link, 2)`. Terminator `(int16)-1` (0x5d5a72). Stack part: `Save(&ms_entryExitStackPosn, 4)`, then for `i < posn` (signed) `Save(&(int16)index(stack[i]), 2)`. Returns `AL = 1`.
  - Verdict: REFUTED (ascending in the original too). Found in passing: the fixed 400-slot bound equals the pool size created in Init (400), so harmless.
  - Class: harmless.
