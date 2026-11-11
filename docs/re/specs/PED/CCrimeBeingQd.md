# CCrimeBeingQd (PED) - RE-SPEC-FINAL group A

## 0x561FF0 CCrimeBeingQd::CCrimeBeingQd  (catalogue: unknown/REA, port level 0)
Proposed: array-init helper of CCrimeBeingQd (vector-constructor-like loop; evidence: tail of CWanted::ClearQdCrimes 0x561FE0 which sets
EAX = this+0x34 (m_CrimesBeingQd) and ECX = 0x10 (16) via the HOODLUM chunk 0x404CD2, then jumps here).
- Convention: REGISTER args: EAX = first element (CCrimeBeingQd*), ECX = count (must be >= 1; `dec ecx; jne` loop, count 0 would loop 2^32 times);
  plain `ret`; clobbers EAX (ends one past the last element) and ECX.
- Behaviour: `do { *(u32*)eax = 0 /* m_nCrimeType = CRIME_NONE */; eax += 0x1C; } while (--ecx)`. Only the first dword of each 0x1C-byte entry is
  written.
- gta-reversed: CWanted::ClearQdCrimes (Wanted.cpp) loops over m_CrimesBeingQd and sets the type to CRIME_NONE - same effect. Open questions: none.
