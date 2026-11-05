# CEntryExit: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x43e650 CEntryExit::GetEntryExitToDisplayNameOf  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/EntryExit.cpp:111` (todo): "TODO: Probably inlined from `CEntryExitManager`".
  - Original bytes: body at 0x1569780. The comment itself is not code; it is a note on the stack lookup.
  - Verdict: REFUTED (comment only, no behaviour).
  - Class: harmless.
- Found in passing (two real differences in the same function):
  1. this->m_nArea (+0x32) == 0. gr returns `nullptr` (EntryExit.cpp:108-110). Original: `mov cl,[eax+0x32]; test cl,cl; je 0x15697c3` (0x1569794-0x1569799) jumps straight to `ret` with EAX = this (EAX was set to ECX at 0x1569786). So the original returns `this` when the enex itself is in area 0. CONFIRMED, keep-original.
  2. The stack lookup. ms_entryExitStackPosn (0x96a7c4) is read through an obfuscated pointer: `[0x547b21] ^ [0x156054c]` = 0xec1befe9 ^ 0xec8d482d = 0x0096a7c4 (0x156979c-0x15697a8). Compare is signed: `cmp ecx,1; jle` (0x15697ab). Test: `ms_entryExitStack[posn-1] == ms_spawnPoint` via `[ecx*4+0x96a71c]` (stack base 0x96a720) at 0x15697b0. Returned value: `[ecx*4+0x96a718]` = `ms_entryExitStack[posn-2]` (0x15697b9). gr returns `ms_entryExitStack[posn]` (line 115). CONFIRMED, keep-original: return `ms_entryExitStack[posn - 2]`.
  - Full original logic: `sp = ms_spawnPoint (0x96a7c0); if (sp->m_nArea == 0) return null; if (this->m_szName[0] == 0) return null; if (this->m_nArea == 0) return this; posn = (int32)ms_entryExitStackPosn; if (posn > 1 && stack[posn-1] == sp) return stack[posn-2]; return this;`

## 0x43f1f0 CEntryExit::WarpGangWithPlayer  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/EntryExit.cpp:589` (original-comment): "Original code tried only twice, but we'll try all offsets". gr loops `do { if (idx >= 8) return; to = memPos + offsets[idx++]; } while (!IsTeleportPointValid(plyrPos, to));`.
  - Original bytes (`ret 4`, ECX unused, player = arg +4 in EBP): group = `CPedGroups::ms_activeGroups[player->m_pPlayerData(+0x480)->m_nPlayerGroup(+0x38)]` (active byte at 0xc098e0 + id, membership at 0xc09928 + id*0x2d4); return if inactive or `!IsLeader(player)` (0x5f69c0). For `i = 0..6` (`cmp eax,7; jl`, 0x43f496-0x43f49e): `m = GetMember(i)` (0x5f69b0); skip null or `m == player`; if the shared byte offset `esi >= 0x40` (8 entries) skip. Up to THREE attempts, unrolled (0x43f267-0x43f2cf, 0x43f2de-0x43f341, 0x43f34c-0x43f3b8): `to = (playerPos.x + off[k].x, playerPos.y + off[k].y, playerPos.z)` with `off` = `CTaskComplexFollowLeaderInFormation::ms_offsets` (0xc196e8, 8-byte entries), `k++`, accepted if `IsTeleportPointValid(playerPos, &to)` (0x43ea90). Before attempts 2 and 3 `if (esi >= 0x40) return` (0x43f2d5, 0x43f343). If the third attempt fails the function RETURNS (`je 0x43f4a4`, 0x43f3b8), abandoning the remaining members.
  - Verdict: CONFIRMED. The comment's "twice" is inaccurate (three attempts), and gr changes behaviour: it keeps trying further offsets instead of returning after the third failure.
  - Class: keep-original.
- Found in passing (keep-original, CONFIRMED):
  1. The candidate position is the PLAYER position plus the offset (x, y; z = player z), not the member position as in gr (`memPos + offsets[...]`, EntryExit.cpp:596). The third attempt builds it with `CVector operator+` (0x40fe30) from `(off.x, off.y, 0)` and the player position.
  2. Heading: `d = (playerPos.x - memPos.x, playerPos.y - memPos.y, 0)` stored f32, `CVector::Normalise(&d)` (0x59c910), `h = CGeneral::LimitRadianAngle(CGeneral::GetRadianAngleBetweenPoints(d.x, d.y, 0, 0))` (0x53cbe0, 0x53cb50). gr uses `(plyrPos - memPos).Heading()` = `atan2(-x, y)` on the unnormalised vector; equality of the two was not checked (open question, 0x43f40b).
  3. Member loop covers slots 0..6 only (the leader slot 7 is never visited); gr iterates `GetMembers()` including the leader (skipped by the `== player` test, so harmless).
  4. Teleport is `vtbl[+0x38](to, false)`; then `FlushImmediately(false)`; `m_fCurrentRotation (+0x558) = m_fAimingRotation (+0x55c) = h`; heading via `CMatrix::SetRotateZOnly(h)` if a matrix exists, else placement heading (+0x10) = h; area code byte +0x2f and m_pEnex (+0x78c) copied directly from the player.
