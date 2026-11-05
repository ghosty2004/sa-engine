# CWeaponInfo (WEAPON) - full specs

CWeaponInfo is 0x70 bytes (`aWeaponInfo` 0xC8AAB8, 80 entries); +0x6C m_nAimOffsetIndex (u16 in gr, read with movsx).
`ms_WeaponAimOffsets` 0xC8A8A8, stride 0x18 (AimX, AimZ, DuckX, DuckZ f32; RLoadA +0x10, RLoadB +0x12, CrouchRLoadA +0x14,
CrouchRLoadB +0x16, u16).

## 0x61DB30 CWeaponInfo::sub_61DB30 (proposed: GetBaseComboByName)  (catalogue: unknown/REA, port level 0)
- Evidence for the name: only caller is CWeaponInfo::LoadWeaponData 0x5BE670; the strings and return values are exactly
  gr's NOTSA helper `GetBaseComboByName` (WeaponInfo.cpp:141) mapping to eMeleeCombo (MELEE_COMBO_UNARMED_1 = 4).
- Convention: `__cdecl int32 (const char* name [esp+4])`, plain `ret`. Preserves ESI/EDI. EDX is zeroed (dead).
- Behaviour: sequence of `repe cmpsb` compares of `name` against NUL-terminated .rdata strings, count = strlen+1
  (i.e. exact string equality incl. terminator; stops at the first mismatch), first match wins:
  "UNARMED" 0x86D6F8 (8) -> 4; "BBALLBAT" 0x86D6EC (9) -> 8; "KNIFE" 0x86D6E4 (6) -> 9; "GOLFCLUB" 0x86D6D8 (9) -> 0xA;
  "SWORD" 0x86D6D0 (6) -> 0xB; "CHAINSAW" 0x86D6C4 (9) -> 0xC; "DILDO" 0x86D6BC (6) -> 0xD;
  "FLOWERS" 0x86D6B4 (8): 0x61DBFA-0x61DC05 `eax = ((ne ? 1 : 0) - 1) & 0xA) + 4` -> 0xE when equal, 4 otherwise.
  So any unknown name returns 4 (UNARMED_1).
- gta-reversed: NOTSA `GetBaseComboByName` (string_view find) gives the same mapping and default. Equivalent.
- Open questions: none.

## 0x73A280 CWeaponInfo::sub_73A280 (proposed: GetReloadSoundTimeA(bool crouching))  (catalogue: unknown/REA, port level 0)
- Evidence: reads `CWeaponInfo::ms_WeaponAimOffsets` (0xC8A8A8, stride 0x18, gr `tAnimAimOffsets`) indexed by
  `this->m_nAimOffsetIndex` (+0x6C); fields +0x10 RLoadA (0xC8A8B8) / +0x14 CrouchRLoadA (0xC8A8BC). gr's CWeapon::Update
  (Weapon.cpp:1034) uses `owner->bIsDucking ? ao->CrouchRLoadA : ao->RLoadA`; only caller is CWeapon::Update 0x73DB40.
- Convention: `__thiscall` (ECX = CWeaponInfo*), arg `bool crouching` [esp+4] (only AL tested), `ret 4`. Returns AX
  (u16); the upper 16 bits of EAX are garbage (`movsx eax,[ecx+0x6C]` then `lea eax,[eax*3]`), callers must use AX only.
- Behaviour: `i = (int32)(int16)this->m_nAimOffsetIndex` (movsx, signed!). crouching ? `ms_WeaponAimOffsets[i].CrouchRLoadA`
  (the load sits in a .text stub at 0x4039E1 reached by `jmp`, returns via `ret 4` at 0x73A297) : `ms_WeaponAimOffsets[i].RLoadA`
  (0x73A29D).
- gta-reversed: inlined in CWeapon::Update; same values (gr uses uint16 index; identical for index < 0x8000).
- Open questions: none.

## 0x73A2B0 CWeaponInfo::sub_73A2B0 (proposed: GetReloadSoundTimeB(bool crouching))  (catalogue: unknown/REA, port level 0)
- Same convention and evidence as 0x73A280 (`__thiscall`, `bool` [esp+4], `ret 4`, AX result, signed movsx of +0x6C).
- Behaviour: crouching ? `ms_WeaponAimOffsets[i].CrouchRLoadB` (0xC8A8BE, +0x16) : `.RLoadB` (0xC8A8BA, +0x12).
- gta-reversed: inlined in CWeapon::Update (Weapon.cpp:1035). Open questions: none.
