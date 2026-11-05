# CClumpModelInfo (STREAM) - full specs, writer A (RE-SPEC-1)

Vtable 0x85BD30 (slot 0 = 0x4C62B0). Destructor 0x4C5750 (not in this assignment): `mov [ecx], 0x85BB9C; ret`
(same reduced trivial-destructor chain as the other model-info classes); it is also the element destructor of
`CModelInfo::ms_clumpModelInfoStore` (0xB1E958, element 0x24, count 0x5C).

## 0x4C62B0 CClumpModelInfo::`scalar deleting destructor' (catalogue: CClumpModelInfo::vfunc_0, specified/REA, port level 1)
- Convention: `__thiscall CClumpModelInfo* (this, uint32 flags)`; flags at [esp+4]; `ret 4`; EAX = this.
- Callees: `orig!(0x4C5750 CClumpModelInfo::~CClumpModelInfo)`, `orig!(0x8214BD operator delete)`.
- Behaviour: ~CClumpModelInfo(this); if `(u8)flags & 1`: operator delete(this); return this.
- Name evidence: slot-0 pattern (see CBaseModelInfo.md). Open questions: none.
