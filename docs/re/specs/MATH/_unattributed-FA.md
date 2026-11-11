# _unattributed-FA (MATH) - RE-SPEC-FINAL group A

## 0x4EEA80 approxEqual  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl bool/int (float a [esp+4], float b [esp+8], float eps [esp+0xC])`, plain `ret`, result EAX = 1 or 0 (full dword).
- Behaviour: x87 `fld a; fsub b; fabs; fcomp eps; test ah,5; jp -> 0` -> returns 1 iff `|a - b| < eps` (strict; equality or NaN -> 0).
- Callers include 0x606580 (pedestrian potential-collision scan). gta-reversed: absent as a hooked function. Open questions: none.

## 0x59C9D0 sub_59C9D0 (empty)  (catalogue: unknown/REA, port level 0)
- Empty function (`ret`), no callers. Port as a no-op. Open questions: none.
