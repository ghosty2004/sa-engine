# SAVE unattributed / free functions (group H1)

## 0x69F910 TextCopy  (catalogue: specified/REA, port level 0)
- `__cdecl void (GxtChar* dst [esp+4], const GxtChar* src [esp+8])`, plain `ret`.
- If `*src == 0`: `*dst = 0`, return (0x69F92D-0x69F931; EAX = dst). Else copy bytes until the source NUL and write
  the NUL (0x69F920-0x69F92A; EAX low byte = 0, upper bytes = junk from the last `mov al`). No bounds.
- Callers: CSimpleVariablesSaveStructure::Construct 0x5D1B80 (2 sites). gta-reversed GxtChar.cpp:87 matches (void).
  Open questions: none.
