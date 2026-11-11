# CTaskComplexPlayHandSignalAnim (AI) - RE-SPEC-3 group I2
Vtable 0x86D5DC, size 0x18. Layout (gr): +0xC m_animationId i32 (-1 = choose from ped type), +0x10 m_AnimBlenDelta f32, +0x14 flags
(bit0 left hand model ref held, bit1 right hand ref held, bit2 anim block ref held, bit3 use fat hands). Hand models 394..397
(0x18A shandl, 0x18B shandr, 0x18C fhandl, 0x18D fhandr); load state = `CStreaming::ms_aInfoForModel[m].m_nLoadState` (bytes 0x8E6B98 /
0x8E6BAC / 0x8E6BC0 / 0x8E6BD4, value 1 = loaded); model info pointers 0xA9B6F0..0xA9B6FC. Anim block "ghands" (0x86D608), index cached in
`ms_animBlock` [0x8D2E38] (initially -1); block load state byte at 0x961ADC + 20*idx.

## 0x61B2F0 CTaskComplexPlayHandSignalAnim::CreateSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, int32 type)`, `ret 4`, SEH.
- 0xCB: `new(0x20) CTaskSimpleStandStill(0, true, false, 8.0f)` (or 0).
- 0x1A9: `seq = new(0x40) CTaskComplexSequence()` (may be 0 - AddTask then runs with ECX = 0, ORIGINAL BUG); `s = new(0x20)`; if s:
  `CTaskSimpleStandStill(s, CGeneral::GetRandomNumberInRange(0, 1500), false, false, 8.0f)` (RNG drawn only after a successful allocation);
  `seq->AddTask(s)`; `h = new(0x24)`; if h: `CTaskSimplePlayHandSignalAnim(h, m_animationId, m_AnimBlenDelta, fat, false)` where `fat =
  (i8)(flags << 4) >> 7` (0 or -1; the ctor stores `!= 0`); `seq->AddTask(h)`; return seq.
- other: 0.

## 0x61B460 CTaskComplexPlayHandSignalAnim::GetAnimIdForPed  (catalogue: specified/REA, port level 0)
- `__thiscall int32 (this, CPed* ped)`, `ret 4`: by `ped->m_nPedType (+0x598)`: 7 -> 0x140, 8 -> 0x141, 9 -> 0x142, 0xB -> 0x144,
  0xE -> 0x143 (computed `((type != 0xE) - 1) & 0x144) - 1`), anything else -> -1.

## 0x61B4F0 CTaskComplexPlayHandSignalAnim::CreateFirstSubTask  (catalogue: specified/REA, port level 5)
- If `g_ikChainMan.IsArmPointing(1, ped)` return 0. If `m_animationId == -1`: `m_animationId = GetAnimIdForPed(ped)`; if still -1 return 0.
  `flags bit3 = (ped model (i16 +0x22) in {0x67, 0x69, 0x6B})`; return `CreateSubTask(0xCB)`.

## 0x61B570 CTaskComplexPlayHandSignalAnim::CreateNextSubTask  (catalogue: specified/REA, port level 0)
- returns 0 (`xor eax,eax; ret 4`).

## 0x61B580 CTaskComplexPlayHandSignalAnim::ControlSubTask  (catalogue: specified/REA, port level 1)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`. `sub = m_pSubTask` (read first).
- Hands (fat: models 0x18C/0x18D, else 0x18A/0x18B): left: if loaded { if !(flags & 1) { `CBaseModelInfo::AddRef(modelInfo)` 0x4C4BA0;
  flags |= 1 } } else `CStreaming::RequestModel(left, 8)`; right: same with bit 1.
- If both bits 0 and 1: `if (ms_animBlock == -1) ms_animBlock = GetAnimationBlockIndex("ghands")`; if the block is loaded:
  `if (!(flags & 4)) { AddAnimBlockRef(ms_animBlock); flags |= 4 }`; else `RequestModel(ms_animBlock + 0x63E7, 8)`.
- If `(flags & 4)` and `m_pSubTask->GetTaskType() == 0xCB`: return `CreateSubTask(0x1A9)`. Else return sub.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x61BA00 CTaskComplexPlayHandSignalAnim::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame (handler 0x83F2BB). No direct callers (vtable slot 1).
- Behaviour: `p = CTask::operator new(0x18)` 0x61A5A0 (cdecl); if p == NULL return 0 (0x61BA34). Read `a = this->+0x10`, `b = this->+0xC` first; `CTaskComplex::CTaskComplex(p)` 0x61A3B0; `p->+0x14 &= 0xF8` (bits 0-2 cleared, rest of that byte uninitialised heap data; 0x61BA4E); `p->vtbl = 0x86D5DC`; `p->+0xC = b`; `p->+0x10 = a`; return p.
- gta-reversed: CTaskComplexPlayHandSignalAnim (+0xC m_animId, +0x10 m_fDropOffSpeed? per gr header) - the clone copies both raw dwords; gr's Clone constructs via ctor which may reinitialise other fields; the bytes only clear the three flag bits.
