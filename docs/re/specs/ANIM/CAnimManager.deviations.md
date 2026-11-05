# CAnimManager: gta-reversed deviation notes (ANIM)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x4D3CC0 CAnimManager::CreateAnimAssocGroups  (gr_deviations = 1)
Row AnimManager.cpp:320 (`#ifdef SA_SKINNED_PEDS` around the AtomicRemoveAnimFromSkinCB pass, lines 320-324).
- Original bytes: for i in 0..ms_numAnimAssocDefinitions-1 (0xB4EA28; def = 0x8AA5B8 + i*0x30, group = ms_aAnimAssocGroups + i*0x14):
  block = first of ms_numAnimBlocks blocks with `stricmp(block.Name, def.BlockName) == 0` (inlined GetAnimationBlock, 0x4D3CF4-0x4D3D10);
  skip if none, if `!block.IsLoaded (+0x10)` or `group.m_Anims (+0x4) != 0`. clump = 0; if `def.ModelIndex (+0x10) != -1`:
  `clump = ms_modelInfoPtrs[idx]->CreateInstance()` (vtable +0x2C) + `RpAnimBlendClumpInit(clump)`. `group.m_GroupID (+0x10) = i`;
  `group.m_IdOffset (+0xC) = def.AnimDescr[0].AnimId`; `group.CreateAssociations(def.BlockName, clump, def.AnimNames (+0x18), def.NumAnims (+0x14))`
  (0x4CE6E0); for j < group.m_NumAnims (+0x8, unsigned): `group.GetAnimation(def.AnimDescr[j].AnimId)->m_Flags (+0xA) |= AnimDescr[j].Flags (w)`.
  If clump: **if `GetFirstAtomic(clump)` and `RpSkinGeometryGetSkin(atomic->geometry)` non-null: `RpClumpForAllAtomics(clump,
  AtomicRemoveAnimFromSkinCB 0x734B90, 0)`** (0x4D3DE7-0x4D3E11); then `RpClumpDestroy(clump)`.
- Verdict: REFUTED - the bytes contain the SA_SKINNED_PEDS branch; gr matches the original only when SA_SKINNED_PEDS is defined.
- Class: harmless for gr's default build (port must always include the skinned branch).

## 0x4D41C0 CAnimManager::UncompressAnimation  (gr_deviations = 1)
Original bytes (`__cdecl void(CAnimBlendHierarchy* h [esp+4])`; hierarchy +0xA m_bIsCompressed, +0xB m_bKeepCompressed, +0x10 m_fTotalTime,
+0x14 m_Link; cache `ms_AnimCache` 0xB5EB20 = CLinkList: usedHead +0x0, usedTail +0xC, freeHead +0x18, freeTail +0x24):
  1. if `h->m_bKeepCompressed`: if `h->m_fTotalTime == 0.0f` (exact; NaN no) tail-jump `CalcTotalTimeCompressed` 0x4CF3E0; return.
  2. else if `!h->m_bIsCompressed`: if `h->m_Link`: unlink it and re-insert after usedHead (inline, 0x4D41F2-0x4D421D); return.
  3. else: `l = ms_AnimCache.Insert(&h)` (0x4D40C0). If null: `t = usedTail.prev` (0xB5EB30); `t->data->RemoveUncompressedData()`
     (0x4CF760); unlink `t` and insert it after freeHead (0xB5EB38) (0x4D4245-0x4D426F); `t->data->m_Link = 0`; `l = Insert(&h)`
     again (not checked). `h->m_Link = l`; tail-jump `h->Uncompress()` 0x4CF5F0.
Row AnimManager.cpp:404 (`assert(!h->m_Link); // Sanity check`):
- Verdict: REFUTED (debug assert, no release behaviour; the rest of gr's branch is equivalent - gr removes via `llr->data->m_Link`,
  the original unlinks the tail link itself, which is the same link when the cache is consistent).
- Class: harmless.

## 0x4D47F0 CAnimManager::LoadAnimFile  (gr_deviations = 1)
Row AnimManager.cpp:631 (`default: NOTSA_UNREACHABLE("Unknown ANPK: {:x}", h.ID)`).
- Original bytes (`__cdecl void(RwStream* s [ebp+8], bool loadCompressed, const char (*uncompressed)[32])`): read the 8-byte section header
  (`RwStreamRead(s, &h, 8)`, 0x4D4817-0x4D481F); `if (strncmp(h.ID, "ANP2", 4) == 0 || strncmp(h.ID, "ANP3", 4) == 0)` -> ANP2/3 loader
  at 0x4D5172 (which sets its "is ANP3" flag by a further `strncmp(.., "ANP3", 4)`, 0x4D5172-0x4D518F); **every other ID, "ANPK" or not,
  falls through to the ANPK loader** (0x4D485C onwards: size rounded up to a multiple of 4, then the INFO chunk etc.). There is no
  "ANPK" comparison at all.
- Verdict: CONFIRMED (gr aborts/UB for an unknown ID; the original parses it as ANPK).
- Class: keep-original.
