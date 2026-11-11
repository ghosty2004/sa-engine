# POP: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x406e80 CPopulation ped-group model lookup (m_PedGroups[GetPedGroupId(group, zone)][slot]) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl i32 (i32 group [esp+4], i32 slot [esp+8], i32 zone [esp+0xc])`; body 0x1560e20.
- `id = m_TranslationArray[group*3 + zone]` (dword table 0x8d2540, index computed as `group + (zone + group*2)`); `return (u16)m_PedGroups[id*21 + slot]` (word table 0xc0f358, zero-extended).
- gta-reversed: composition of `CPopulation::GetPedGroupId` (Population.h:218) and the m_PedGroups lookup; no bounds checks.

## 0x420920 CNodeAddress::operator== (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall bool (const CNodeAddress& o)`, `ret 4`; body 0x1565640: returns EAX = 1 if `word[+0] == o.word[+0] && word[+2] == o.word[+2]` (area then node), else 0.

## 0x420950 CNodeAddress::operator!= (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall bool (const CNodeAddress& o)`, `ret 4`; body 0x156ab40: EAX = 0 if both words equal, else 1.

## 0x420980 CNodeAddress::operator< (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall bool (const CNodeAddress& o)`, `ret 4`; body 0x156e7e0: `if ((u16)area < (u16)o.area) return 1; if (area != o.area) return 0; return (u16)node < (u16)o.node` (unsigned `jb`, so -1 sorts last).

## 0x420a60 CompressedVector2D to CVector (x/y * 0.125f, z = 0) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CVector* (CVector* out)`, `ret 4` (this = 2 x i16). `out.z = 0` (integer store, relocated at 0x401492); `y = (float)(i16)this[+2] * 0.125f [0x858c48]`, `x = (float)(i16)this[+0] * 0.125f` (y computed first), stores x then y. EAX = out.
- gta-reversed: `CompressedVector2D::Uncompress`-style (x/8). Catalogue extent note: 0x401492-0x40149f belongs here.

## 0x421120 CAutoPilot speed-step increment (i8 +0x50, clamp 120) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`; body 0x1563560: `v = (i8)this[+0x50] + 1; if (v >= 0x78) v = 0x78; this[+0x50] = (u8)v`.
- Open question: field name at +0x50 (0x1563560).

## 0x4241a0 CNodeAddress::CNodeAddress (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CNodeAddress* ()`: `word[+0] = 0xffff` (area = -1; node word untouched), EAX = this.

## 0x4241b0 CNodeAddress::CNodeAddress (proposed)  (catalogue: unknown/REA, port level 0)
- Same body as 0x4241a0.
