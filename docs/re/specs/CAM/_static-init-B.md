# CAM static initialisers / atexit destructors (group B)

`0x821D30` = `eh_vector_constructor_iterator(ptr, size, count, ctor, dtor)`, `0x821E02` =
`eh_vector_destructor_iterator(ptr, size, count, dtor)`, `0x821D1E` = `_atexit(fn)`. All are `__cdecl void()`.

## 0x84D6D0 __init_gHandShaker  (catalogue: specified/REA, port level 0)
- `eh_vector_constructor_iterator(0xB6ECA0 /*gHandShaker*/, 0x94, 6, CHandShaker::CHandShaker 0x517740,
  CHandShaker::~CHandShaker 0x509810)` (0x84D6E6), then `_atexit(__dtor_gHandShaker 0x8563B0)`; `pop ecx; ret`.

## 0x84D700 __init_TheCamera  (catalogue: specified/REA, port level 3)
- `CCamera::CCamera(this = 0xB6F028 /*TheCamera*/)` (0x51A450, thiscall via ECX), then `_atexit(__dtor_TheCamera 0x856410)`.

## 0x84D720 __init_gIdleCam  (catalogue: specified/REA, port level 3)
- `CIdleCam::Init(this = 0xB6FDA0 /*gIdleCam*/)` (0x50E6D0; the static "constructor" is the Init method), then
  `_atexit(__dtor_gIdleCam 0x8563D0)`.

## 0x84D740 __init_preMirrorMat  (catalogue: specified/REA, port level 0)
- Only `_atexit(__dtor_preMirrorMat 0x8563E0)`. No constructor call: `preMirrorMat` (0xB6FE40, CMatrix) stays zero
  (.bss) until written.

## 0x84D750 __init_gpCamColVars  (catalogue: specified/REA, port level 0)
- `gpCamColVars (0xB6FE88, float*) = &gCamColVars[gCurCamColVars]` = `0x8CC8E0 + (uint8)[0x8CCB80] * 24` (6 floats per row;
  `movzx`, `lea eax,[eax+eax*2]`, `lea ecx,[eax*8+0x8CC8E0]`). `gCurCamColVars` initial .data value is 5. No atexit.

## 0x84D770 __init_g_b6fec0  (catalogue: specified/REA, port level 0)
- `eh_vector_constructor_iterator(0xB6FEC0, 0x9C, 1, ctor 0x5173F0, dtor 0x50E080)`, then `_atexit(__dtor_g_b6fec0 0x8563F0)`.
- The object at 0xB6FEC0 is unnamed in gr. Its size (0x9C) equals `sizeof(CIdleCam)`; the ctor 0x5173F0 is
  `call 0x50E180; mov eax, ecx; ret` (returns ECX as `this`, so it relies on 0x50E180 preserving ECX), the dtor 0x50E080 is
  a bare `ret`. 0x50E180 is uncatalogued-name (catalogue hint "sub_50e180_from_CCam__Process_DW_HeliChaseCam"). Proposed:
  a second CIdleCam-sized helper object used by the DW cinematic cameras (DWCineyCam globals sit just before it at
  0xB6FE8C-0xB6FEBB); class not proven.
- Open question: identity of the object at 0xB6FEC0 and of 0x50E180 (needs that function's spec).

## 0x8563B0 __dtor_gHandShaker  (catalogue: specified/REA, port level 0)
- `eh_vector_destructor_iterator(0xB6ECA0, 0x94, 6, CHandShaker::~CHandShaker 0x509810)`.

## 0x8563D0 __dtor_gIdleCam  (catalogue: specified/REA, port level 0)
- A single `ret`: nothing is destroyed.

## 0x8563E0 __dtor_preMirrorMat  (catalogue: specified/REA, port level 2)
- `mov ecx, 0xB6FE40; jmp CMatrix::~CMatrix 0x59ACD0` (tail call).

## 0x8563F0 __dtor_g_b6fec0  (catalogue: specified/REA, port level 0)
- `eh_vector_destructor_iterator(0xB6FEC0, 0x9C, 1, 0x50E080 /*empty dtor*/)`.

## 0x856410 __dtor_TheCamera  (catalogue: specified/REA, port level 3)
- `mov ecx, 0xB6F028; jmp CCamera::~CCamera 0x50A870` (tail call; the catalogue names 0x50A870 just "CCamera"; its body
  sets the vtable 0x8630E8 and destroys the `m_aCamTweak`/member arrays, i.e. it is the destructor).
