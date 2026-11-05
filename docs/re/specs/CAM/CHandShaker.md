# CHandShaker (CAM) - full specs

Layout (gr `HandShaker.h`, size 0x94): +0x00 `m_ang` CVector, +0x0C `m_lim` CVector, +0x18 `m_motion` CVector,
+0x24 `m_vel` CVector, +0x30 `m_slow` CVector, +0x3C `m_resultMat` CMatrix (0x48 bytes; its `m_pAttachMatrix` is
at +0x3C+0x40 = +0x7C and `m_bOwnsAttachedMatrix` at +0x80), +0x84 `m_scaleReactionMin` f32, +0x88
`m_scaleReactionMax` f32, +0x8C `m_twitchFreq` i32, +0x90 `m_twitchVel` f32. Global array `gHandShaker` at 0xB6ECA0
(6 entries, stride 0x94), constructed by `__init_gHandShaker` 0x84D6D0 and destroyed by `__dtor_gHandShaker` 0x8563B0.

## 0x509810 CHandShaker::~CHandShaker  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall void (CHandShaker* this = ECX)`.
- Body: `add ecx, 0x3c` (0x509810); `jmp 0x59ACD0` (CMatrix::~CMatrix, tail call with ECX = &this->m_resultMat).
- Callers: referenced from `__init_gHandShaker` 0x84D6D0 (as the per-element destructor for the array helper) and
  `__dtor_gHandShaker` 0x8563B0. Callees: CMatrix::~CMatrix 0x59ACD0 (via `orig!`).
- Behaviour: destroy `m_resultMat` (CMatrix dtor frees an owned attach matrix). Nothing else.
- gta-reversed: `~CHandShaker() = default` - equivalent.

## 0x517740 CHandShaker::CHandShaker  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall CHandShaker* (CHandShaker* this = ECX)`, plain `ret`, returns `this` in EAX
  (0x517753). ESI saved/restored.
- Callers: `__init_gHandShaker` 0x84D6D0. Callees: CHandShaker::SetDefaults 0x517330 (thiscall, ECX unchanged).
- Behaviour (order): `*(u32*)(this+0x7C) = 0` (0x517745, m_resultMat.m_pAttachMatrix = nullptr);
  `*(u32*)(this+0x80) = 0` (0x517748, m_resultMat.m_bOwnsAttachedMatrix = false, written as a full dword);
  `SetDefaults()` (0x51774E); `return this`. This is the inlined default `CMatrix()` constructor followed by
  SetDefaults; m_ang/m_vel are set by SetDefaults -> Reset (0x517330 tail-jumps to Reset 0x50D860).
- gta-reversed: `CHandShaker() { SetDefaults(); }` - equivalent (CMatrix default ctor zeroes the same two fields).
