# _static-init-J1 (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x8569B0 __dtor_CTaskComplexFollowLeaderInFormation__ms_offsets  (catalogue: specified/REA, port level 0)
- atexit callback, `__cdecl void ()`: empty body (`ret` at 0x8569B0); the destroyed static has a trivial destructor.

## 0x8568F0 __dtor_col1  (catalogue: specified/REA, port level 5)
- atexit destructor (`__cdecl void ()`): `mov ecx, 0xC17824` (static CColModel `col1`, gr name unknown; used by the AI collision
  checks) then tail-jump CColModel::~CColModel 0x40F700. Registered by the matching static initialiser.
