# CCutsceneObject (ANIM) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x5B19D0 CCutsceneObject::CCutsceneObject  (catalogue: specified/REA, port level 15)
- Convention: `__thiscall CCutsceneObject*()`, returns this, plain `ret`; SEH frame (unwinds CObject on exception).
  Caller: CCutsceneMgr::CreateCutsceneObject 0x5B02A0.
- Behaviour (order of stores):
  1. `CObject::CObject()` 0x5A1D10. `vtable = 0x868A60`.
  2. `m_vWorldPosition (+0x184..+0x18C) = 0`, `m_vForce (+0x190..+0x198) = 0` (6 dwords).
  3. `byte[+0x36] = (byte[+0x36] & 7) | 0x10` (keep entity type bits, status := 2 STATUS_SIMPLE); `m_nObjectType (+0x13C, u8) = 4`
     (OBJECT_TYPE_CUTSCENE); `+0x17C = 0` (m_nAttachBone / m_pAttachTo union); `+0x180 m_pAttachmentObject = 0`;
     `m_nFlags (+0x1C) = (flags & ~1) | 0x10000400` (clear m_bUsesCollision, set m_bStreamingDontDelete bit 10 and m_bLightObject bit 28)
     (0x5B1A1E-0x5B1A50).
  4. `CObject::SetIsStatic(false)` 0x5A0760.
  5. `m_nFlags &= ~0x08000000` (m_bBackfaceCulled off); `m_fContactSurfaceBrightness (+0x12C) = 0.5f`.
- gta-reversed: `CutsceneObject.cpp:22` - same stores (order differs only between independent fields).

## 0x5B1A80 CCutsceneObject::~CCutsceneObject  (catalogue: unknown/REA, port level 15)
- Convention: `__thiscall void (this)`. `vtable = 0x868A60` (CCutsceneObject), then tail-jump to CObject::~CObject 0x59F660. Caller: the scalar
  deleting destructor 0x5B1DE0.
- gta-reversed: CCutsceneObject has no user dtor (`= default`) - matches. Open questions: none.

## 0x5B1A90 CCutsceneObject::SetupCarPipeAtomicCB  (catalogue: unknown/REA, port level 3)
- `__cdecl RpAtomic* (RpAtomic* a [esp+4], void* data)`, plain `ret`: `CCarFXRenderer::CustomCarPipeAtomicSetup(a)` 0x5D5B20; return a.
  Caller SetupCarPipeAtomicsForClump 0x5B1AB0. gta-reversed: same. Open questions: none.

## 0x5B1DE0 CCutsceneObject::`scalar deleting destructor'  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall CCutsceneObject* (this, uint8 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `~CCutsceneObject()` 0x5B1A80; if `flags & 1` call the CObject pool `operator delete(this)` 0x5A1F20 (cdecl). Return this.
- gta-reversed: compiler-generated - matches. Open questions: none.
