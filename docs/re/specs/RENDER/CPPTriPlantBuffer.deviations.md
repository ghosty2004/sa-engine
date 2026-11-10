# CPPTriPlantBuffer (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/PPTriPlantBuffer.cpp` (pin 20d8d2a). Layout: +0 m_CurrentIndex (i32), +4 m_Buffer (PPTriPlant,
stride 0x5C), +0xB84 m_PlantModelsSet, +0xB88 m_pPlantModelsTab[4].

## 0x5DB0C0 CPPTriPlantBuffer::Flush  (gr_deviations = 1)
Original: `__thiscall void ()` (ECX = this), plain `ret` (0x5DB126). Entry goes through a .text detour (0x5DB0C9 -> 0x409BF4 ->
0x5DB0CE) that only loads `eax = m_PlantModelsSet`.
Row PPTriPlantBuffer.cpp:33 ("FIX_BUGS: m_pPlantModelsTab[m_PlantModelsSet] can be null. With std::array we check it.").
- Original bytes: if `m_CurrentIndex <= 0` (signed, 0x5DB0C6) return. `tab = (m_PlantModelsSet <= 3 unsigned) ? m_pPlantModelsTab[set]
  : NULL` (jump table 0x5DB128 / `xor edi,edi` 0x5DB0FC); `r = rand()` (CRT 0x821B1E, 0x5DB0FE); `CGrassRenderer::DrawTriPlants(this+4,
  m_CurrentIndex, tab)` (0x5DB10F, cdecl, 3 args); `m_CurrentIndex = 0` (0x5DB115); `srand(r)` (0x821B11, 0x5DB11B).
- Verdict: REFUTED - gr's GetPlantModelsTab(set) (0x5DACA0: `set < 4 ? tab[set] : NULL`) is the same as the inlined original, and no
  extra NULL check exists in the gr code despite the comment; call order (rand, draw, clear, srand) is identical. Class: harmless.
  Note: the save/restore `r = rand(); ...; srand(r)` is what neutralises DrawTriPlants' own `srand` on the global CRT RNG (see
  CGrassRenderer.deviations.md); ports must keep both.
