# CPlantLocTri (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Plant/PlantLocTri.cpp` (pin 20d8d2a). CPlantLocTri: +0 m_V1, +0xC m_V2, +0x18 m_V3, +0x24 m_Center,
+0x30 m_SphereRadius, +0x34 m_Seed[3], +0x40 m_nMaxNumPlants[3] (u16), +0x46 m_SurfaceId, +0x47 m_nLighting, +0x48 flags (bit0 createsPlants,
bit1 createsObjects, bit2 createdObjects).

## 0x5DC290 CPlantLocTri::Add  (gr_deviations = 1)
Original: `__thiscall CPlantLocTri* (const CVector* p1, const CVector* p2, const CVector* p3, uint8 surface, tColLighting light,
bool createsPlants, bool createsObjects)`, `ret 0x1C`; returns this or NULL (0x5DC4FE).
Row PlantLocTri.cpp:50 (`rng::generate(m_Seed, [] { return CGeneral::GetRandomNumberInRange(0.0f, 1.0f); })`).
- Original bytes (0x5DC458-0x5DC4A9): three times, in order m_Seed[0], [1], [2]: `m_Seed[k] = (float)rand() * 3.0518509e-05f` (CRT rand
  0x821B1E, fild, f32 [0x858C7C] = 0x38000100 = 1/32767, stored f32). The u16 densities are stored between the third rand and the third
  seed store (0x5DC496-0x5DC49E) - no observable effect.
- Verdict: CONFIRMED - gr's GetRandomNumberInRange(0, 1) uses its own formula (cat14 finding 1), not `rand() * (1/32767)`. Class: keep-original.
Found in passing (CONFIRMED, keep-original, rounding-level): `m_Center = ((p2 + p1) + p3) * 0.33333334f` per component (f32 [0x859040] =
0x3EAAAAAB, 0x5DC3AB-0x5DC3CA; gr divides by 3.0f); `m_SphereRadius = sqrt((dz*dz + dy*dy) + dx*dx) * 1.75f` with d = center - V1
([0x862E2C], 0x5DC3CD-0x5DC400). Densities: `n_k = (u16)_ftol(area * props->density_k)` with density_k at props +0x20/+0x48/+0x70
(0x5DC423-0x5DC43E, area from GetPlantDensity 0x5DC210 left in ST0 and reused); list slot `props->m_SlotId` (u16 at +0).
Flag byte +0x48 = `(createsObjects & 1) << 1 | (old & 0xF8) | (createsPlants & 1)` (0x5DC2F0-0x5DC314).
