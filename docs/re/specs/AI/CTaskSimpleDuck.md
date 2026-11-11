# CTaskSimpleDuck (AI) - RE-SPEC-REST group A

## 0x61C420 CTaskSimpleDuck::StopFireGun  (catalogue: specified/REA, port level 0)
- Name note: the bytes do not stop anything; it is a const predicate. Proposed name `CTaskSimpleDuck::IsDuckControlled...` UNCLEAR;
  keep the catalogue name. `__thiscall bool ()`, plain `ret`, returns EAX = 0/1.
- Behaviour: if this+0x1C (f32) != 0.0f [0x858B50] (`fcomp; test ah,0x44; jp` -> taken when not equal or NaN) return 1.
  p = this+0x10 (anim association pointer); if p == NULL return 1. `fld [p+0x18]; fcomp 1.0f [0x858624]; test ah,5; jnp` -> return 1
  when blend (p+0x18) >= 1.0 or NaN; continue only when blend < 1.0. Then if this+0x19 (u8) != 0 return 1; if (int16)this+0xE > 0
  return 1; else return 0.
- Summary: return !(this+0x1C == 0.0 && p != NULL && p->blend < 1.0 && this+0x19 == 0 && (int16)this+0xE <= 0).

## 0x692340 CTaskSimpleDuck::AbortBecauseOfOtherDuck  (catalogue: specified/REA, port level 1)
- `__thiscall void (this, CPed* ped [esp+4] unused)`, `ret 4`, returns AL = 1 (left in EAX; callers treat it as void).
- If +0x10 (m_pDuckAnim) != NULL: SetFinishCallback(0x4D1390, NULL) 0x4CEBE0; +0x10 = 0. If +0x14 (m_pMoveAnim) != NULL: anim+0x1C
  (blend delta) = -8.0f (imm 0xC1000000); SetFinishCallback(0x4D1390, NULL); +0x14 = 0. Then +0x1A = 1 and +0x18 = 1 (u8, in this
  order; gr m_bIsAborting / m_bIsFinished).
