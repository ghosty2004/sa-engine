# std:: template instantiations used by AI code (MSVC 7.1 STL) - RE-SPEC-FINAL (helper B)

std::vector layout (MSVC 7.1): +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend. Element sizes appearing here: 4 (pointers), 0xC, 0x14 (CPedTaskPair). Allocation via operator new 0x82119A, release via _free 0x8214BD.

## 0x5EAAA0 std::fill<T*>  (catalogue: unknown/REA, port level 0)
- `std::fill(T* first [esp+4], T* last [esp+8], const T* val [esp+0xC])`, cdecl, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5ECF10 (vector::_Insert_n instantiation).
- Behaviour: if first == last return (EAX = first) (0x5EAAAA); else loop `*p = *val` (one dword), `p += 0x4` until p == last (pointer equality test, so (last-first) must be a multiple of the size). Value re-read from *val on every iteration. EAX = last on exit (callers ignore it).

## 0x5EAAC0 std::fill<T*>  (catalogue: unknown/REA, port level 0)
- `std::fill(T* first [esp+4], T* last [esp+8], const T* val [esp+0xC])`, cdecl, plain `ret`; T = 0x14-byte element (CPedTaskPair: Ped, Task, Slot, MatchID, UsedTask). Caller: 0x5ED1B0 (vector::_Insert_n instantiation).
- Behaviour: if first == last return (EAX = first) (0x5EAACA); else loop `*p = *val` (5 dwords (`rep movsd`, ecx = 5)), `p += 0x14` until p == last (pointer equality test, so (last-first) must be a multiple of the size). Value re-read from *val on every iteration. EAX = last on exit (callers ignore it).

## 0x5EAAF0 std::fill<T*>  (catalogue: unknown/REA, port level 0)
- `std::fill(T* first [esp+4], T* last [esp+8], const T* val [esp+0xC])`, cdecl, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5ED4B0 (vector::_Insert_n instantiation).
- Behaviour: if first == last return (EAX = first) (0x5EAAFA); else loop `*p = *val` (one dword), `p += 0x4` until p == last (pointer equality test, so (last-first) must be a multiple of the size). Value re-read from *val on every iteration. EAX = last on exit (callers ignore it).

## 0x5EAB10 std::fill<T*>  (catalogue: unknown/REA, port level 0)
- `std::fill(T* first [esp+4], T* last [esp+8], const T* val [esp+0xC])`, cdecl, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5EDA50 (vector::_Insert_n instantiation).
- Behaviour: if first == last return (EAX = first) (0x5EAB1A); else loop `*p = *val` (one dword), `p += 0x4` until p == last (pointer equality test, so (last-first) must be a multiple of the size). Value re-read from *val on every iteration. EAX = last on exit (callers ignore it).

## 0x5EAB80 std::vector<T>::sub_5EAB80 (proposed: std::copy<T*> (checked-iterator return))  (catalogue: unknown/REA, port level 0)
- Name proposal: `std::copy` for T = 0x14-byte element (CPedTaskPair: Ped, Task, Slot, MatchID, UsedTask), returning the iterator through a hidden return pointer. Caller: CPedAttractor::DeRegisterPed 0x5EC5B0 (vector::erase).
- `__cdecl Iter* (Iter* ret [esp+4], T* first [esp+8], T* last [esp+0xC], T* dest [esp+0x10])`, plain `ret`.
- Behaviour: if first == last -> `*ret = dest`, return ret (0x5EABB2). Else forward loop: copy 5 dwords (`rep movsd`, ecx = 5) from first to dest, `first += 0x14; dest += 0x14` until first == last (0x5EAB93-0x5EABA6); `*ret = dest` (end of destination) and return ret (0x5EABA8-0x5EABAE).

## 0x5EAC50 std::vector<T>::sub_5EAC50 (proposed: std::copy_backward<T*>)  (catalogue: unknown/REA, port level 0)
- `__cdecl Iter* (Iter* ret [esp+4], T* first [esp+8], T* last [esp+0xC], T* destEnd [esp+0x10])`, plain `ret`; T = 0x14-byte element (CPedTaskPair: Ped, Task, Slot, MatchID, UsedTask). Caller: 0x5EB170.
- Behaviour: if first == last -> `*ret = destEnd`, return ret. Else loop: `last -= 0x14; destEnd -= 0x14; *destEnd = *last` (5 dwords (`rep movsd`, ecx = 5)) until last == first; `*ret = destEnd` (new beginning of the destination); EAX = ret.

## 0x5EACC0 std::vector<T>::sub_5EACC0 (proposed: std::copy_backward<T*>)  (catalogue: unknown/REA, port level 0)
- `__cdecl Iter* (Iter* ret [esp+4], T* first [esp+8], T* last [esp+0xC], T* destEnd [esp+0x10])`, plain `ret`; T = 0xC-byte element (3 dwords). Caller: 0x5EB220.
- Behaviour: if first == last -> `*ret = destEnd`, return ret. Else loop: `last -= 0xC; destEnd -= 0xC; *destEnd = *last` (3 dwords (+0, +4, +8)) until last == first; `*ret = destEnd` (new beginning of the destination); EAX = ret.

## 0x5EAE10 std::copy_backward<T*> (proposed: std::uninitialized_copy<T*>)  (catalogue: unknown/REA, port level 0)
- Name correction: this is `std::uninitialized_copy` (forward copy with placement-new null check), not copy_backward. `__cdecl T* (T* first [esp+4], T* last [esp+8], T* dest [esp+0xC])`, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Callers: 0x5EB930, 0x5ECF10.
- Behaviour: EAX = dest; if first == last return dest. Else loop: `if dest != NULL` -> copy one dword from *first to *dest (the placement-new null test is done per element); `first += 0x4; dest += 0x4` until first == last. Returns EAX = dest end.

## 0x5EAE40 std::vector<T>::sub_5EAE40 (proposed: std::uninitialized_copy<T*>)  (catalogue: unknown/REA, port level 0)
- Name correction: this is `std::uninitialized_copy` (forward copy with placement-new null check), not copy_backward. `__cdecl T* (T* first [esp+4], T* last [esp+8], T* dest [esp+0xC])`, plain `ret`; T = 0x14-byte element (CPedTaskPair: Ped, Task, Slot, MatchID, UsedTask). Callers: 0x5EB960, 0x5ED1B0.
- Behaviour: EAX = dest; if first == last return dest. Else loop: `if dest != NULL` -> copy 5 dwords (`rep movsd`, ecx = 5) from *first to *dest (the placement-new null test is done per element); `first += 0x14; dest += 0x14` until first == last. Returns EAX = dest end.

## 0x5EAE70 std::copy_backward<T*> (proposed: std::uninitialized_copy<T*>)  (catalogue: unknown/REA, port level 0)
- Name correction: this is `std::uninitialized_copy` (forward copy with placement-new null check), not copy_backward. `__cdecl T* (T* first [esp+4], T* last [esp+8], T* dest [esp+0xC])`, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Callers: 0x5EB990, 0x5ED4B0.
- Behaviour: EAX = dest; if first == last return dest. Else loop: `if dest != NULL` -> copy one dword from *first to *dest (the placement-new null test is done per element); `first += 0x4; dest += 0x4` until first == last. Returns EAX = dest end.

## 0x5EAEA0 std::vector<T>::sub_5EAEA0 (proposed: std::uninitialized_copy<T*>)  (catalogue: unknown/REA, port level 0)
- Name correction: this is `std::uninitialized_copy` (forward copy with placement-new null check), not copy_backward. `__cdecl T* (T* first [esp+4], T* last [esp+8], T* dest [esp+0xC])`, plain `ret`; T = 0xC-byte element (3 dwords). Callers: 0x5EB9D0, 0x5ED750.
- Behaviour: EAX = dest; if first == last return dest. Else loop: `if dest != NULL` -> copy 3 dwords (+0, +4, +8) from *first to *dest (the placement-new null test is done per element); `first += 0xC; dest += 0xC` until first == last. Returns EAX = dest end.

## 0x5EAEE0 std::copy_backward<T*> (proposed: std::uninitialized_copy<T*>)  (catalogue: unknown/REA, port level 0)
- Name correction: this is `std::uninitialized_copy` (forward copy with placement-new null check), not copy_backward. `__cdecl T* (T* first [esp+4], T* last [esp+8], T* dest [esp+0xC])`, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Callers: 0x5EBA00, 0x5EDA50.
- Behaviour: EAX = dest; if first == last return dest. Else loop: `if dest != NULL` -> copy one dword from *first to *dest (the placement-new null test is done per element); `first += 0x4; dest += 0x4` until first == last. Returns EAX = dest end.

## 0x5EB140 std::copy_backward<T*>  (catalogue: unknown/REA, port level 0)
- `__cdecl Iter* (Iter* ret [esp+4], T* first [esp+8], T* last [esp+0xC], T* destEnd [esp+0x10])`, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5ECF10.
- Behaviour: if first == last -> `*ret = destEnd`, return ret. Else loop: `last -= 0x4; destEnd -= 0x4; *destEnd = *last` (one dword) until last == first; `*ret = destEnd` (new beginning of the destination); EAX = ret.

## 0x5EB170 std::vector<T>::sub_5EB170 (proposed: std::copy_backward wrapper (_Copy_backward_opt dispatch))  (catalogue: unknown/REA, port level 1)
- `__cdecl Iter* (Iter* ret [esp+4], T* first [esp+8], T* last [esp+0xC], T* destEnd [esp+0x10])`, plain `ret`; T = 0x14-byte element (CPedTaskPair: Ped, Task, Slot, MatchID, UsedTask). Caller: 0x5ED1B0.
- Behaviour: re-pushes the arguments and calls 0x5EAC50 (copy_backward, cdecl) as `(0x5EAC50)(ret, first, last, destEnd, <extra dword = destEnd>)` (5 dwords pushed, `add esp, 0x14`; the 5th is the iterator-category tag slot, unused by the callee); returns EAX = ret (the callee already stored the result through ret).

## 0x5EB1B0 std::copy_backward<T*>  (catalogue: unknown/REA, port level 0)
- `__cdecl Iter* (Iter* ret [esp+4], T* first [esp+8], T* last [esp+0xC], T* destEnd [esp+0x10])`, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5ED4B0.
- Behaviour: if first == last -> `*ret = destEnd`, return ret. Else loop: `last -= 0x4; destEnd -= 0x4; *destEnd = *last` (one dword) until last == first; `*ret = destEnd` (new beginning of the destination); EAX = ret.

## 0x5EB1E0 std::vector<T>::sub_5EB1E0 (proposed: std::fill<T*> for a 0xC-byte T)  (catalogue: unknown/REA, port level 0)
- `std::fill(T* first [esp+4], T* last [esp+8], const T* val [esp+0xC])`, cdecl, plain `ret`; T = 0xC bytes (3 dwords). Caller: 0x5ED750 (vector<T12>::_Insert_n).
- Behaviour: if first == last return; else loop `*p = *val` (dwords +0, +4, +8, val re-read each time), `p += 0xC` until p == last (0x5EB1F3-0x5EB20C).

## 0x5EB220 std::vector<T>::sub_5EB220 (proposed: std::copy_backward wrapper (_Copy_backward_opt dispatch))  (catalogue: unknown/REA, port level 1)
- `__cdecl Iter* (Iter* ret [esp+4], T* first [esp+8], T* last [esp+0xC], T* destEnd [esp+0x10])`, plain `ret`; T = 0xC-byte element (3 dwords). Caller: 0x5ED750.
- Behaviour: re-pushes the arguments and calls 0x5EACC0 (copy_backward, cdecl) as `(0x5EACC0)(ret, first, last, destEnd, <extra dword = destEnd>)` (5 dwords pushed, `add esp, 0x14`; the 5th is the iterator-category tag slot, unused by the callee); returns EAX = ret (the callee already stored the result through ret).

## 0x5EB260 std::copy_backward<T*>  (catalogue: unknown/REA, port level 0)
- `__cdecl Iter* (Iter* ret [esp+4], T* first [esp+8], T* last [esp+0xC], T* destEnd [esp+0x10])`, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5EDA50.
- Behaviour: if first == last -> `*ret = destEnd`, return ret. Else loop: `last -= 0x4; destEnd -= 0x4; *destEnd = *last` (one dword) until last == first; `*ret = destEnd` (new beginning of the destination); EAX = ret.

## 0x5EB2A0 std::uninitialized_fill_n<T*>  (catalogue: unknown/REA, port level 0)
- `__cdecl T* (T* dest [esp+4], u32 count [esp+8], const T* val [esp+0xC])`, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Callers: 0x5EC070, 0x5ECF10, 0x5EE200.
- Behaviour: `if count == 0` (`test; jbe`) return with EAX = count (= 0, NOT dest). Else for count iterations: `if dest != NULL` copy one dword from *val into *dest; `dest += 0x4`. Returns EAX = dest end (callers ignore it; the _Ufill wrappers compute the end themselves).

## 0x5EB2E0 std::uninitialized_fill_n<T*>  (catalogue: unknown/REA, port level 0)
- `__cdecl T* (T* dest [esp+4], u32 count [esp+8], const T* val [esp+0xC])`, plain `ret`; T = 0x14-byte element (CPedTaskPair: Ped, Task, Slot, MatchID, UsedTask). Callers: 0x5EC0E0, 0x5ED1B0, 0x5EE270.
- Behaviour: `if count == 0` (`test; jbe`) return with EAX = count (= 0, NOT dest). Else for count iterations: `if dest != NULL` copy 5 dwords (`rep movsd`, ecx = 5) from *val into *dest; `dest += 0x14`. Returns EAX = dest end (callers ignore it; the _Ufill wrappers compute the end themselves). The 0x14-byte copy goes through a .HOODLUM detour (0x5EB2F9 -> 0x40A25B sets ECX = 5 and jumps back to the `rep movsd` at 0x5EB2FE); behaviour is the plain 5-dword copy.

## 0x5EB310 std::uninitialized_fill_n<T*>  (catalogue: unknown/REA, port level 0)
- `__cdecl T* (T* dest [esp+4], u32 count [esp+8], const T* val [esp+0xC])`, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Callers: 0x5EC110, 0x5ED4B0, 0x5EE300, CPedAttractor::SetTaskForPed 0x5EECA0.
- Behaviour: `if count == 0` (`test; jbe`) return with EAX = count (= 0, NOT dest). Else for count iterations: `if dest != NULL` copy one dword from *val into *dest; `dest += 0x4`. Returns EAX = dest end (callers ignore it; the _Ufill wrappers compute the end themselves).

## 0x5EB340 std::uninitialized_fill_n<T*>  (catalogue: unknown/REA, port level 0)
- `__cdecl T* (T* dest [esp+4], u32 count [esp+8], const T* val [esp+0xC])`, plain `ret`; T = 0xC-byte element (3 dwords). Callers: 0x5EC140, 0x5ED750, 0x5EE370.
- Behaviour: `if count == 0` (`test; jbe`) return with EAX = count (= 0, NOT dest). Else for count iterations: `if dest != NULL` copy 3 dwords (+0, +4, +8) from *val into *dest; `dest += 0xC`. Returns EAX = dest end (callers ignore it; the _Ufill wrappers compute the end themselves).

## 0x5EB390 std::uninitialized_fill_n<T*>  (catalogue: unknown/REA, port level 0)
- `__cdecl T* (T* dest [esp+4], u32 count [esp+8], const T* val [esp+0xC])`, plain `ret`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Callers: 0x5EC1B0, 0x5EDA50, 0x5EE400.
- Behaviour: `if count == 0` (`test; jbe`) return with EAX = count (= 0, NOT dest). Else for count iterations: `if dest != NULL` copy one dword from *val into *dest; `dest += 0x4`. Returns EAX = dest end (callers ignore it; the _Ufill wrappers compute the end themselves).

## 0x5EB930 std::vector<T>::sub_5EB930 (proposed: std::vector<T>::_Ucopy)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this = ECX: vector*, unused, T* first [esp+4], T* last [esp+8], T* dest [esp+0xC])`, `ret 0xC`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5ECF10 (two sites).
- Behaviour: calls 0x5EAE10 (uninitialized_copy, cdecl) with (first, last, dest) followed by two extra dwords (ECX and dest, tag/allocator slots unused by the callee; 5 pushes, `add esp, 0x14`); returns the callee's EAX (dest end).

## 0x5EB960 std::vector<T>::sub_5EB960 (proposed: std::vector<T>::_Ucopy)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this = ECX: vector*, unused, T* first [esp+4], T* last [esp+8], T* dest [esp+0xC])`, `ret 0xC`; T = 0x14-byte element (CPedTaskPair: Ped, Task, Slot, MatchID, UsedTask). Caller: 0x5ED1B0 (two sites).
- Behaviour: calls 0x5EAE40 (uninitialized_copy, cdecl) with (first, last, dest) followed by two extra dwords (ECX and dest, tag/allocator slots unused by the callee; 5 pushes, `add esp, 0x14`); returns the callee's EAX (dest end).

## 0x5EB990 std::vector<T>::sub_5EB990 (proposed: std::vector<T>::_Ucopy)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this = ECX: vector*, unused, T* first [esp+4], T* last [esp+8], T* dest [esp+0xC])`, `ret 0xC`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5ED4B0 (two sites).
- Behaviour: calls 0x5EAE70 (uninitialized_copy, cdecl) with (first, last, dest) followed by two extra dwords (ECX and dest, tag/allocator slots unused by the callee; 5 pushes, `add esp, 0x14`); returns the callee's EAX (dest end).

## 0x5EB9D0 std::vector<T>::sub_5EB9D0 (proposed: std::vector<T>::_Ucopy)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this = ECX: vector*, unused, T* first [esp+4], T* last [esp+8], T* dest [esp+0xC])`, `ret 0xC`; T = 0xC-byte element (3 dwords). Caller: 0x5ED750 (two sites).
- Behaviour: calls 0x5EAEA0 (uninitialized_copy, cdecl) with (first, last, dest) followed by two extra dwords (ECX and dest, tag/allocator slots unused by the callee; 5 pushes, `add esp, 0x14`); returns the callee's EAX (dest end).

## 0x5EBA00 std::vector<T>::sub_5EBA00 (proposed: std::vector<T>::_Ucopy)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this = ECX: vector*, unused, T* first [esp+4], T* last [esp+8], T* dest [esp+0xC])`, `ret 0xC`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5EDA50 (two sites).
- Behaviour: calls 0x5EAEE0 (uninitialized_copy, cdecl) with (first, last, dest) followed by two extra dwords (ECX and dest, tag/allocator slots unused by the callee; 5 pushes, `add esp, 0x14`); returns the callee's EAX (dest end).

## 0x5EC040 std::vector<T>::_Tidy  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: std::vector<T>* with MSVC 7.1 layout +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend)`, plain `ret`. No direct callers (EH-unwind / inlined; out-of-line copy).
- Behaviour: `if _Myfirst != NULL` -> `_free(_Myfirst)` 0x8214BD (cdecl, `add esp, 4`); then `_Myfirst = 0; _Mylast = 0; _Myend = 0` (stores +4, +8, +0xC in that order). No element destructors (trivial T).

## 0x5EC070 std::vector<T>::sub_5EC070 (proposed: std::vector<T>::_Ufill)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this = ECX: vector*, T* dest [esp+4], u32 count [esp+8], const T* val [esp+0xC])`, `ret 0xC`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5ECF10.
- Behaviour: calls 0x5EB2A0 (uninitialized_fill_n, cdecl) with (dest, count, val, ECX, val) (5 pushes, `add esp, 0x14`; the last two are unused tag slots), then returns EAX = dest + count*0x4 (computed locally, independent of the callee's return).

## 0x5EC0B0 std::vector<T>::_Tidy  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: std::vector<T>* with MSVC 7.1 layout +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend)`, plain `ret`. No direct callers (EH-unwind / inlined; out-of-line copy).
- Behaviour: `if _Myfirst != NULL` -> `_free(_Myfirst)` 0x8214BD (cdecl, `add esp, 4`); then `_Myfirst = 0; _Mylast = 0; _Myend = 0` (stores +4, +8, +0xC in that order). No element destructors (trivial T).

## 0x5EC0E0 std::vector<T>::sub_5EC0E0 (proposed: std::vector<T>::_Ufill)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this = ECX: vector*, T* dest [esp+4], u32 count [esp+8], const T* val [esp+0xC])`, `ret 0xC`; T = 0x14-byte element (CPedTaskPair: Ped, Task, Slot, MatchID, UsedTask). Caller: 0x5ED1B0.
- Behaviour: calls 0x5EB2E0 (uninitialized_fill_n, cdecl) with (dest, count, val, ECX, val) (5 pushes, `add esp, 0x14`; the last two are unused tag slots), then returns EAX = dest + count*0x14 (computed locally, independent of the callee's return).

## 0x5EC110 std::vector<T>::sub_5EC110 (proposed: std::vector<T>::_Ufill)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this = ECX: vector*, T* dest [esp+4], u32 count [esp+8], const T* val [esp+0xC])`, `ret 0xC`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5ED4B0.
- Behaviour: calls 0x5EB310 (uninitialized_fill_n, cdecl) with (dest, count, val, ECX, val) (5 pushes, `add esp, 0x14`; the last two are unused tag slots), then returns EAX = dest + count*0x4 (computed locally, independent of the callee's return).

## 0x5EC140 std::vector<T>::sub_5EC140 (proposed: std::vector<T>::_Ufill)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this = ECX: vector*, T* dest [esp+4], u32 count [esp+8], const T* val [esp+0xC])`, `ret 0xC`; T = 0xC-byte element (3 dwords). Caller: 0x5ED750.
- Behaviour: calls 0x5EB340 (uninitialized_fill_n, cdecl) with (dest, count, val, ECX, val) (5 pushes, `add esp, 0x14`; the last two are unused tag slots), then returns EAX = dest + count*0xC (computed locally, independent of the callee's return).

## 0x5EC180 std::vector<T>::_Tidy  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: std::vector<T>* with MSVC 7.1 layout +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend)`, plain `ret`. No direct callers (EH-unwind / inlined; out-of-line copy).
- Behaviour: `if _Myfirst != NULL` -> `_free(_Myfirst)` 0x8214BD (cdecl, `add esp, 4`); then `_Myfirst = 0; _Mylast = 0; _Myend = 0` (stores +4, +8, +0xC in that order). No element destructors (trivial T).

## 0x5EC1B0 std::vector<T>::sub_5EC1B0 (proposed: std::vector<T>::_Ufill)  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this = ECX: vector*, T* dest [esp+4], u32 count [esp+8], const T* val [esp+0xC])`, `ret 0xC`; T = 4-byte element (pointer type, e.g. CPed*/CTask*/attractor*). Caller: 0x5EDA50.
- Behaviour: calls 0x5EB390 (uninitialized_fill_n, cdecl) with (dest, count, val, ECX, val) (5 pushes, `add esp, 0x14`; the last two are unused tag slots), then returns EAX = dest + count*0x4 (computed locally, independent of the callee's return).

## 0x5EC1F0 std::vector<T>::sub_5EC1F0 (proposed: std::vector<T>::_Tidy)  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: std::vector<T>* with MSVC 7.1 layout +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend)`, plain `ret`. One caller (EH unwind/dtor path; see bundle).
- Behaviour: `if _Myfirst != NULL` -> `_free(_Myfirst)` 0x8214BD (cdecl, `add esp, 4`); then `_Myfirst = 0; _Mylast = 0; _Myend = 0` (stores +4, +8, +0xC in that order). No element destructors (trivial T). The `m_First = 0` store is moved into a .HOODLUM detour (0x5EC205 -> 0x40A22C -> back to 0x5EC20A); same order and effect.

## 0x5EC260 std::vector<T>::_Tidy  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: std::vector<T>* with MSVC 7.1 layout +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend)`, plain `ret`. Referenced only from the EH-unwind funclets of CPedAttractor::Destructor 0x5EC410.
- Behaviour: `if _Myfirst != NULL` -> `_free(_Myfirst)` 0x8214BD (cdecl, `add esp, 4`); then `_Myfirst = 0; _Mylast = 0; _Myend = 0` (stores +4, +8, +0xC in that order). No element destructors (trivial T).

## 0x5EC2D0 std::vector<T>::_Tidy  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: std::vector<T>* with MSVC 7.1 layout +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend)`, plain `ret`. No direct callers (EH-unwind / inlined; out-of-line copy).
- Behaviour: `if _Myfirst != NULL` -> `_free(_Myfirst)` 0x8214BD (cdecl, `add esp, 4`); then `_Myfirst = 0; _Mylast = 0; _Myend = 0` (stores +4, +8, +0xC in that order). No element destructors (trivial T).

## 0x5EC340 std::vector<T>::_Tidy  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: std::vector<T>* with MSVC 7.1 layout +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend)`, plain `ret`. Referenced only from the EH-unwind funclets of CPedAttractorManager::CPedAttractorManager 0x5EDF10.
- Behaviour: `if _Myfirst != NULL` -> `_free(_Myfirst)` 0x8214BD (cdecl, `add esp, 4`); then `_Myfirst = 0; _Mylast = 0; _Myend = 0` (stores +4, +8, +0xC in that order). No element destructors (trivial T).

## 0x5EC3B0 std::vector<T>::_Tidy  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: std::vector<T>* with MSVC 7.1 layout +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend)`, plain `ret`. No direct callers (EH-unwind / inlined; out-of-line copy).
- Behaviour: `if _Myfirst != NULL` -> `_free(_Myfirst)` 0x8214BD (cdecl, `add esp, 4`); then `_Myfirst = 0; _Mylast = 0; _Myend = 0` (stores +4, +8, +0xC in that order). No element destructors (trivial T).

## 0x5EC3E0 std::vector<T>::_Tidy  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: std::vector<T>* with MSVC 7.1 layout +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend)`, plain `ret`. No direct callers (EH-unwind / inlined; out-of-line copy).
- Behaviour: `if _Myfirst != NULL` -> `_free(_Myfirst)` 0x8214BD (cdecl, `add esp, 4`); then `_Myfirst = 0; _Mylast = 0; _Myend = 0` (stores +4, +8, +0xC in that order). No element destructors (trivial T).

## 0x5EC820 std::vector<T>::_Tidy  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: std::vector<T>* with MSVC 7.1 layout +0 allocator (4 bytes), +4 _Myfirst, +8 _Mylast, +0xC _Myend)`, plain `ret`. No direct callers (EH-unwind / inlined; out-of-line copy).
- Behaviour: `if _Myfirst != NULL` -> `_free(_Myfirst)` 0x8214BD (cdecl, `add esp, 4`); then `_Myfirst = 0; _Mylast = 0; _Myend = 0` (stores +4, +8, +0xC in that order). No element destructors (trivial T).

## 0x5ECAB0 std::vector<T>::_Xlen  (catalogue: unknown/REA, port level 5)
- `__thiscall [[noreturn]] void (this unused)`, never returns; SEH frame (handler pushed at +2). Caller: 0x5ECF10 (vector::_Insert_n, when the requested size exceeds max_size()).
- Behaviour: builds a local `std::string` (SSO capacity 0xF, size 0, first char 0) and assigns the C string at 0x860AB0 = "vector<T> too long" (length computed by an inline strlen loop) via std::string::assign(const char*, len) 0x4F3F60; constructs a `std::length_error` from it with 0x4F4090 (logic_error(const string&)), sets its vtable to 0x887D68 (length_error vftable), and throws it: `_CxxThrowException(&obj, 0x893634 throw-info)` 0x822D5D. Port: raise the equivalent fatal "vector<T> too long" length error.

## 0x5ECB30 std::vector<T>::_Xlen  (catalogue: unknown/REA, port level 5)
- `__thiscall [[noreturn]] void (this unused)`, never returns; SEH frame (handler pushed at +2). Caller: 0x5ED1B0 (vector::_Insert_n, when the requested size exceeds max_size()).
- Behaviour: builds a local `std::string` (SSO capacity 0xF, size 0, first char 0) and assigns the C string at 0x860AB0 = "vector<T> too long" (length computed by an inline strlen loop) via std::string::assign(const char*, len) 0x4F3F60; constructs a `std::length_error` from it with 0x4F4090 (logic_error(const string&)), sets its vtable to 0x887D68 (length_error vftable), and throws it: `_CxxThrowException(&obj, 0x893634 throw-info)` 0x822D5D. Port: raise the equivalent fatal "vector<T> too long" length error.

## 0x5ECBB0 std::vector<T>::_Xlen  (catalogue: unknown/REA, port level 5)
- `__thiscall [[noreturn]] void (this unused)`, never returns; SEH frame (handler pushed at +2). Caller: 0x5ED4B0 (vector::_Insert_n, when the requested size exceeds max_size()).
- Behaviour: builds a local `std::string` (SSO capacity 0xF, size 0, first char 0) and assigns the C string at 0x860AB0 = "vector<T> too long" (length computed by an inline strlen loop) via std::string::assign(const char*, len) 0x4F3F60; constructs a `std::length_error` from it with 0x4F4090 (logic_error(const string&)), sets its vtable to 0x887D68 (length_error vftable), and throws it: `_CxxThrowException(&obj, 0x893634 throw-info)` 0x822D5D. Port: raise the equivalent fatal "vector<T> too long" length error.

## 0x5ECC30 std::vector<T>::_Xlen  (catalogue: unknown/REA, port level 5)
- `__thiscall [[noreturn]] void (this unused)`, never returns; SEH frame (handler pushed at +2). Caller: 0x5ED750 (vector::_Insert_n, when the requested size exceeds max_size()).
- Behaviour: builds a local `std::string` (SSO capacity 0xF, size 0, first char 0) and assigns the C string at 0x860AB0 = "vector<T> too long" (length computed by an inline strlen loop) via std::string::assign(const char*, len) 0x4F3F60; constructs a `std::length_error` from it with 0x4F4090 (logic_error(const string&)), sets its vtable to 0x887D68 (length_error vftable), and throws it: `_CxxThrowException(&obj, 0x893634 throw-info)` 0x822D5D. Port: raise the equivalent fatal "vector<T> too long" length error.

## 0x5ECCB0 std::vector<T>::_Xlen  (catalogue: unknown/REA, port level 5)
- `__thiscall [[noreturn]] void (this unused)`, never returns; SEH frame (handler pushed at +2). Caller: 0x5EDA50 (vector::_Insert_n, when the requested size exceeds max_size()).
- Behaviour: builds a local `std::string` (SSO capacity 0xF, size 0, first char 0) and assigns the C string at 0x860AB0 = "vector<T> too long" (length computed by an inline strlen loop) via std::string::assign(const char*, len) 0x4F3F60; constructs a `std::length_error` from it with 0x4F4090 (logic_error(const string&)), sets its vtable to 0x887D68 (length_error vftable), and throws it: `_CxxThrowException(&obj, 0x893634 throw-info)` 0x822D5D. Port: raise the equivalent fatal "vector<T> too long" length error.

## 0x5ECF10 std::vector<T>::_Insert_n  (catalogue: unknown/REA, port level 6)
- MSVC 7.1 `std::vector<T>::_Insert_n(iterator where [ebp+8], size_type count [ebp+0xC], const T& val [ebp+0x10])` for a 4-byte T (pointer), `__thiscall` (this = ECX: vector with +4 _Myfirst, +8 _Mylast, +0xC _Myend), `ret 0xC`, EBP frame + SEH (handler 0x83DB60; try/catch around the reallocation copy: the catch funclet frees the new block with _free 0x8214BD and rethrows). Caller: 0x5EE200. Structurally identical to 0x5EDA50 (same instruction sequence, other instantiation).
- Step 0: `tmp = *val` (copied into the val argument slot, 0x5ECF32/0x5ECF3D; every later fill uses &tmp, so inserting an element of the vector itself is safe). `cap = first ? (end-first)>>2 : 0` (0x5ECF38-0x5ECF4B; `sar`, signed).
- `if count == 0` -> return (0x5ECF53). `size = first ? (last-first)>>2 : 0`; `if (u32)(0x3FFFFFFF - size) < count` (`jae` unsigned, 0x5ECF72) -> `_Xlen()` 0x5ECAB0 (throws "vector<T> too long"; the code after it is the unreachable epilogue).
- Case A, reallocation: `if cap < size + count` (unsigned, 0x5ECFA2): `newcap = (0x3FFFFFFF - cap/2 < cap) ? 0 : cap + cap/2` (0x5ECFA8-0x5ECFBB; `cap/2` = `shr 1`); `if newcap < size + count` -> `newcap = size + count` (0x5ECFCF-0x5ECFE3). `buf = operator new(newcap*4)` 0x82119A (0x5ECFEC). Then: `p = uninitialized_copy(first, where, buf)` 0x5EAE10 (0x5ED016); `uninitialized_fill_n(p, count, &tmp)` 0x5EB2A0 (0x5ED02E); `uninitialized_copy(where, last, p + count)` (0x5ED052). `n = size_old + count` (size re-read from the old first/last, 0x5ED05D-0x5ED070); `if first` -> `_free(first)` 0x8214BD (0x5ED077); `end = buf + newcap*4; last = buf + n*4; first = buf` (stores +0xC, +8, +4 in that order, 0x5ED087-0x5ED08D).
- Case B, fits and `(u32)((last - where)>>2) < count` (0x5ED0C7): `_Ucopy(where, last, where + count*4)` 0x5EB930 (0x5ED0E8); `_Ufill(last, count - (last-where)/4, &tmp)` 0x5EC070 (0x5ED10B); `last += count*4` (0x5ED11B); then `fill(where, last - count*4 (= old last), &tmp)` 0x5EAAA0 (shared tail 0x5ED187-0x5ED192).
- Case C, fits and the tail is at least count long: `last' = _Ucopy(last - count*4, last, last)` 0x5EB930 (0x5ED14A), stored to `+8` (0x5ED150); `copy_backward(where, oldlast - count*4, oldlast)` 0x5EB140 (hidden-return form, 0x5ED171); `fill(where, where + count*4, &tmp)` 0x5EAAA0 (0x5ED192).
- Port: equivalent to `Vec::splice`-style insertion of `count` copies of `tmp` at `where`, with the exact MSVC growth rule above (capacity grows by 50%, at least to the needed size; max_size 0x3FFFFFFF elements). Allocation via operator new 0x82119A / free via _free 0x8214BD matters only for the heap layout.

## 0x5EDA50 std::vector<T>::_Insert_n  (catalogue: unknown/REA, port level 6)
- MSVC 7.1 `std::vector<T>::_Insert_n(iterator where [ebp+8], size_type count [ebp+0xC], const T& val [ebp+0x10])` for a 4-byte T (pointer), `__thiscall` (this = ECX: vector with +4 _Myfirst, +8 _Mylast, +0xC _Myend), `ret 0xC`, EBP frame + SEH (handler 0x83DBA0; try/catch around the reallocation copy: the catch funclet frees the new block with _free 0x8214BD and rethrows). Caller: 0x5EE400 (CPedAttractorManager::sub_5EE400). Structurally identical to 0x5ECF10 (same instruction sequence, other instantiation).
- Step 0: `tmp = *val` (copied into the val argument slot, 0x5EDA72/0x5EDA7D; every later fill uses &tmp, so inserting an element of the vector itself is safe). `cap = first ? (end-first)>>2 : 0` (0x5EDA78-0x5EDA8B; `sar`, signed).
- `if count == 0` -> return (0x5EDA93). `size = first ? (last-first)>>2 : 0`; `if (u32)(0x3FFFFFFF - size) < count` (`jae` unsigned, 0x5EDAB2) -> `_Xlen()` 0x5ECCB0 (throws "vector<T> too long"; the code after it is the unreachable epilogue).
- Case A, reallocation: `if cap < size + count` (unsigned, 0x5EDAE2): `newcap = (0x3FFFFFFF - cap/2 < cap) ? 0 : cap + cap/2` (0x5EDAE8-0x5EDAFB; `cap/2` = `shr 1`); `if newcap < size + count` -> `newcap = size + count` (0x5EDB0F-0x5EDB23). `buf = operator new(newcap*4)` 0x82119A (0x5EDB2C). Then: `p = uninitialized_copy(first, where, buf)` 0x5EAEE0 (0x5EDB56); `uninitialized_fill_n(p, count, &tmp)` 0x5EB390 (0x5EDB6E); `uninitialized_copy(where, last, p + count)` (0x5EDB92). `n = size_old + count` (size re-read from the old first/last, 0x5EDB9D-0x5EDBB0); `if first` -> `_free(first)` 0x8214BD (0x5EDBB7); `end = buf + newcap*4; last = buf + n*4; first = buf` (stores +0xC, +8, +4 in that order, 0x5EDBC7-0x5EDBCD).
- Case B, fits and `(u32)((last - where)>>2) < count` (0x5EDC07): `_Ucopy(where, last, where + count*4)` 0x5EBA00 (0x5EDC28); `_Ufill(last, count - (last-where)/4, &tmp)` 0x5EC1B0 (0x5EDC4B); `last += count*4` (0x5EDC5B); then `fill(where, last - count*4 (= old last), &tmp)` 0x5EAB10 (shared tail 0x5EDCC7-0x5EDCD2).
- Case C, fits and the tail is at least count long: `last' = _Ucopy(last - count*4, last, last)` 0x5EBA00 (0x5EDC8A), stored to `+8` (0x5EDC90); `copy_backward(where, oldlast - count*4, oldlast)` 0x5EB260 (hidden-return form, 0x5EDCB1); `fill(where, where + count*4, &tmp)` 0x5EAB10 (0x5EDCD2).
- Port: equivalent to `Vec::splice`-style insertion of `count` copies of `tmp` at `where`, with the exact MSVC growth rule above (capacity grows by 50%, at least to the needed size; max_size 0x3FFFFFFF elements). Allocation via operator new 0x82119A / free via _free 0x8214BD matters only for the heap layout.
