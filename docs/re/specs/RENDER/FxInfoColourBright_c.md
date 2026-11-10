# FxInfoColourBright_c (RENDER) - full specs

## 0x4A67D0 FxInfoColourBright_c::FxInfoColourBright_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall FxInfoColourBright_c* (this = ECX)`, plain `ret`, returns `this` in EAX. SEH frame (state 0 after
  the base part, 1 after the member ctor) only for unwinding (~FxInfo_c 0x4A4940, ~FxInterpInfoU255_c 0x4A87F0).
- Callers: FxInfoManager_c::AddFxInfo 0x4A7B00 (type 0x4400, after `GetMem(0x18, 4)`). Callees:
  FxInterpInfoU255_c::FxInterpInfoU255_c 0x4A87D0 (thiscall on this+8), FxMemoryPool_c::GetMem 0x4A9CA0 (thiscall,
  ECX = &g_fxMan.m_Pool 0xA9AF2C, args (size, align)).
- Layout (0x18): +0 vtable, +4 m_nType (u16), +6 m_bTimeModeParticle, +8 m_InterpInfo (FxInterpInfoU255_c: +8 vtable,
  +0x0C looped, +0x0D numKeys, +0x0E count, +0x10 times*, +0x14 keys**).
- Behaviour (in order):
  1. `this->vtable = 0x85A944` (0x4A67F8; the FxInfo_c base ctor is inlined as this single store, +4..+7 untouched yet).
  2. `FxInterpInfoU255_c::ctor(this+8)` (0x4A67FE): writes +8 vtable 0x85A978, +0x0C = 0, +0x10 = 0, +0x14 = 0 (it does not
     touch +0x0D/+0x0E/+0x0F).
  3. `*(u16*)(this+4) = 0x4400` (FX_INFO_COLOURBRIGHT_DATA, 0x4A6811); `*(u8*)(this+0x0E) = 5` (byte store only, 0x4A6817).
  4. `this->+0x14 = g_fxMan.m_Pool.GetMem(0x14, 4)` (5 key-array pointers, 0x4A681B-0x4A6824). Return this.
- Edge cases: +6 (m_bTimeModeParticle), +0x0D and +0x0F are left as the pool memory had them (FxInfoManager_c::Load sets +6
  later). Readers of the count use `byte [+6 of the interp]` (e.g. FxInterpInfoU255_c::Load 0x5C18FA, 0x5C1AEA `movsx byte`),
  so only the low byte matters. The GetMem result is not checked.
- gta-reversed: Blueprints/FxInfoColourBright.cpp:6 (`m_nType = ...; m_InterpInfo.Allocate(5)`); gr's Allocate stores the
  count as int16 (writes +0x0F too) - harmless. Open questions: none.
