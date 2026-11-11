# CPedGroupDefaultTaskAllocatorSitInLeaderCar (AI) - RE-SPEC-REST group B

## 0x5F6FC0 CPedGroupDefaultTaskAllocatorSitInLeaderCar::AllocateDefaultTasks  (catalogue: specified/REA, port level 4)
- `__thiscall void (this, CPedGroup* g [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH frame. `this` unused. Default task pairs:
  `pair[k] = g + 0x21C + 0x14*k` (+0 ped, +4 task); members `g + 0xC + 4*k` (k = 0..6 followers, 7 = leader at g+0x28).
- ASSIGN(k, task): if `pair[k].ped != 0` and (`ped == 0` or `pair[k].ped == ped`): `pair[k].task = task` (plain overwrite, the previous
  task is not deleted); else if task != 0: `task->vtbl[0](1)` (deleting dtor).
- 1. `L = g->+0x28`; L == 0 -> return. `V = L->+0x58C`; V == 0 -> return.
- 2. `s = new(0x40) ? CTaskComplexSequence() 0x632BD0 : 0`; `s->AddTask(new(0x50) ? CTaskComplexEnterCarAsDriver(V) 0x6402F0 : 0)`
  0x632D10; `s->AddTask(new(0x60) ? CTaskSimpleCarDrive(V, 0, 0) 0x63C340 : 0)`; ASSIGN(7, s) (leader pair g+0x2A8). (AddTask is
  called with this = 0 if the 0x40 allocation fails - unreachable in practice.)
- 3. `max = (u8)V->+0x488` (m_nMaxPassengers); `seat = 0`; for k = 0..6: if `member[k] != 0` and `seat < max` (signed compare):
  `door = CCarEnterExit::ComputeTargetDoorToEnterAsPassenger(V, seat)` 0x64F190 (cdecl); `s = new Sequence`; `s->AddTask(new(0x50) ?
  CTaskComplexEnterCarAsPassenger(V, door, false) 0x640340 : 0)`; `s->AddTask(new(0x60) ? CTaskSimpleCarDrive(V, 0, 0) : 0)`;
  ASSIGN(k, s); `seat++`.
- gta-reversed (unhooked code in PedGroupDefaultTaskAllocatorSitInLeaderCar.h:21) differs: it stores every follower sequence into the
  LEADER pair (`SetPedDefaultTask(leader, LEADER_MEM_ID, ...)`) instead of pair[k], and VERIFYs that the previous task is null; the bytes
  use pair[k] and overwrite. Port follows the bytes. Open questions: none.
