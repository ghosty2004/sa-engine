# _unattributed-FA (PED) - RE-SPEC-FINAL group A

## 0x453F40 sub_453f40 (player ped state setup)  (catalogue: unknown/REA, port level 3)
Catalogue name: <none>, proposed: anim finish callback `static void <Pickups/AssetPurchase>::FinishAnimCB2(CAnimBlendAssociation* anim, CPed* ped)`
(registered with SetFinishCallback at 0x4543F1 inside 0x4541E0; also in the data table 0x8D0954). Owner class UNCLEAR (same as 0x453E40).
- Convention: `__cdecl void (CAnimBlendAssociation* anim [esp+4], CPed* ped [esp+8])`, plain `ret`.
- Behaviour: `anim->+0x2E |= 4` (flags: delete when blended out); `anim->+0x1C (m_fBlendDelta) = -1000.0f` (imm 0xC47A0000);
  `CPad::GetPad(0)->+0x10E &= 0xBF` (clear bit 6); if `anim->m_fBlendAmount (+0x18) > 0.5f` ([0x858B8C]=0.5f, `test ah,0x41; jne`):
  `ped->+0x46C |= 0x4000`; then (independent of the blend test) if `ped->m_nPedState (+0x530) == 0x13` call `ped->SetPedState(1)` 0x5E4500.
- gta-reversed: absent. Open questions: owner class; names of pad bit +0x10E&0x40 and ped flag 0x4000.

## 0x5DE9E0 CAEPedAudioEntity-member dtor (unwind)  (catalogue: unknown/REA, port level 2)
- `__thiscall void (this)`: `vtable = 0x859988` (CAEAudioEntity), tail-jump `CAESound::~CAESound(this+8)` 0x4EF660 - inlined CAEAudioEntity dtor used
  as SEH unwind action of CPed::Constructor 0x5E8030. Open questions: none.
