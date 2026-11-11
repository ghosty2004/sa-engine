# _unattributed-FA (SCRIPT) - RE-SPEC-FINAL group A

## 0x45AFB0 sub_45afb0 (vector by value helper)  (catalogue: unknown/REA, port level 0)
Catalogue name: <none>, proposed: `void CPhysical::SetTurnSpeed(CVector v)` (by value; evidence: writes this+0x50 = CPhysical::m_vecTurnSpeed;
callers: script opcodes (ProcessCommands900To999 0x483BD0, 1300To1399 0x48CDD0, 1400To1499 0x48EAA0, 2000To2099 0x472310) and
CAutomobile::ProcessHarvester 0x6A9680).
- Convention: `__thiscall void (CPhysical* this, float x [esp+4], float y [esp+8], float z [esp+0xC])`, `ret 0xC`. 0x45AFB0 jumps to the
  HOODLUM body 0x15658A0. Copies through a 12-byte local (dword moves, no x87).
- Behaviour: `this->+0x50 = x; +0x54 = y; +0x58 = z` (bit copies).
- gta-reversed: CPhysical::SetTurnSpeed (inline in gr headers) - same. Open questions: none.

## 0x6F9E00 CScripted2dEffects ms_userLists element dtor  (catalogue: unknown/REA, port level 0)
- Empty body (`ret`); element dtor of CScripted2dEffects::ms_userLists (tUserList). Open questions: none.

## 0x6F9E30 sub_6F9E30 (empty)  (catalogue: unknown/REA, port level 0)
- Empty body (`ret`); referenced by 0x6F9E60 / 0x6FA790 as an element dtor (CScriptedEffectPair). Open questions: none.
