# CCollisionData: gta-reversed deviation notes (COLL)

Rows from `catalog/gr-deviations.csv`, checked against the original bytes (RE-SPEC-1, writer B).

## 0x40f6c0 CCollisionData::SetLinkPtr  (gr_deviations = 1)
- CollisionData.cpp:210 "Original calculation method:" (gr computes the address with `std::align(4, 4,
  &m_pTrianglePlanes[m_nNumTriangles], space = 10)` instead of `(addr + 3) & ~3`).
  - Original bytes: stub 0x40f6c0 -> 0x1568990: `eax = movsx word [this+4]` (m_nNumTriangles, SIGN-extended);
    `edx = (([this+0x1c] + eax*10 + 3) & ~3)`; `[edx] = arg [esp+4]`; `ret 4`.
  - Verdict: REFUTED. `std::align` with 4-byte alignment and space 10 yields the same rounded-up address as
    `(addr + 3) & ~3`. Side note (not this row): gr declares `m_nNumTriangles` as `uint16` and indexes with it
    zero-extended; the original sign-extends (`movsx`). Only differs for >= 0x8000 triangles (never in data);
    a port should sign-extend to match.
  - Class: harmless.

## 0x40f6e0 CCollisionData::GetLinkPtr  (gr_deviations = 2)
- CollisionData.cpp:221 "Original calculation method:"
  - Original bytes: 0x156d850..0x156d863: same address computation as SetLinkPtr (movsx of +4, `*10`, `+3`,
    `& ~3`), returns `[edx]` in EAX, `ret` (no args).
  - Verdict: REFUTED (same value as gr's `std::align`; same signedness side note as above).
  - Class: harmless.
- CollisionData.cpp:228 "Sanity check" (`assert(l->data == this)`)
  - Original bytes: no check of any kind; the load at 0x156d861 is returned directly.
  - Verdict: REFUTED (an assert is not a release-build behavioural difference; the port must not add a check).
  - Class: harmless.
