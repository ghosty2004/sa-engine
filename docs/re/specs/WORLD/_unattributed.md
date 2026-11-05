# WORLD functions whose class file is outside this writer's set

Each has an evidence-backed proposed name; the class file it belongs to (CPool template, CRT math,
CTagManager) is not a WORLD class file assigned here.

## 0x49c3b0 sub_49c3b0 (proposed: `CPool<CStuntJump>::GetAt(int32 idx)`)  (catalogue: unknown/REA, port level 0)
- Evidence: `this` layout = CPool header (`+0 m_pObjects`, `+4 m_byteMap`); stride 0x44 = sizeof(CStuntJump);
  only caller `CStuntJumpManager::Update` 0x49c490 (which iterates `CStuntJumpManager::mp_poolStuntJumps`).
- Convention / signature: `__thiscall T* (this = ECX: CPool*, int32 idx [esp+4])`, return EAX, `ret 4`.
  Stub `jmp 0x156acc0`.
- Behaviour: `if ((i8) m_byteMap[idx] < 0) return NULL` (slot free, bit 0x80); else
  `return m_pObjects + idx * 0x44`. No bounds check.
- Open questions: none.

## 0x538ad0 sub_538ad0 (proposed: MSVC inline `ceilf(float)` = `(float)ceil((double)x)`)  (catalogue: unknown/REA, port level 0)
- Evidence: converts the f32 argument to f64 and calls CRT 0x823820, which is `ceil` (x87 path 0x82ba31
  sets the control word 0x1b3f = round toward +inf; SSE2 path selected by `[0xc9c400]`). Caller
  `CObject::SpecialEntityCalcCollisionSteps` 0x5a02e0; gta-reversed uses `std::ceil(float)` there
  (Object.cpp:453/465).
- Convention / signature: `__cdecl` (float x [esp+4]), result in **ST0** (the f64 returned by `ceil`,
  not rounded to f32 by this function), plain `ret`.
- Behaviour: `fld x; fstp qword [esp]` (exact widening), `call 0x823820`, `add esp,8`, return.
  Result = `ceil((f64)x)` exactly (always representable as f32 for f32 inputs).
- Open questions: none.

## 0x49cd50 sub_49cd50 (proposed: `CTagManager::ResetAlpha(const CEntity&)`)  (catalogue: unknown/REA, port level 1)
- Evidence: reads `CTagManager::ms_numTags` 0xa9ad70 and `CTagManager::ms_tagDesc` 0xa9a8c0 (entries of 8
  bytes: `Entity` +0, `Alpha` u8 +4), calls `CVisibilityPlugins::SetUserValue` on the entity's atomic;
  only caller `CEntity::CreateRwObject` 0x533d30, where gta-reversed calls `CTagManager::ResetAlpha`
  (Entity.cpp, `CTagManager::IsTag` branch). gr declares ResetAlpha but does not hook 0x49cd50.
- Convention / signature: `__cdecl void (CEntity* e [esp+4])`, plain `ret`.
- Callees: `CVisibilityPlugins::SetUserValue` 0x732380 (`__cdecl (RpAtomic*, u16)`, caller pops 8).
- Behaviour:
  1. `atomic = e->m_pRwObject (+0x18)`; if NULL return.
  2. `n = ms_numTags`. Search **backwards**: `for (i = n; i != 0; ) { i--; if (ms_tagDesc[i].Entity == e)
     goto found; }` (reads `dword [0xa9a8b8 + (i+1)*8]` = `ms_tagDesc[i].Entity`). Last matching entry
     wins.
  3. found: `SetUserValue(atomic, (u16) ms_tagDesc[i].Alpha)`.
  4. not found (or `n == 0`): `desc = NULL`, reads byte at address 4 → access violation (original bug,
     no guard).
- The pushed 32-bit value has its low 16 bits = alpha (zero-extended byte) and upper 16 bits = leftover
  EAX bits (0 on the not-found path, `0x00a9` from the table address on the found path); the callee
  takes `u16`, so only the low half matters.
- Note: `n` is used as a signed int32; a negative `ms_numTags` would loop through negative indices
  until i wraps to 0 (does not happen in practice).
- gta-reversed: `CTagManager::ResetAlpha` (TagManager.cpp:169) uses `FindTagDesc`, which searches
  **forwards** (first match) and, with `notsa::bugfixes::GenericCrashing`, returns early on NULL. Both
  differ from the bytes (search direction only matters with duplicate entries). Port the bytes.
- Open questions: none.
