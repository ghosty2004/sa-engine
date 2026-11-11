# AI static initialisers / atexit destructors, group J3 - RE-SPEC-AI-2

## 0x8561F0 __dtor_CInformFriendsEventQueue__ms_informFriendsEvents  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `eh vector destructor iterator` 0x821E02(ptr = 0xA9AFB8, elemSize = 0xC, count = 8, dtor = FUN_004b2950 0x4B2950) (pushes 0x8561F0-0x8561F9): destroys the 8 elements in reverse order.

## 0x856210 __dtor_CInformGroupEventQueue__ms_informGroupEvents  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `eh vector destructor iterator` 0x821E02(ptr = 0xA9B018, elemSize = 0x10, count = 8, dtor = CInformGroupEvent::Destructor 0x4B29A0) (pushes 0x856210-0x856219): destroys the 8 elements in reverse order.

## 0x856810 __dtor_CPedGroupDefaultTaskAllocators__ms_FollowLimitedAllocator  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC0990C` and tail-jump to the (unnamed) allocator dtor 0x5F6490 (0x856815). The target is `this->vtbl = 0x86C754` then `ret` (0x5F6490): only the vtable is reset.

## 0x856820 __dtor_CPedGroupDefaultTaskAllocators__ms_StandStillAllocator  (catalogue: specified/REA, port level 0)
- atexit destructor, `__cdecl void ()`: `ECX = 0xC09910` and tail-jump to the (unnamed) allocator dtor 0x5F64C0 (0x856825). The target is `this->vtbl = 0x86C754` then `ret` (0x5F64C0): only the vtable is reset.
