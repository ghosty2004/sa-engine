# CControllerConfigManager (INPUT) - RE-SPEC-FINAL group A

## 0x719360 CControllerConfigManager::sub_719360  (catalogue: unknown/REA, port level 0)
- `__cdecl uint8 (uint8 c [esp+4])`, plain `ret`, AL: if `c < 0x80` return c; else return the byte at `0x8D5AD8 + 2*c` (low byte of a 16-bit
  per-char table entry; only the AL part of EAX is defined). Caller GetControllerSettingTextKeyBoard 0x52FE10 (2). Proposed: `GetGxtCharForKey`
  (high-character translation). Open questions: name of table 0x8D5AD8.
