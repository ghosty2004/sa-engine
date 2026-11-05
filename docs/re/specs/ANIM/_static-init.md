# _static-init (ANIM) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x84BFD0 __init_CAnimManager__ms_aAnimations  (catalogue: specified/REA, port level 0)
- CRT static initialiser, `__cdecl void()`. Calls `eh_vector_constructor_iterator` 0x821D30 (`__stdcall`, 5 args, callee
  cleans) with (ptr = `CAnimManager::ms_aAnimations` 0xB4EA40, size = 0x18, count = 0x9C4 = 2500,
  ctor = CAnimBlendHierarchy::Constructor 0x4CF270, dtor = CAnimBlendHierarchy::Destructor 0x4CF970) (0x84BFD0-0x84BFE6),
  then `_atexit(__dtor_CAnimManager__ms_aAnimations 0x8562C0)` (0x84BFEB-0x84BFF5, `pop ecx` cleans its arg).
- Port: construct 2500 CAnimBlendHierarchy objects in order 0..2499; register the destructor for process exit.

## 0x84C000 __init_CAnimManager__ms_aAnimBlocks  (catalogue: specified/REA, port level 0)
- CRT static initialiser, `__cdecl void()`, no atexit. For i = 0..179 (`ecx = 0xB4` counted down, 0x84C005-0x84C01A),
  entry = 0xB5D4A0 + i*0x20 (`CAnimManager::ms_aAnimBlocks`, CAnimBlock): `Name[0] (+0x00, u8) = 0`; `RefCnt (+0x12, u16) = 0`.
  Nothing else is written (.bss is zero anyway). Inlined CAnimBlock default constructor.

## 0x850040 __init_CCutsceneMgr__ms_cutsceneAssociations  (catalogue: specified/REA, port level 1)
- CRT static initialiser. `CAnimBlendAssocGroup::Constructor` 0x4CDE70 (thiscall, ECX = 0xBC4020
  `CCutsceneMgr::ms_cutsceneAssociations`) (0x850040-0x850045); `_atexit(__dtor_CCutsceneMgr__ms_cutsceneAssociations 0x8566F0)`.

## 0x851A70 __init_g_boneNodeMan  (catalogue: specified/REA, port level 2)
- CRT static initialiser. `BoneNodeManager_c::Constructor` 0x617330 (thiscall, ECX = `g_boneNodeMan` 0xC10820);
  `_atexit(__dtor_g_boneNodeMan 0x8568D0)` (0x851A70-0x851A84).

## 0x851B50 __init_g_ikChainMan  (catalogue: specified/REA, port level 2)
- CRT static initialiser. `IKChainManager_c::IKChainManager_c` 0x617FC0 (thiscall, ECX = `g_ikChainMan` 0xC15448);
  `_atexit(__dtor_g_ikChainMan 0x8568E0)` (0x851B50-0x851B64).

## 0x853E20 __init_CVehicleAnimGroupData__m_vehicleAnimGroups  (catalogue: specified/REA, port level 0)
- CRT static initialiser. `eh_vector_constructor_iterator` 0x821D30 (stdcall) with (ptr = 0xC1CDC0
  `CVehicleAnimGroupData::m_vehicleAnimGroups`, size = 0x94, count = 0x1E = 30, ctor = CVehicleAnimGroup::CVehicleAnimGroup
  0x5BD420, dtor = 0x5B9E30) (0x853E20-0x853E36); then `_atexit(0x856A60)`.
- 0x5B9E30 (the element "destructor") is a bare `ret`: CVehicleAnimGroup has a trivial destructor.

## 0x8562C0 __dtor_CAnimManager__ms_aAnimations  (catalogue: specified/REA, port level 0)
- atexit callback, `__cdecl void()`. `eh_vector_destructor_iterator` 0x821E02 (stdcall, 4 args) with (ptr 0xB4EA40, size 0x18,
  count 0x9C4, dtor CAnimBlendHierarchy::Destructor 0x4CF970) (0x8562C0-0x8562D1): destroys the 2500 hierarchies in reverse order
  (the CRT iterator runs from the last element down).

## 0x8566F0 __dtor_CCutsceneMgr__ms_cutsceneAssociations  (catalogue: specified/REA, port level 4)
- atexit callback: tail-jump to `CAnimBlendAssocGroup::Destructor` 0x4CE1D0 with ECX = 0xBC4020 (0x8566F0-0x8566F5).

## 0x8568D0 __dtor_g_boneNodeMan  (catalogue: specified/REA, port level 2)
- atexit callback: tail-jump to `BoneNodeManager_c::Destructor` 0x617390 with ECX = 0xC10820 (0x8568D0-0x8568D5).

## 0x8568E0 __dtor_g_ikChainMan  (catalogue: specified/REA, port level 2)
- atexit callback: tail-jump to 0x618030 with ECX = 0xC15448 (0x8568E0-0x8568E5). 0x618030 (catalogue FUN_00618030) is
  `IKChainManager_c::~IKChainManager_c`: SEH frame, destroys the TList_c at +0xB0C then +0xB00 (0x4A8DE0) and the 32 IKChain_c
  (gr `IKChainManager.h:15` says "0x8568E0" for the destructor; that is this thunk, the real body is 0x618030).

## 0x856A60 __dtor_CVehicleAnimGroupData__m_vehicleAnimGroups  (catalogue: specified/REA, port level 0)
- atexit callback. `eh_vector_destructor_iterator` 0x821E02 with (0xC1CDC0, 0x94, 0x1E, dtor 0x5B9E30 = bare `ret`)
  (0x856A60-0x856A71). Observable effect: none. A port may treat it as a no-op.
