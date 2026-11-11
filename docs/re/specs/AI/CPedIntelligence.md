# CPedIntelligence (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4ABEA0 CPedIntelligence::GetPedFOVRange  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`, result in ST0: `fld this+0xBC (m_fHearingRange); fcomp this+0xC0 (m_fSeeingRange); test ah,0x41; jne` (0x4ABEA0-0x4ABEB1):
  returns m_fHearingRange iff hearing > seeing strictly; otherwise (equal, less, NaN) returns m_fSeeingRange (0x4ABEBA).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x602320 CPedIntelligence::RemoveAllInterestingEntities  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`: for the 3 slots at this+0x288 (+0x288, +0x28C, +0x290): if non-null `CEntity::CleanUpOldReference(slot, &slot)` 0x571A00; slot = 0 (always) (0x602330-0x602346).
