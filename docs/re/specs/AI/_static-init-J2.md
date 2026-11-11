# AI static initialisers / atexit destructors, group J2 - RE-SPEC-AI-2

## 0x856800 __dtor_CPedGroupDefaultTaskAllocators__ms_FollowAnyMeansAllocator  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC09908` and tail-jump to the (unnamed) allocator dtor 0x5F6460 (0x856805). The target is `this->vtbl = 0x86C754` then `ret` (0x5F6460): only the vtable is reset.

## 0x8569D0 __dtor_g_c1a43c  (catalogue: specified/REA, port level 0)
- Empty atexit destructor: `ret` (0x8569D0); the object it belongs to has a trivial destructor.
