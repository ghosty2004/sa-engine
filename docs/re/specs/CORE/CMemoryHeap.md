# CMemoryHeap (CORE) - RE-SPEC-FINAL group A

## 0x72EC60 CMemoryHeap::Shutdown  (catalogue: specified/REA, port level 0)
- `__thiscall void (CMemoryHeap* this)`, plain `ret`: `free(this->+0)` via 0x82413F (CRT free-like, cdecl); `+0 = 0`. No code callers.
- gta-reversed: CMemoryHeap (absent). Open questions: identity of 0x82413F (CRT heap free, not the _free at 0x8213AE).
