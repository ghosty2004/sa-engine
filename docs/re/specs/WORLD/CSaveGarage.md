# CSaveGarage (WORLD) - full specs, writer E (RE-SPEC-1)

Layouts (gta-reversed `Garage.h`, VALIDATE_SIZE(CGarage, 0xD8), VALIDATE_SIZE(CSaveGarage, 0x50); offsets
confirmed by the bytes below):
- `CGarage`: +0x00 CVector m_vPosn, +0x0C CVector2D m_vDirectionA, +0x14 CVector2D m_vDirectionB,
  +0x1C f32 m_fTopZ, +0x20 f32 m_fWidth, +0x24 f32 m_fHeight, +0x28 m_fLeftCoord, +0x2C m_fRightCoord,
  +0x30 m_fFrontCoord, +0x34 m_fBackCoord, +0x38 m_fDoorPosition, +0x3C u32 m_nTimeToOpen,
  +0x40 CVehicle* m_pTargetCar, +0x44 char m_anName[8], +0x4C u8 m_nType, +0x4D u8 m_nDoorState,
  +0x4E u8 m_nFlags, +0x4F u8 m_nOriginalType, +0x50 CAEDoorAudioEntity m_GarageAudio (0x88 bytes).
- `CSaveGarage`: +0x00 u8 m_nType, +0x01 u8 m_nDoorState, +0x02 u8 m_nFlags, +0x03 (pad, never written),
  +0x04 CVector m_vPosn, +0x10 dirA, +0x18 dirB, +0x20 m_fTopZ, +0x24 m_fWidth, +0x28 m_fHeight,
  +0x2C Left, +0x30 Right, +0x34 Front, +0x38 Back, +0x3C m_fDoorPosition, +0x40 u32 m_nTimeToOpen,
  +0x44 char m_anName[8], +0x4C u8 m_nOriginalType, +0x4D..+0x4F pad (never written).

## 0x5D3020 CSaveGarage::CopyGarageIntoSaveGarage  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (CSaveGarage* this = ECX, const CGarage* g = [esp+4])`, `ret 4`. No return
  value (EAX = g, AL later overwritten with g->m_nOriginalType; callers ignore it).
- Callers: 1 (CGarages::Save 0x5D3160). Callees: none (the string copy is inlined).
- Globals: none.
- Behaviour, in this order:
  1. `this[0x00] = g[0x4C]` (type), `this[0x01] = g[0x4D]` (door state), `this[0x02] = g[0x4E]` (flags), byte
     copies.
  2. 15 dword copies: `this[0x04 + 4*i] = g[0x00 + 4*i]` for i = 0..14, i.e. CGarage +0x00..+0x3B (posn,
     dirA, dirB, topZ, width, height, left, right, front, back, doorPos) and +0x3C m_nTimeToOpen to
     CSaveGarage +0x04..+0x40. Raw 32-bit moves (no x87, NaN payloads preserved).
  3. Unbounded `strcpy(this->m_anName (+0x44), g->m_anName (+0x44))`: byte loop that copies up to and
     including the first NUL. Bytes of the destination after the NUL are left untouched.
  4. `this[0x4C] = g[0x4F]` (m_nOriginalType).
- Edge cases: the padding bytes +0x03 and +0x4D..+0x4F and name bytes after the terminator are not written
  (their content depends on the caller's buffer). If the garage name has no NUL in its 8 bytes the copy runs
  on into g->m_nType/m_nDoorState/... until a zero byte (writing past this+0x4B); m_nDoorState 0 or
  m_nType 0 normally stops it. m_pTargetCar (+0x40) and m_GarageAudio are not saved.
- gta-reversed: Garage.cpp:226, reversed but not hooked by itself (called from hooked Save). Same fields; gr
  uses `strcpy_s(m_anName, g.m_anName)` (bounded, aborts on overflow) and copies m_nOriginalType before the
  name. Only the unterminated-name case differs (keep-original: plain strcpy).
- Open questions: none.

## 0x5D30C0 CSaveGarage::CopyGarageOutOfSaveGarage  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (const CSaveGarage* this = ECX, CGarage* g = [esp+4])`, `ret 4`. No return
  value (EAX = g on exit).
- Callers: 1 (CGarages::Load 0x5D3270). Callees: none.
- Globals: none.
- Behaviour, in this order:
  1. `g[0x4C] = this[0x00]`, `g[0x4D] = this[0x01]`, `g[0x4E] = this[0x02]` (bytes).
  2. 15 dword copies `g[0x00 + 4*i] = this[0x04 + 4*i]`, i = 0..14 (CGarage +0x00..+0x3F).
  3. `g->m_pTargetCar (+0x40) = nullptr` (plain store, no CEntity::CleanUpOldReference on the old value).
  4. Unbounded strcpy of the name, this+0x44 -> g+0x44 (byte loop including the NUL).
  5. `g[0x4F] = this[0x4C]` (m_nOriginalType).
- Edge cases: an old m_pTargetCar registered reference is abandoned (not unregistered). Name without NUL
  runs into this+0x4C.. (m_nOriginalType and padding) and writes past g+0x4B into g->m_nType.. until a
  zero byte; step 1 values may then be overwritten (the original order is: bytes first, name later).
  m_GarageAudio is untouched.
- gta-reversed: Garage.cpp:246, same fields and the explicit `m_pTargetCar = nullptr`; uses strcpy_s
  (differs only for an unterminated name). Keep-original.
- Open questions: none.
