# CAttractorScanner (CALLED-ONLY) — RE-SPEC-REST

## 0x5FFF90 CAttractorScanner::Clear  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this; gr layout `AttractorScanner.h`: +0x18 i32[10], +0x40 i32[10], +0x68 f32[10]), no args, plain `ret`.
  Callers: CPedIntelligence::FlushIntelligence 0x601DA0 (tail call), CEventScanner::CEventScanner 0x605300, 0x6060A0.
- Behaviour: for i = 0..9 (0x5FFFA0 loop): `this+0x18[i] = 0`, `this+0x40[i] = 0`, `this+0x68[i] = [0x86C52C]=15.0f * [0x86C52C]=15.0f`
  (= 225.0f, computed with fld/fmul/fstp); then if i != 4 and i != 7: `this+0x68[i] = 25.0f` (immediate 0x41C80000). So slots 4 and 7
  get 225.0 (15²), the others 25.0 (5²). The fields at +0x0..+0x14 (flag, timer, effect in use) are NOT cleared.
- gta-reversed: `plugin::CallMethod<0x5FFF90>` only.
