# AI static initialisers / atexit destructors, group J4 - RE-SPEC-AI-2

## 0x8567C0 __dtor_g_c09870  (catalogue: specified/REA, port level 1)
- atexit destructor, `__cdecl void ()` for a 3-dword container at 0xC09870 (pointer, count, capacity): if `[0xC09870] != 0` `free([0xC09870])` 0x8214BD (cdecl); then `[0xC09870] = [0xC09874] = [0xC09878] = 0` (0x8567D2-0x8567E6).

## 0x856830 __dtor_CPedGroupDefaultTaskAllocators__ms_ChatAllocator  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC09914` and tail-jump to the (unnamed) allocator dtor 0x5F64F0 (0x856835). The target is `this->vtbl = 0x86C754` then `ret` (0x5F64F0): only the vtable is reset.

## 0x856840 __dtor_CPedGroupDefaultTaskAllocators__ms_RandomAllocator  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC09918` and tail-jump to the (unnamed) allocator dtor 0x5F6520 (0x856845). The target is `this->vtbl = 0x86C754` then `ret` (0x5F6520): only the vtable is reset.

## 0x856850 __dtor_CPedGroupDefaultTaskAllocators__ms_SitInLeaderCarAllocator  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC0991C` and tail-jump to the (unnamed) allocator dtor 0x5F6550 (0x856855). The target is `this->vtbl = 0x86C754` then `ret` (0x5F6550): only the vtable is reset.

## 0x856860 __dtor_CPedGroups__ms_groups  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `eh vector destructor iterator` 0x821E02(ptr = 0xC09920, elemSize = 0x2D4, count = 8, dtor = CPedGroup::Destructor 0x5FC190) (pushes 0x856860-0x85686C): destroys the 8 elements in reverse order.

## 0x8568B0 __dtor_CPedScriptedTaskRecord__ms_scriptedTasks  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `eh vector destructor iterator` 0x821E02(ptr = 0xC0B1E8, elemSize = 0x14, count = 128, dtor = (missed by ghidra: data-code-immediate) 0x6082E0) (pushes 0x8568B0-0x8568BC): destroys the 128 elements in reverse order.

## 0x856900 __dtor_CScriptedBrainTaskStore__ms_entries  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `eh vector destructor iterator` 0x821E02(ptr = 0xC188F0, elemSize = 0x10, count = 48, dtor = (missed by ghidra: data-code-immediate) 0x62EC50) (pushes 0x856900-0x856909): destroys the 48 elements in reverse order.

## 0x856920 __dtor_CTaskSequences__ms_taskSequence  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `eh vector destructor iterator` 0x821E02(ptr = 0xC178F0, elemSize = 0x40, count = 64, dtor = CTaskComplexSequence 0x6389F0) (pushes 0x856920-0x856929): destroys the 64 elements in reverse order.

## 0x856960 __dtor_ms_ClimbColModel  (catalogue: specified/REA, port level 5)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC19518` and tail-jump to CColModel 0x40F700 (0x856965).

## 0x856970 __dtor_ms_StandUpColModel  (catalogue: specified/REA, port level 5)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC19548` and tail-jump to CColModel 0x40F700 (0x856975).

## 0x856980 __dtor_ms_VaultColModel  (catalogue: specified/REA, port level 5)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC19578` and tail-jump to CColModel 0x40F700 (0x856985).

## 0x856990 __dtor_ms_FindEdgeColModel  (catalogue: specified/REA, port level 5)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC195A8` and tail-jump to CColModel 0x40F700 (0x856995).

## 0x8569C0 __dtor_CCover__m_ListOfProcessedBuildings  (catalogue: specified/REA, port level 1)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC1A2B8` and tail-jump to CPtrListDoubleLink::Flush 0x552470 (0x8569C5).

## 0x8569E0 __dtor_g_c1a45c  (catalogue: specified/REA, port level 0)
- Empty atexit destructor: `ret` (0x8569E0); the object it belongs to has a trivial destructor.

## 0x8569F0 __dtor_g_c1a4dc  (catalogue: specified/REA, port level 0)
- Empty atexit destructor: `ret` (0x8569F0); the object it belongs to has a trivial destructor.
