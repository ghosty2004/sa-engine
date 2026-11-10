# CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset> (AI) - RE-SPEC-3 group I3

## 0x661F10 CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::`scalar deleting destructor'  (catalogue name ~CTaskComplexSeekEntity)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x86FBB4. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x661F30 (0x661F13): SEH; `vtbl = 0x86FBB4`; if [+0xC] (m_Entity) != 0 CleanUpOldReference; then the embedded seek-pos calculator at +0x40 gets its base vtable `[+0x40] = 0x859DC4`; then ~CTaskComplex 0x61A3D0. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x661F18, call at 0x661F20; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.
