# CFontChar (HUD) - full specs

## 0x718E50 CFontChar::Set  (catalogue: specified/REA, port level 0)
- `__thiscall void Set(const CFontChar* src [esp+4])` (ECX = this), `ret 4`; EAX = this on return (0x718E50 `mov eax, ecx`).
- Caller: CFont::RenderFontBuffer 0x719840 (2 sites). No callees.
- Copies field by field (0x718E56-0x718EBF): dword +0x00 (CLetter, DLetter and the 2 padding bytes), dwords +0x04, +0x08
  (Pos), +0x0C (Width), +0x10 (Height), bytes +0x14..+0x17 (Color r,g,b,a), dwords +0x18 (Wrap), +0x1C (Slope),
  +0x20, +0x24 (SlopeRef), bytes +0x28 (ContainImages), +0x29 (FontStyle), +0x2A (PropOn), word +0x2C (FontTexture),
  byte +0x2E (Outline). Padding bytes +0x2B and +0x2F are NOT copied (destination keeps its old values).
- gta-reversed: FontSpecifiers.h:65 `*this = *setup` (struct copy; also copies the two padding bytes - unobservable unless
  someone reads padding). Open questions: none.
