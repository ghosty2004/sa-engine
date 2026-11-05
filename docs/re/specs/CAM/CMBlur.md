# CMBlur (CAM) - full specs

Both functions are empty PC stubs of the PS2 motion-blur code.

## 0x71D700 CMBlur::MotionBlurRender  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl void ()` as called; body is a single `ret` at 0x71D700 (no stack cleanup).
  Arguments pushed by the caller (if any) are ignored and cleaned by the caller.
- Callers: CCamera::RenderMotionBlur 0x50B8F0. Callees: none. Globals: none.
- Behaviour: no-op.
- gta-reversed: not reversed in gr (no hook); nothing to compare.

## 0x71D710 CMBlur::MotionBlurOpen  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl bool/int32 ()`; `mov eax, 1` (0x71D710), `ret` (0x71D715). Returns 1 in the full
  EAX (callers may test AL or EAX; both are 1). Any pushed arguments are ignored.
- Callers: CCamera::SetRwCamera 0x50C100. Callees: none. Globals: none.
- Behaviour: return 1.
- gta-reversed: no hook; nothing to compare.
