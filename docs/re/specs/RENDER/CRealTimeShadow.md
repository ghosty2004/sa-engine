# CRealTimeShadow (RENDER) - full specs

CRealTimeShadow (0x4C, gr `RealTimeShadow.h`): +0x00 m_pOwner (CPhysical*), +0x04 m_bKeepAlive, +0x05 m_nIntensity,
+0x08 CShadowCamera m_camera (0x8: +0 RwCamera*, +4 RwTexture*), +0x10 m_bBlurred, +0x14 CShadowCamera m_blurCamera,
+0x1C m_nBlurPasses, +0x20 m_bDrawMoreBlur, +0x24 m_nRwObjectType (u32), +0x28 m_pLight, +0x2C CSphere m_boundingSphere
(center +0x2C/+0x30/+0x34, radius +0x38), +0x3C CSphere m_baseSphere (center +0x3C..+0x44, radius +0x48).

## 0x7059E0 CRealTimeShadow::GetShadowCamera  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall` (ECX = CRealTimeShadow*), `lea eax, [ecx+8]; ret` (0x7059E0): returns `&m_camera` (CShadowCamera*).
  Always the non-blurred camera, regardless of m_bBlurred.
- Caller: CShadows::CastShadowEntityXYZ 0x70A040. gta-reversed: declared only (no body in RealTimeShadow.cpp). Open questions: none.

## 0x706520 CRealTimeShadow::SetupForThisEntity  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall bool (CPhysical* owner [esp+4])` (this = ECX), returns AL (1/0), `ret 4` (0x7065ED / 0x7065FB).
- Callers: CRealTimeShadowManager::GetRealTimeShadow 0x706970.
- Callees: RpClumpGetBoundingSphere 0x734FC0 (cdecl (RpClump*, RwSphere*, bool useLTM=1)), _rpAtomicResyncInterpolatedSphere
  0x7491F0 (cdecl (RpAtomic*)), RwV3dTransformPoints 0x7EDD90 (cdecl (out, in, count, RwMatrix*)), CShadowCamera::SetFrustum
  0x7054C0 (thiscall (float) ret 4), CShadowCamera::SetCenter 0x705590 (thiscall (RwV3d*) ret 4), CRealTimeShadow::Destroy 0x705990.
- Constants: 0x858F14 = 1.1f.
- Fields: owner +0x18 m_pRwObject (RwObject*: byte +0 type, 1 = atomic, 2 = clump; +4 parent RwFrame*, frame +0x10 = modelling
  matrix). RpAtomic +0x1C boundingSphere (16 bytes), +0x4C interpolator flags (bit 1 = dirty sphere).
- Behaviour:
  1. `m_pOwner = owner` (0x706528), unconditionally, before any check.
  2. `obj = owner->m_pRwObject` (0x70652A). If null -> return 0 (0x7065F7), WITHOUT calling Destroy and without writing +0x24.
  3. `m_nRwObjectType = (u32)obj->type` (byte zero-extended, 0x70653A). If type == 0 or type > 2 (0x70653D `jbe`, 0x706546 `ja`)
     -> `Destroy()` (0x7065F2) and return 0.
  4. type 1 (atomic, 0x70657D): if `byte [obj+0x4C] & 2` -> `_rpAtomicResyncInterpolatedSphere(obj)`. Copy the 16 bytes
     obj+0x1C..0x2B into m_boundingSphere (+0x2C..+0x3B) (0x706592-0x7065A7).
     type 2 (clump, 0x706553): `RpClumpGetBoundingSphere(obj, &m_boundingSphere, 1)`.
     Both: `m_baseSphere.radius (+0x48) = m_boundingSphere.radius (+0x38)` (raw dword copy, 0x70655F/0x7065AA), then
     `RwV3dTransformPoints(&m_baseSphere.center (+0x3C), &m_boundingSphere.center (+0x2C), 1, &frame->modelling)` with
     frame = `[obj+4]`, matrix = frame + 0x10 (0x706565/0x7065B0) - the frame's MODELLING matrix, not its LTM.
  5. `m_camera.SetFrustum(m_boundingSphere.radius * 1.1f)` (0x7065C6 fld +0x38, fmul [0x858F14], fstp to the f32 arg; ECX = this+8).
  6. `m_camera.SetCenter(&m_baseSphere.center)` (ECX = this+8, arg this+0x3C, 0x7065E3). Return 1 (0x7065EA).
- Edge cases: always uses m_camera (+8), never m_blurCamera. On the null-RwObject path m_pOwner stays set and the shadow is
  not destroyed (caller-visible). The clump sphere comes from RpClumpGetBoundingSphere with its third argument 1.
- gta-reversed: declared in RealTimeShadow.h, no body (absent). Open questions: none.
