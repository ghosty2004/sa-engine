# C3dMarker (RENDER) - full specs

C3dMarker (0xA0 in gr, `3dMarker.h`): +0x00 CMatrix m_Mat (0x48; CMatrix +0x40 m_pAttachMatrix RwMatrix*, +0x44
m_bOwnsAttachedMatrix), +0x48 m_Atomic, ... Array `C3dMarkers::m_aMarkerArray` is built by `__init_C3dMarkers__m_aMarkerArray`
0x855310 (see `_static-init-R1.md`).

## 0x720F60 C3dMarker::C3dMarker  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (this = ECX: C3dMarker*), no stack args, plain `ret`; returns `this` in EAX (0x720F60 `mov eax, ecx`).
- Callers: `__init_C3dMarkers__m_aMarkerArray` 0x855310 (as the element ctor passed to the array-construct helper). Callees: none.
- Behaviour: this is the inlined default `CMatrix()` of `m_Mat` only: `[this+0x40] = 0` (0x720F64, m_pAttachMatrix),
  `[this+0x44] = 0` (0x720F67, m_bOwnsAttachedMatrix, written as a full dword). Nothing else is initialised.
- Edge cases: all other fields stay as they were (static storage, so zero at startup).
- gta-reversed: `C3dMarker() = default` with in-class initialisers (3dMarker.h: m_Color 255x4, m_PulsePeriod 1024, m_RotateRate 5,
  m_PulseFraction 0.25f, m_StdSize/m_Size/m_Brightness 1.0f, m_LastMapReadX/Y 30000, m_RoofHeight 65535.0f, ...). DIFFERENCE:
  the original ctor writes none of those; a port must only zero +0x40/+0x44. Open questions: none.

## 0x720F70 C3dMarker::~C3dMarker  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall` (ECX = C3dMarker*), tail jump `jmp 0x59ACD0` (0x720F70) = CMatrix::~CMatrix on `m_Mat` (offset 0).
- Callers: `__init_C3dMarkers__m_aMarkerArray` 0x855310 (dtor argument of the array helper), `__dtor_C3dMarkers__m_aMarkerArray` 0x856BD0.
- Behaviour (0x59ACD0): if `[this+0x44] != 0` (owns) and `[this+0x40] != 0` -> `RwMatrixDestroy([this+0x40])` (0x7F2A20, cdecl,
  popped by `pop ecx`). Nothing else; no RpAtomic is destroyed here.
- gta-reversed: `= default` (calls ~CMatrix, same). Open questions: none.
