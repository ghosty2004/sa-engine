# CTaskAllocator (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x69D980 CTaskAllocator::operator new  (catalogue: specified/REA, port level 1)
- `__cdecl void* (size_t)`: entry obfuscated (`nop; jmp 0x401E3E` loads ECX = ms_pTaskAllocatorPool `*(0xB744BC)`, and returns to 0x69D986); tail-jump `CTaskAllocatorPool::New` 0x69D8E0 (thiscall on the pool). The size argument is ignored.
- Checked: `[0x15606F0] = 0x01A0D9D2`, `[0x410860] = 0xFF166AE9`; `0x01A0D9D2 + 0xFF166AE9 + 1 = 0xB744BC` (mod 2^32), so ECX = `*(CPool**)0xB744BC` (the `mov ecx,[ecx]`).

## 0x69D990 CTaskAllocator::operator delete  (catalogue: specified/REA, port level 0)
- `__cdecl void (void* p [esp+4])`, `ret`: pool = [0xB744BC] (ms_pTaskAllocatorPool); `idx = (p - m_pObjects) >> 5` (element 0x20, `sar`); `m_byteMap[idx] |= 0x80`;
  `if (idx < m_nFirstFree) m_nFirstFree = idx` (0x69D99B-0x69D9B5).

## 0x69DAC0 CTaskAllocator::~CTaskAllocator  (catalogue: specified/REA, port level 0)
- Scalar deleting destructor: `this->vtbl = 0x870E30` (0x69DAC8); if flags & 1: allocator-pool free inlined ([0xB744BC], `>> 5`, byteMap |= 0x80, firstFree = min,
  0x69DAD1-0x69DAF1); returns this; `ret 4`.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x69BB40 CTaskAllocator::sub_69BB40 (proposed: CTaskAllocator::~CTaskAllocator (non-deleting))  (catalogue: unknown/REA, port level 0)
- `__thiscall void (this)`: `vtbl = 0x870E30` (CTaskAllocator base vtable; done in the .HOODLUM detour 0x40A41C, entry `nop; jmp`); `ret` (0x69BB46). 8 references (base-dtor call sites of the derived allocators).
