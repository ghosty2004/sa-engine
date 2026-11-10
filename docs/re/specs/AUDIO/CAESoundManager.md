# CAESoundManager (AUDIO) - full specs (RE-SPEC-3, group U1)

Instance AESoundManager 0xB62CB0 (size 0x8CBC). +4 CAESound m_VirtuallyPlayingSoundList[300] (stride 0x74).

## 0x4EFD30 CAESoundManager::CAESoundManager  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CAESoundManager* (this = ECX)`, returns this, plain `ret`. Caller: `__init_AESoundManager` 0x84CD20.
- Callees: `eh vector constructor iterator` 0x821D30 (`__stdcall (void* arr, u32 size, i32 count, ctor, dtor)`, callee pops).
- Behaviour: `eh_vector_constructor_iterator(this + 4, 0x74, 0x12C (300), 0x446FE0, 0x4EF660)` (0x4EFD31-0x4EFD48). 0x446FE0
  is the CAESound default ctor (`mov eax,ecx; mov [eax+8],0; ret` - only m_PhysicalEntity = 0), 0x4EF660 is ~CAESound
  (used for unwinding). No other field is initialised.
- gta-reversed: gr's CAESound default ctor has default member initialisers for every field; the original constructor only
  zeroes +8 of each sound (static object: rest is .bss zero, so -1/-100/1 defaults of gr are NOT present in the original).
  Port: only clear +8 per element. Open questions: none.
