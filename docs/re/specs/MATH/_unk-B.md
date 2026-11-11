# MATH: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x541E90 CMatrix::Rotate(const CVector&) (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: forwards a vector's three components to CMatrix::Rotate(float, float, float) 0x59B460 with ECX unchanged; only caller CPhysical::PositionAttachedEntity 0x546FF0.
- Convention: `__thiscall void (CMatrix* this = ECX, const CVector* v [esp+4])`, `ret 4`. Behaviour: `Rotate(v->x, v->y, v->z)` (pushed as raw dwords z, y, x). Open questions: none.

## 0x54E8E0 CMatrixLink::~CMatrixLink (proposed)  (catalogue: specified/REA, port level 2)
- Single `jmp CMatrix::~CMatrix 0x59ACD0` (thiscall). Used as element dtor by CMatrixLinkList::Shutdown 0x54E990 and CPlaceable::ShutdownMatrixArray 0x54EFD0, and as unwind action of 0x54EB70. Open questions: none.

## 0x54EB70 CMatrixLinkList::~CMatrixLinkList (proposed)  (catalogue: unknown/REA, port level 3)
- Name evidence: only reference __dtor_gMatrixList 0x856490 (tail call); destroys the six CMatrixLink sentinels built by the ctor 0x54F440.
- Convention: `__thiscall void (this = ECX)`; SEH frame handler 0x83CAFD (states 4..0 = remaining members).
- Behaviour: `CMatrix::~CMatrix` (0x59ACD0) on this+0x1A4, +0x150, +0xFC, +0xA8, +0x54, +0 in that order (reverse declaration order, stride 0x54). Open questions: none.

## 0x54EEA0 TransformVectors  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl void (RwV3d* out [esp+4], int32 count [esp+8], const CMatrix* mat [esp+0xC], const RwV3d* in [esp+0x10])`, caller pops, plain `ret`.
  Entry is `jmp 0x408375` (.HOODLUM stub): the stub decodes the address of CGame::m_pWorkingMatrix1 (0xB72920) from obfuscated words (0x15608C8, rol by a byte from 0x642511) and loads it into EAX, then returns to 0x54EEA5. Net effect: EAX = CGame::m_pWorkingMatrix1 (RwMatrix*).
- Callers: CCamera::CalculateFrustumPlanes 0x514D60 (2 sites), CEntity::GetIsBoundingBoxOnScreen 0x5345D0.
- Callees: CMatrix::CopyToRwMatrix 0x59B8B0 (`mat->CopyToRwMatrix(m_pWorkingMatrix1)`), RwMatrixUpdate 0x7F18A0, RwV3dTransformVector 0x7EDDC0 (`(out, in, matrix)` cdecl).
- Globals: CGame::m_pWorkingMatrix1 0xB72920 (RwMatrix*, read at 0x54EEB0 and once per loop iteration at 0x54EED1).
- Behaviour: copy `mat` into the working RwMatrix; RwMatrixUpdate it; if `count != 0` (test at 0x54EEC3), loop `count` times (dec/jne, so a negative count loops ~2^32 times): `RwV3dTransformVector(out, in, m_pWorkingMatrix1); out += 12 bytes; in += 12 bytes`. No return value.
- gta-reversed: present as a free helper (direction-only transform); port uses the same order. Open questions: none.

## 0x54EF40 CSimpleTransform::UpdateRwMatrix  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (CSimpleTransform* this = ECX, RwMatrix* out [esp+4])`, `ret 4`.
- Callers: 37 (CEntity::UpdateRwMatrix 0x446F90, CObject::ProcessGarageDoorBehaviour, CPickup::GiveUsAPickUpObject, ...). Callee: RwMatrixUpdate 0x7F18A0 (cdecl, popped at 0x54EF89).
- Fields: this+0 CVector m_vPosn, this+0xC float m_fHeading.
- Behaviour (x87 order): `c = cos(heading)` (fld/fcos 0x54EF40); `s = sin(heading)` (heading reloaded from memory, fsin 0x54EF4F). Integer stores: out+8 (right.z), +0x18 (up.z), +0x20, +0x24 (at.x, at.y) = 0; out+0x28 (at.z) = 1.0f (0x3F800000). Then out+0 = c (right.x), out+4 = s (right.y), out+0x10 = -s (up.x), out+0x14 = c (up.y), all f32. out+0x30/+0x34/+0x38 = pos.x/y/z (dword copies). `RwMatrixUpdate(out)`.
- Not written: out+0xC flags (only via RwMatrixUpdate), +0x1C/+0x2C/+0x3C pads.
- gta-reversed: matches SimpleTransform.cpp (UpdateRwMatrix). Open questions: none.

## 0x54F0C0 CMatrixLink::CMatrixLink (proposed)  (catalogue: specified/REA, port level 0)
- Name evidence: element ctor passed by CMatrixLinkList::Init 0x54F0D0 (eh vector ctor over the CMatrixLink array); body is the inline `CMatrix()` ctor (gr Matrix.h: `m_pAttachMatrix(nullptr), m_bOwnsAttachedMatrix(false)`), identical to C3dMarker / CFallingGlassPane ctors.
- Convention: `__thiscall CMatrixLink* (this = ECX)`, returns this, plain `ret`.
- Behaviour: `*(u32*)(this+0x40) = 0; *(u32*)(this+0x44) = 0; return this;` (+0x44 is a full dword store even though gr's m_bOwnsAttachedMatrix is a bool). Open questions: none.

## 0x54F1B0 CSimpleTransform::UpdateMatrix  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (CSimpleTransform* this = ECX, CMatrix* out [esp+4])`, `ret 4`. 114 callers (CPlaceable::GetMatrix 0x411990, ...).
- Callees: CMatrix::SetTranslate 0x59AF40 (`__thiscall (float x, float y, float z)`, pushed z,y,x as raw dwords), CMatrix::SetRotateZOnly 0x59B020 (`__thiscall (float heading)`).
- Behaviour: `out->SetTranslate(pos.x, pos.y, pos.z); out->SetRotateZOnly(this->m_fHeading /* +0xC */);`.
- gta-reversed: matches SimpleTransform.cpp. Open questions: none.

## 0x54F440 CMatrixLinkList::CMatrixLinkList (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller __init_gMatrixList 0x84E300 (static ctor of gMatrixList); gr MatrixLinkList.h: CMatrixLinkList holds 6 CMatrixLink sentinels (m_head, m_tail, m_allocatedListHead, m_allocatedListTail, m_freeListHead, m_freeListTail), each 0x54 bytes; the body is six inlined CMatrixLink/CMatrix ctors.
- Convention: `__thiscall CMatrixLinkList* (this = ECX)`, returns this, plain `ret`.
- Behaviour: zero dwords at +0x40,+0x44, +0x94,+0x98, +0xE8,+0xEC, +0x13C,+0x140, +0x190,+0x194, +0x1E4,+0x1E8 (CMatrix::m_pAttachMatrix / m_bOwnsAttachedMatrix of each link; stride 0x54). Nothing else (m_pLinks at +0x1F8 is left as is; static storage is zero).
- Open questions: none.

## 0x59ACD0 CMatrix::~CMatrix (proposed)  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (CMatrix* this = ECX)`, plain `ret`. 263 callers (direct and EH unwind).
- Behaviour: `if (this->m_bOwnsAttachedMatrix /* dword +0x44, full 32-bit test */ && this->m_pAttachMatrix /* +0x40 */) RwMatrixDestroy(m_pAttachMatrix)` (0x7F2A20, cdecl). Fields are not cleared.
- gta-reversed: gr `CMatrix::~CMatrix()` same. Note +0x44 is tested as a dword. Open questions: none.

## 0x59BCF0 CMatrix::CMatrix(const CMatrix&) (proposed)  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CMatrix* (this = ECX, const CMatrix* src [esp+4])`, `ret 4`, returns this. 27 callers (CVehicleRecording::InterpolateInfoForCar, replay, ...).
- Behaviour: `+0x40 = 0; +0x44 = 0;` then `rep movsd` of 16 dwords: this[0..0x40) = src[0..0x40) (right, flags, up, pad, at, pad, pos, pad copied verbatim). Not an Attach copy: the result never owns or references src's RwMatrix.
- gta-reversed: gr copy ctor uses CopyOnlyMatrix (copies the same 0x40 bytes). Open questions: none.

## 0x59C050 CMatrix::CMatrix(RwMatrix*, bool) (proposed)  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall CMatrix* (this = ECX, RwMatrix* m [esp+4], bool owns [esp+8] (dword pushed through))`, `ret 8`, returns this. 29 callers.
- Behaviour: `this->m_pAttachMatrix = 0` (+0x40, written before the call; +0x44 not pre-cleared); `CMatrix::Attach(this, m, owns)` (0x59BD10); return this.
- gta-reversed: gr `CMatrix(RwMatrix*, bool)` (Matrix.cpp) same. Open questions: none.

## 0x59C790 Multiply3x3(const CMatrix&, const CVector&)  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl CVector* (CVector* ret [esp+4] (hidden), const CMatrix* m [esp+8], const CVector* v [esp+0xC])`, returns the hidden pointer in EAX, plain `ret`. 82 callers.
- Behaviour (exact x87 order, f32 loads, each component stored to a temp then copied): `x = (m[0x20]*v.z + m[0x10]*v.y) + m[0x00]*v.x`; `y = (m[0x24]*v.z + m[0x04]*v.x) + m[0x14]*v.y`; `z = (m[0x28]*v.z + m[0x08]*v.x) + m[0x18]*v.y` (i.e. m.right*v.x + m.up*v.y + m.at*v.z with this summation order per component). All three are computed before out is written, so `out` may alias `v`.
- gta-reversed: replaced by `CMatrix::TransformVector` (Matrix.h:166); summation order must follow the bytes above. Open questions: none.

## 0x59C810 Multiply3x3(const CVector&, const CMatrix&) (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: gr Matrix.h:180 (`Use this instead of Multiply3x3(_VM) (0x59C810)`) - the vector-times-matrix (inverse-rotation) overload.
- Convention: `__cdecl CVector* (CVector* ret [esp+4] (hidden), const CVector* v [esp+8], const CMatrix* m [esp+0xC])`, returns hidden pointer, plain `ret`. 20 callers (CRoadBlocks::CreateRoadBlockBetween2Points 4 sites, scripts, Fx_c, cameras, vehicles).
- Behaviour (x87 order): `x = (m[0x04]*v.y + m[0x08]*v.z) + v.x*m[0x00]` (= dot(v, right)); `y = (m[0x14]*v.y + m[0x10]*v.x) + m[0x18]*v.z` (dot(v, up)); `z = (m[0x24]*v.y + m[0x20]*v.x) + m[0x28]*v.z` (dot(v, at)). All computed before storing.
- gta-reversed: replaced by `CMatrix::InverseTransformVector`; order per bytes. Open questions: none.
