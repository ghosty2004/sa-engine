# CRealTimeShadowManager (RENDER) - full specs

Layouts (gr `RealTimeShadowManager.h`, `RealTimeShadow.h`, `ShadowCamera.h`):
CRealTimeShadowManager (0x54, instance g_realTimeShadowMan): +0 m_bInitialised (u8), +1 m_bNeedsReinit, +4 m_apShadows[16]
(CRealTimeShadow*), +0x44 m_BlurCamera (CShadowCamera: +0 RwCamera*, +4 RwTexture*), +0x4C m_GradientCamera.
CRealTimeShadow (0x4C): +0 m_pOwner (CPhysical*), +4 m_bKeepAlive (u8), +5 m_nIntensity (u8), +8 m_camera (CShadowCamera),
+0x10 m_bBlurred, +0x14 m_blurCamera (CShadowCamera). RwCamera +0x60 frameBuffer (RwRaster*); RwRaster +0xC width (i32).
CPhysical +0x44 m_vecMoveSpeed, +0x134 m_pShadowData; CPed +0x46C m_nPedFlags (byte +0x46D bit 0 = bInVehicle), +0x58C m_pVehicle,
+0x598 m_nPedType (0 = PED_TYPE_PLAYER1).

## 0x706870 CRealTimeShadowManager::ReInit  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall` (ECX = this), no args. Ends with a TAIL JUMP `jmp 0x705D20` (0x706963) with ECX = this+0x4C, so it returns
  whatever CShadowCamera::MakeGradientRaster returns (EAX, RwCamera*); gr declares void (callers ignore EAX).
- Callers: CRealTimeShadowManager::Update 0x706AB0 (only when initialised and m_bNeedsReinit). Callees: RwRasterDestroy 0x7FB020,
  RwRasterCreate 0x7FB230, RwTextureSetRaster 0x7F35D0 (all cdecl; the args of two cameras are popped together by `add esp,0x38`
  at 0x7068EC / 0x706957), CShadowCamera::MakeGradientRaster 0x705D20 (thiscall).
- "Recreate(cam)" below = for a CShadowCamera `sc`, in this exact order:
  `rc = sc.m_pRwCamera; old = rc->frameBuffer (+0x60); w = old->width (+0xC); rc->frameBuffer = NULL; RwRasterDestroy(old);
  r = RwRasterCreate(w, w, 0 /*depth*/, 5 /*rwRASTERTYPECAMERATEXTURE*/); sc.m_pRwCamera->frameBuffer = r (re-read);
  RwTextureSetRaster(sc.m_pRwRenderTexture, r)`. Return values of Destroy/SetRaster are ignored; r is not checked for NULL.
- Behaviour:
  1. For i = 0..15 (counter 16 at [esp+0x10] decremented, 0x70687E/0x7068F2), s = m_apShadows[i] (edi = this+4+4*i):
     Recreate(s->m_camera) (s+8, 0x706888-0x7068B3), then Recreate(s->m_blurCamera) (s+0x14, 0x7068B8-0x7068E3). Both are done
     for every shadow regardless of m_bBlurred / m_pOwner; s is not NULL-checked.
  2. Recreate(m_BlurCamera) (this+0x44, 0x7068FD-0x706924), then Recreate(m_GradientCamera) (this+0x4C, 0x706929-0x706952).
  3. Tail-call `m_GradientCamera.MakeGradientRaster()` (0x70695B-0x706963).
- Edge cases: assumes every camera has an RwCamera with a frame buffer (a NULL anywhere crashes, as in the original). The new raster
  is square with the old raster's width.
- gta-reversed: `plugin::CallMethod` only (RealTimeShadowManager.cpp:66). Open questions: none.

## 0x706970 CRealTimeShadowManager::GetRealTimeShadow  (catalogue: specified/REA, port level 6)
- Convention: `__thiscall` (ECX = this), arg `CPhysical* physical` [esp+4], `ret 4` (0x706A5B). Returns `CRealTimeShadow*` in EAX,
  which can be NULL (gr declares a reference: wrong).
- Callers: CRealTimeShadowManager::DoShadowThisFrame 0x706BA0 (return ignored). Callees: CRealTimeShadow::SetupForThisEntity 0x706520
  (thiscall, arg owner, `ret 4`, returns bool; ignored here).
- Globals: CWorld::PlayerInFocus 0xB7CD74 (u8), CWorld::Players 0xB7CD98 (CPlayerInfo[], stride 0x190, +0 m_pPed). Constant
  f32 [0x85B324] = 0.09 (0x3DB851EC, = 0.3^2).
- Behaviour:
  1. `result = NULL` (esi, 0x70697C), `isPlayer = false` (dl, 0x70697E), `allowed = true` (bl, 0x706983).
  2. If `(physical->+0x36 & 7) == 3` (ped, 0x706980): `isPlayer = (ped->m_nPedType (+0x598) == 0)` (0x706987-0x70698F).
  3. If `!isPlayer` (0x706994): `p = CWorld::Players[PlayerInFocus].m_pPed` (0x706996-0x7069A3; not NULL-checked); if
     `p->+0x46D & 1` (bInVehicle, 0x7069A9) and `v = p->m_pVehicle (+0x58C) != NULL` (0x7069B8): `s2 = (x*x + y*y) + z*z` with
     (x,y,z) = v->m_vecMoveSpeed (+0x44/+0x48/+0x4C) (0x7069BC-0x7069D3, that order); if `s2 > 0.09f` (fcomp, `test ah,0x41; jne`,
     strict; equal/NaN keep allowed) then `allowed = false` (0x7069E8).
     So any non-player-1 entity gets no shadow while the focus player sits in a vehicle moving faster than 0.3 units/step.
  4. If `this->m_bInitialised == 0` (0x7069EA) or `!allowed` (0x7069EF): return NULL.
  5. If isPlayer: `result = m_apShadows[0]` (0x7069F7) - unconditionally, even if slot 0 currently has another owner.
     Else: scan slots 1..15 in increasing order (0x7069FC-0x706A39, 3 iterations of 5 unrolled checks starting at this+8), and for
     each slot whose `m_pOwner == NULL` set `result = slot` - the LAST (highest-index) free slot wins. Slot 0 is never used for
     non-player entities.
  6. If result != NULL (0x706A3B): `result->SetupForThisEntity(physical)` (0x706A42), `physical->m_pShadowData (+0x134) = result`
     (0x706A47), `result->m_bKeepAlive = 1` (0x706A4D), `result->m_nIntensity = 0` (0x706A51).
  7. Return result (0x706A56).
- Edge cases: when the player gets slot 0 while it belongs to another entity, that entity's m_pShadowData still points to slot 0
  (SetupForThisEntity overwrites the owner; no clearing of the previous owner here). The speed gate also applies to non-player peds.
- gta-reversed: `plugin::CallMethodAndReturn` plus an unfinished commented-out sketch (RealTimeShadowManager.cpp:120-137) that is wrong
  in three ways: it returns when `m_bInitialised` is TRUE (the original returns NULL when it is false), the speed gate is applied to
  `!IsTypePed() || IsPlayer()` (the original applies it to everything except player-1 peds), and it returns when speed^2 < 0.09 (the
  original refuses when speed^2 > 0.09). The gr-deviations row (cpp:131, "Maybe wrong?", deviates=no) concerns that dead sketch only.
  Open questions: none.

## 0x7067A0 CRealTimeShadowManager::~CRealTimeShadowManager  (catalogue: unknown/REA, port level 4)
- `__thiscall void (this)`: `CShadowCamera::Destroy(this+0x4C)` 0x705400, then tail-jump `CShadowCamera::Destroy(this+0x44)`. Global dtor of
  g_realTimeShadowMan (0x856AD0). gta-reversed: member dtors - same order. Open questions: none.
