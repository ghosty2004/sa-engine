# CAnimBlendNode (ANIM) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x4CFA30 sub_4cfa30 (proposed: CompressQuaternion, `FixedQuat<int16,4096.f>` from CQuaternion; catalogue: unnamed, unknown/REA, port level 0)
- Name evidence: only caller CAnimBlendNode::CalcDeltasCompressed 0x4D0350; scale 4096.0 equals gr's
  `KeyFrameCompressed::Rot` type `FixedQuat<int16, 4096.f>` (`Animation/AnimSequenceFrames.h:34`).
- Convention: `__cdecl void(int16* out [esp+4], const float* q [esp+8])`, plain `ret` (caller cleans).
  Preserves ESI/EDI (pushed). Callee: `_ftol2` 0x821B40 (x87 ST0 -> truncation toward zero, result in EDX:EAX).
- Behaviour, for k = 0,1,2,3 in that order (x, y, z, w; offsets +0,+4,+8,+0xC of q, +0,+2,+4,+6 of out):
  `out[k] = (int16)_ftol2(q[k] * 4096.0f)` - `fld f32 q[k]; fmul f32 [0x858BF8]=4096.0f`, truncate, store the low
  16 bits of EAX (0x4CFA35-0x4CFA7C). Out-of-range values wrap (low word of the 64-bit result, no clamping).
- gta-reversed: corresponds to `FixedQuat` assignment; gr `extensions/FixedFloat.hpp` `Set(v, round=false)` does
  `static_cast<int16>(v * 4096.f)` = truncation, same as the original (but gr casts straight to int16, which is
  UB/saturating-free on overflow on x86 only via cvttss2si -> 0x8000; the original takes the low word of the int64
  `_ftol2` result - differs only for |q*4096| >= 32768, i.e. |q| >= 8, never for unit quaternions).
