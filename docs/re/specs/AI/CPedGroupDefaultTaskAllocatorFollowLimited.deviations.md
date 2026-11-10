# CPedGroupDefaultTaskAllocatorFollowLimited: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x5F6C70 CPedGroupDefaultTaskAllocatorFollowLimited::AllocateDefaultTasks  (gr_deviations = 1 yes; 1 assert no)
Original bytes (`__thiscall void (const this, CPedGroup* group [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH; `this` unused). Group
layout: +0x28 = membership.members[7] (leader); default task pairs at group+0x21C (= intelligence (+0x30) + 0x1EC), stride 0x14
(+0 Ped, +4 Task).
  1. Followers (0x5F6C94-0x5F6D12): for i = 0..6 (ONLY the 7 follower slots; `esi = 8*i < 0x38`): pair = defaultPairs[i]; skip if
     pair.Ped == 0, or if ped != 0 and pair.Ped != ped. Else `t = CTask::operator new(0x28)`; if t != 0
     `CTaskComplexFollowLeaderInFormation(t, group, leader (group+0x28), &off, 10.0f (imm 0x41200000))` 0x6949A0 with
     `off = { ms_offsets[i].x [0xC196E8 + 8*i], ms_offsets[i].y [0xC196EC + 8*i], 0.0 }` (z stored at 0x5F6CED); `pair.Task = t` (or 0).
     The previous pair.Task is overwritten without deletion. No early exit after a match.
  2. Leader (0x5F6D14-0x5F6D81): pair = defaultPairs[7] (+0x2A8 Ped, +0x2AC Task); same Ped / ped filter; `t = operator new(0x38)`; if
     t != 0: `heading = ftol((rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * 8.0f [0x859000])` (0..7; rand called only when the
     allocation succeeded); `CTaskComplexWanderGang(t, 4 (PEDMOVE_WALK), heading, 5000, 1, 0.5f [0x86FCB0])` 0x66F5C0; `pair.Task = t` (or 0).
Row PedGroupDefaultTaskAllocatorFollowLimited.h:44 (`if (notsa::IsFixBugs()) { if (ped) break; }`):
- Original: no break; the loop always runs to i = 6 and the leader block always runs (0x5F6D09-0x5F6D14).
- Verdict: CONFIRMED (with FixBugs gr stops after the first match). Class: keep-original.
Row .h:27 `assert(!tp.Task)`: REFUTED (debug assert only; original overwrites). Class: harmless.
Additional CONFIRMED difference (no row, gr code at .h:19-41 is wrong): gr chooses the task by `ped == leader` (the PARAMETER) and loops
over all 8 pairs; the original gives every follower slot 0..6 a CTaskComplexFollowLeaderInFormation and the leader slot 7 a
CTaskComplexWanderGang, independent of `ped`. gr also uses the gr RandomNodeHeading helper. Port from the bytes.
