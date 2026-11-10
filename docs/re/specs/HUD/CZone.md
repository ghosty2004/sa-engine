# CZone (HUD) - full specs

## 0x5720C0 CZone::GetTranslatedName  (catalogue: specified/REA, port level 4)
- `__thiscall const GxtChar* () const` (ECX = CZone*), plain `ret`, EAX = result of the tail call.
- `return TheText.Get(&this->m_TextLabel)`: `push (this + 8)` (0x5720C0 `add ecx, 8`), `ecx = &TheText 0xC1B340`,
  `call CText::Get 0x6A0050` (thiscall, 1 arg). CZone +8 = m_TextLabel[8] (gr Zone.h).
- Callers: CPlaceName::GetForMap 0x571D90, CPlaceName::Process 0x571F20. gta-reversed Zone.cpp:7 matches. Open questions: none.
