# CTaskComplexUseClosestFreeScriptedAttractor (AI) - RE-SPEC-FINAL (helper B)

## 0x634740 CTaskComplexUseClosestFreeScriptedAttractor::ComputeClosestFreeScriptedEffect  (catalogue: specified/REA, port level 4)
- `__thiscall C2dEffect* (this unused, CPed* ped [esp+4])`, `ret 4`; SEH frame (a local CMatrix). Caller: CreateFirstSubTask 0x639530.
- Behaviour: `best = FLT_MAX (0x7F7FFFFF); result = NULL`. For i = 0..63 (`effect = CScripted2dEffects::ms_effects 0xC3AB00 + i*0x40`, `list = ms_userLists 0xC3A200 + i*0x24`, loop until list == 0xC3AB00):
  1. Skip unless `ms_activated[i]` (u8 at 0xC3A1A0 + i).
  2. If `list->+0x20 (u8 use-list flag) != 0`: accept if `(int32)(int16)ped->m_nModelIndex (+0x22)` equals one of the 4 dwords `list->+0..+0xC`; otherwise accept if for some j in 0..3 `list[j] == -2 && list->+0x10[j] == ped->m_nPedType (+0x598)`; otherwise skip.
  3. `d = P - effect->m_pos (+0..+8)` with P = ped position (matrix pos or placement); `q = (dz*dz + dy*dy) + dx*dx` (x87 order); `q` is saved (fst, into the argument slot); `fcomp best; test ah, 5; jp skip`: continue only if `q < best`.
  4. A local CMatrix is built (attach fields zeroed, `CMatrix::SetScale(1.0f)` 0x59AED0) and destroyed (`~CMatrix` 0x59ACD0) without being used. `if CPedAttractorManager::HasEmptySlot(GetPedAttractorManager() 0x5EE190, effect, NULL)` 0x5EBB00 -> `best = q; result = effect`.
- Return result.

## 0x639530 CTaskComplexUseClosestFreeScriptedAttractor::CreateFirstSubTask  (catalogue: specified/REA, port level 6)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame. Field +0xC = move state.
- Behaviour: `e = ComputeClosestFreeScriptedEffect(this, ped)` 0x634740; NULL -> return 0. `CInterestingEvents::Add(&g_InterestingEvents 0xC0B058, 3, ped)` 0x602590. `p = CTask::operator new(0x24)`; NULL -> 0. By `+0xC`: 6 -> `CTaskComplexUseEffectRunning(p, e, NULL)` 0x636780; 7 -> `0x636860(p, e, NULL)` (sprinting variant); any other value (4 or not) -> `CTaskComplexUseEffect::CTaskComplexUseEffect(p, e, NULL)` 0x6321F0. Return p.
