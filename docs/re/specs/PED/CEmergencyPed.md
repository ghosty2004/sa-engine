# CEmergencyPed (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x5DE3E0 CEmergencyPed::vfunc_0 = scalar deleting destructor  (catalogue: specified/REA, port level 17)
- Convention: virtual slot 0, `__thiscall CEmergencyPed*(uint8 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: call 0x5DE3B0 (catalogue FUN_005DE3B0 = CEmergencyPed's non-deleting destructor); if `flags & 1`:
  `CPed::operator delete(this)` 0x5E4760.

## 0x5DE400 CEmergencyPed::ProcessControl  (catalogue: specified/REA, port level 32)
- Convention: virtual `__thiscall void()`, plain `ret` / tail jump. No static callers (vtable only).
- Behaviour:
  1. `CPed::ProcessControl()` 0x5E8CD0.
  2. if `m_nFlags (+0x1C) & 0x40` (bit 6, m_bWasPostponed) return; if `m_nPedState (+0x530) == 55` (DEAD) return (0x5DE408-0x5DE419).
  3. `CWeapon::Update(&m_aWeapons[(int8)m_nActiveWeaponSlot], nullptr)` (0x73DB40, thiscall, **owner argument 0**) (0x5DE41B-0x5DE42E).
  4. if `m_nPedState == 54` (DIE) return.
  5. `CWeapon::Update(&activeWeapon, nullptr)` **a second time** (0x5DE43C-0x5DE44F; slot re-read).
  6. if `m_nPedType (+0x598)` is 18 (MEDIC) or 19 (FIREMAN): tail-jump `CPed::IsPedInControl()` 0x5E3960 (result ignored; the medic /
     fireman AI of earlier games is compiled out). Otherwise return.
- gta-reversed: CEmergencyPed::ProcessControl is not reversed there; port from the bytes (keep both Update calls and the null owner).

## 0x5DE3B0 CEmergencyPed::~CEmergencyPed  (catalogue: specified/REA, port level 16)
- `__thiscall` (ECX = this), body: `this->vtable = 0x86C200`, then tail-jump to CPed::~CPed 0x5E8620 (ECX unchanged). Caller:
  CEmergencyPed::`scalar deleting destructor' 0x5DE3E0.
