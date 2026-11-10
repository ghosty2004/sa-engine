# CPedGeometryAnalyser (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

Budget note: these four functions (226, 883, 26 and 1900 instructions) were NOT verified against the bytes in this pass. Every row below is UNCLEAR
pending a byte check; the port must follow the original, so do not take gr's bug-fix branches. Rows (gr file source/game_sa/PedGeometryAnalyser.cpp):

## 0x5F1890 CPedGeometryAnalyser::ComputeRouteRoundSphere  (gr_deviations = 2)
- cpp:1024 `notsa::bugfixes::CPedGeometryAnalyser_ComputeRouteRoundSphere_IncorrectDetourPosition`: UNCLEAR (not checked). Class: keep-original (original = bugfix off).
- cpp:1036 NOTE/BUG (Pirulax) (deviates=no): UNCLEAR (not checked).
- cpp:1041 `..._AmbigousDetourTarget` bugfix: UNCLEAR (not checked). Class: keep-original (original = bugfix off).

## 0x5F1FA0 CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached  (gr_deviations = 3)
- cpp:314 BUGFIX, cpp:318 BUGFIX, cpp:391 "differs a bit from the original code ... bb's two axes": UNCLEAR (not checked). Class: keep-original.
- cpp:340 "original code ... simplified", cpp:350 assert, cpp:409 grid vectors (classified harmless): UNCLEAR (not checked).

## 0x5F3B70 CPedGeometryAnalyser::ComputeClosestSurfacePoint-ped  (gr_deviations = 1)
- cpp:229 `bugfixes::GenericUB: return (!GenericUB || computed) && ComputeClosestSurfacePoint(...)`: UNCLEAR (not checked; the original presumably ignores the
  `computed` result - verify at the call of ComputeEntityBoundingBoxCorners inside 0x5F3B70). Class: keep-original.

## 0x5F3DD0 CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox-2  (gr_deviations = 2)
- cpp:727 "For completeness, the original code:" (maybe), cpp:871 assert (harmless), cpp:995 `default: NOTSA_UNREACHABLE_CASE(forceDirection)` (maybe): UNCLEAR (not checked).
