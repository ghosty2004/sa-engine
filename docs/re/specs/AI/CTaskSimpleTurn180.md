# CTaskSimpleTurn180 (AI) - RE-SPEC-FINAL (helper B)

## 0x631D60 CTaskSimpleTurn180::FinishAnimCB  (catalogue: unknown/REA, port level 0)
- `static void __cdecl (CAnimBlendAssociation* assoc, void* data [esp+8])`, plain `ret`. Referenced by CTaskSimpleTurn180::ProcessPed 0x639990.
- Behaviour: `data->+8 (finished) = 1; data->+0xC (m_pAnim) = 0` (0x631D64/0x631D68).
