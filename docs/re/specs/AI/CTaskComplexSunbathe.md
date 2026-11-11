# CTaskComplexSunbathe (AI) - RE-SPEC-AI-2 group J3

## 0x632040 CTaskComplexSunbathe::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x1A1` (0x632040); `ret` -> returns 417 (TASK_COMPLEX_SUNBATHE). Vtable: slot 4 (+0x10) of vtable 0x86E0AC (CTaskComplexSunbathe).

## 0x632140 CTaskComplexSunbathe::CanSunbathe  (catalogue: specified/REA, port level 1)
- `__cdecl bool ()` (static, ECX unused), plain `ret`, returns EAX = 0/1.
- If ms_nGameClockHours [0xB70153] < 10 or >= 18 (unsigned) return 0. w = (u16)CWeather::NewWeatherType [0xC8131C]; if w is 1, 3,
  14, 5, 10 or 18 (the SUNNY_* types, tested in this order) return 1; else return 0x62F2D0(w) ? 1 : 0 (EAX pushed, callee reads
  the low word).
- 0x62F2D0 (cdecl `bool (int16 weather)`, proposed `CWeather::IsExtraSunny` / `IsWeatherExtraSunny`): 1 for 0, 2, 13, 6, 11, 17
  (the EXTRASUNNY_* types), else 0.
- gta-reversed: install commented out (TaskComplexSunbathe.cpp:12).

## 0x632190 CTaskComplexSunbathe::ShouldLoadSunbatheAnims  (catalogue: specified/REA, port level 0)
- `__cdecl bool ()` (static), result AL. Entry 0x632190 is `nop; nop; jmp 0x402274` (relocated `movzx eax, [CWorld::PlayerInFocus
  0xB7CD74]`; `jmp 0x632197`).
- ped = CWorld::Players[PlayerInFocus].m_pPed (dword at 0xB7CD98 + 0x190*idx). If !(ped+0x46C & 0x100) (bInVehicle) or
  ped+0x58C (m_pVehicle) == NULL -> return 1. v = vehicle+0x44 (m_vecMoveSpeed): s = x*x; s += y*y; s += z*z (x87: fld z, y, x;
  x*x, + y*y, + z*z, 0x6321B8-0x6321CF); compare with [0x863244]=0.04f: `fcomp; test ah,0x41; jne` -> return 1 if s <= 0.04 or NaN;
  return 0 (AL) if s > 0.04.
- gta-reversed: install commented out (TaskComplexSunbathe.cpp:13).

## 0x6366A0 CTaskComplexSunbathe::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x38) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexSunbathe::CTaskComplexSunbathe(this+0x34, (u8)this+0xC) 0x631F80; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

## 0x6381A0 CTaskComplexSunbathe::ControlSubTask  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`.
- Fields: +8 m_pSubTask, +0xD u8, +0xE/+0xF u8 "anim block referenced" flags, +0x10 u8, +0x14/+0x18/+0x1C CTaskTimer (start,
  interval, started u8), +0x1D u8, +0x24/+0x28 CAnimBlock* (block+0x10 = loaded), +0x2C/+0x30 anim block indices.
- Behaviour:
  1. If +0x10 != 0 return NULL.
  2. If +0xE == 0: if (+0x24)->+0x10 != 0 { CAnimManager::AddAnimBlockRef(+0x2C) 0x4D3FB0; +0xE = 1 } else CStreaming::RequestModel(+0x2C
     + 0x63E7, 8) 0x4087E0.
  3. If +0xF == 0 and ShouldLoadSunbatheAnims() 0x632190: same with +0x28 / +0x30 / flag +0xF.
  4. If +0xD != 0: ped+0x528 |= 8; ped+0x474 |= 0x100000.
  5. If !CanSunbathe() 0x632140: if +0x1C != 0 { +0x1D = 1; +0x18 = +0x18 - (m_snTimeInMilliseconds - +0x14) }; if
     m_pSubTask->GetTaskType() == 0x1A2 (418, SIMPLE_SUNBATHE): m_pSubTask->MakeAbortable(ped, 0, NULL) (vtable +0x18, result ignored).
  6. Return m_pSubTask (re-read).
- gta-reversed: not hooked at this address.

## 0x638290 CTaskComplexSunbathe::CreateSubTask  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8] unused)`, `ret 8`, SEH frame. Entry is `nop; jmp
  0x409B02` (relocated `mov eax, fs:[0]`), back at 0x638296. The taskType stack slot is reused to hold the new task pointer.
- Callees: CTaskTimer::GetTimeLeft 0x62EB80 (unnamed in the catalogue; `started (+8) ? start (+0) + interval (+4) - now : 0`, `now`
  loaded through an obfuscated pointer that resolves to 0xB7CB84), CGeneral::GetRandomNumberInRange(int) 0x407180, CTaskSimpleRunAnim
  ctor 0x61A900 (group, anim, blend, taskType, name, holdLast), CTaskSimpleRunTimedAnim ctor 0x61ABE0, CTaskSimplePause ctor 0x48E750.
- Fields: +0x20 sunbathe variant (int32), +0x14 timer, +0x24/+0x28 anim blocks (+0x10 loaded flag).
- Behaviour: if m_pSubTask != NULL and m_pSubTask->GetTaskType() == taskType return m_pSubTask. Else by taskType (allocation
  failures give NULL):
  - 0xCA (PAUSE): if ((+0x28)->loaded == 0 and ShouldLoadSunbatheAnims()) -> new(0x18) Pause(10000); else if (+0x24)->loaded != 0 ->
    new(0x18) Pause(GetRandomNumberInRange(1000, 5000)); else Pause(10000).
  - 0x1AC (428, START_SUNBATHING): new(0x20) RunAnim(0x30, +0x20 + 0xF4, 4.0f, 0x1AC, "start sunbathing" [0x86E654], 1).
  - 0x1A2 (418, SUNBATHE): anim = +0x20 + 0xEF. If (+0x28)->loaded and (+0x20 == 0 or +0x20 == 2): hi = GetTimeLeft(+0x14) + 1000;
    if hi > 12000 hi = 12000 (GetTimeLeft called again for the value when <= 12000); dur = GetRandomNumberInRange(3000, hi). Else dur
    = GetTimeLeft(+0x14) + 1000. new(0x34) CTaskSimpleRunTimedAnim(0x2F, anim, 4.0f, -4.0f (imm 0xC0800000), dur, 0x1A2, "sunbathe"
    [0x86E098], 1).
  - 0x1AD (429, IDLE_SUNBATHING): v = +0x20; anim = v == 0 ? GetRandomNumberInRange(0, 3) + 0xFE : v == 2 ?
    GetRandomNumberInRange(0, 3) + 0x101 : the taskType slot value (= 0x1AD; original quirk, 0x6384BC); new(0x20) RunAnim(0x30, anim,
    4.0f, 0x1AD, "idle sunbathing" [0x86E644], 1).
  - 0x1AE (430, STOP_SUNBATHING): new(0x20) RunAnim(0x30, +0x20 + 0xF9, 4.0f, 0x1AE, "stop sunbathing" [0x86E634], 0).
  - 0x516 (FINISHED) and anything else: NULL.
- gta-reversed: not hooked at this address.

## 0x6399F0 CTaskComplexSunbathe::CreateNextSubTask  (catalogue: specified/REA, port level 6)
- Convention: `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. CS(x) = this->CreateSubTask(x, ped) 0x638290. loaded(b) = byte
  (b)+0x10 != 0 for the anim blocks +0x24 / +0x28.
- Behaviour:
  1. If ped+0x598 (ped type) is 0x16 (22) or 5: g_InterestingEvents.Add(2, ped) 0x602590 (this = 0xC0B058).
  2. Switch on m_pSubTask->GetTaskType() (byte table 0x639BC0 / dword table 0x639BA8 on t - 0xCA, range 0..0xE4; anything else
     -> NULL):
     - 0xCA (PAUSE): if +0xD != 0: return (ShouldLoadSunbatheAnims() && loaded(+0x28)) ? CS(0x1AE STOP) : CS(0x516 FINISHED). Else:
       return (loaded(+0x28) && loaded(+0x24)) ? CS(0x1AC START) : CS(0x516).
     - 0x1AC (START): if loaded(+0x24): now = m_snTimeInMilliseconds (read first); i = CGeneral::GetRandomNumberInRange(20000,
       100000) 0x407180; +0x14 = now; +0x18 = i; +0x1C = 1; r = CS(0x1A2 SUNBATHE); +0xD = 1; return r. Else CS(0x516).
     - 0x1A2 (SUNBATHE): if !loaded(+0x28) return CS(0xCA PAUSE). If !CanSunbathe() or CTaskTimer::IsOutOfTime(+0x14) 0x420E30 return
       CS(0x1AE). If +0x20 (variant) is 0 or 2 return CS(0x1AD IDLE); else CS(0x1AE).
     - 0x1AD (IDLE): return loaded(+0x24) ? CS(0x1A2) : CS(0x516).
     - 0x1AE (STOP): r = CS(0x516); +0xD = 0; return r.
- gta-reversed: not hooked at this address.

## 0x639CB0 CTaskComplexSunbathe::CreateFirstSubTask  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. CS(x) = CreateSubTask(x, ped) 0x638290; loaded() as in
  0x6399F0.
- Behaviour:
  1. `_rand()` 0x821B1E is called once and its result discarded (0x639CB4) - keep (advances the CRT RNG).
  2. Variant +0x20: if ped type (+0x598) == 5: v = 2 - _ftol((float)(rand() & 0xFFFF) * [0x858B14]=3.0517578e-05f * [0x859024]=-3.0f)
     (fild, fmul, fmul, _ftol 0x821B40 truncation; gives 2..4). Else if CPopulation::IsSunbather((int16)ped+0x22) 0x611760: v =
     (rand() < 0x3FFF) ? 1 : 0. Else v = 1.
  3. For each anim block not loaded: CStreaming::RequestModel(index + 0x63E7, 8) for (+0x24, +0x2C) then (+0x28, +0x30).
  4. +0xD = 0; s = ShouldLoadSunbatheAnims().
  5. If +0xC != 0 and s: if +0x34 == 0 and loaded(+0x28) return CS(0x1AC START); else return CS(0xCA PAUSE).
  6. If (+0xC != 0 and !s) or (+0xC == 0 and !s): if !loaded(+0x24) return NULL.
  7. If loaded(+0x24): r = rand() & 0xFFFF; +0x14 = m_snTimeInMilliseconds; +0x18 = 20000 - _ftol((float)r * 3.0517578e-05f *
     [0x86E668]=-80000.0f); +0x1C = 1; +0xD = 1; return CS(0x1A2 SUNBATHE). Else (only reachable with s true): return CS(0xCA).
- gta-reversed: not hooked at this address.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x62EB80 CTaskComplexSunbathe::sub_62EB80 (proposed: CTaskTimer::GetTimeLeft)  (catalogue: unknown/REA, port level 0)
- Name correction: operates on a CTaskTimer {+0 m_nStartTime, +4 m_nInterval, +8 m_bStarted}. Caller: CTaskComplexSunbathe::CreateSubTask 0x638290 (3 sites, on its timer member).
- `__thiscall int32 (this: CTaskTimer*)`, plain `ret`.
- Behaviour: `if m_bStarted == 0` -> return 0 (0x62EB95). Else (through a .HOODLUM detour 0x40131C that loads `now = [0xB7CB84 CTimer::m_snTimeInMilliseconds]`; the address is computed as `[0x4EC103] ^ [0x156061C]`) return `m_nInterval - now + m_nStartTime` (u32 wrap-around arithmetic, 0x62EB8D-0x62EB92).
