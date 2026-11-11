# CORE: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x54FA10 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FA60 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FAB0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FB00 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FB50 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FBA0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FBF0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FC40 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FC90 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FCE0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FD30 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FD80 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FDD0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FE20 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FE70 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FEC0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x54FF10 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5501E0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5502B0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550380 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550450 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550520 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5505D0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5506A0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550750 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550820 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5508F0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5509C0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550A90 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550B60 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550C30 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550D00 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550DD0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x550EA0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8213AE, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5512F0 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x551350 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x5513B0 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x551410 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x551470 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x5514D0 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x551530 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x551590 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x5515F0 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x551650 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x5516B0 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x551710 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x551770 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x5517D0 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x551830 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x551890 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x5518F0 CPool<T>::`scalar deleting destructor' (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: inlined CPool<T>::~CPool / Flush (as in the family 0x54FA10, 0x5A6E60) followed by the MSVC delete-flag test; no callers or data references, so T is unknown.
- Convention: `__thiscall CPool* (this = ECX, u32 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `if (m_nSize > 0) { if (m_bOwnsAllocations (byte +0x10)) { free(m_pObjects); free(m_byteMap); } zero +0, +4, +8, +0xC; } if (flags & 1) operator delete(this); return this;` Open questions: none.

## 0x552380 CPtrNodeSingleLink::operator new (proposed)  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl void* (size_t)` (size ignored), result EAX. Body: `ECX = CPools::ms_pPtrNodeSingleLinkPool [0xB74484]; jmp CPtrNodeSingleLinkPool::New 0x552240` (tail call; New is thiscall without stack args).
- Callers: CPtrListSingleLink::AddItem 0x5335E0, CQuadTreeNode<T>::AddItem 0x552CD0. gta-reversed: gr `CPtrNodeSingleLink::operator new` (PtrNodeSingleLink.h) same. Open questions: none.

## 0x552390 CPtrNodeSingleLink::operator delete (proposed)  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (void* p [esp+4])`, plain `ret`. Entry `nop; jmp 0x409B8D` (.HOODLUM stub): the stub decodes the pool-pointer address (not/xor of obfuscated dwords 0x15608F0 / 0x6C8D62) and loads ECX = *that address = ms_pPtrNodeSingleLinkPool (0xB74484, by symmetry with 0x552380; the decoded address itself is not proven statically - open question 0x409B9C), then jumps back to 0x552396.
- Callers: CPtrListSingleLink::DeleteItem 0x533610 (2), CEntity::Remove 0x534AE0.
- Behaviour (inline CPool::Delete, element size 8): `idx = (p - pool->m_pObjects) >> 3` (sar, signed); `pool->m_byteMap[idx] |= 0x80` (free flag); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (signed compare, 0x5523AF).
- gta-reversed: same as CPool::Delete. Open question: confirm the .HOODLUM-decoded address equals 0xB74484 (0x409B8E..0x409B9C).

## 0x5523C0 CPtrNodeDoubleLink::operator new (proposed)  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl void* (size_t)` (size ignored). Body: `ECX = ms_pPtrNodeDoubleLinkPool [0xB74488]; jmp CPtrNodeDoubleLinkPool::New 0x5522E0`.
- Callers: CPtrListDoubleLink::AddItem 0x533670, CEntity::CreateRwObject, CEntity::AttachToRwObject, CPhysical::RemoveAndAdd, ... (8). Open questions: none.

## 0x5523D0 CPtrNodeDoubleLink::operator delete (proposed)  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (void* p [esp+4])`, plain `ret`. Globals: ms_pPtrNodeDoubleLinkPool 0xB74488 (read).
- Behaviour (inline CPool::Delete, element size 12): `idx = (p - pool->m_pObjects) / 12` computed as signed magic divide (imul 0x2AAAAAAB, sar edx 1, add sign bit; 0x5523DD..0x5523EE); `m_byteMap[idx] |= 0x80`; `if (idx < m_nFirstFree) m_nFirstFree = idx`.
- Callers: CPtrListDoubleLink::DeleteItem 0x5336B0, CEntity::Remove, CPhysical::Remove, CPhysical::RemoveAndAdd, ... (7). Open questions: none.

## 0x552400 CPtrListSingleLink::Flush  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (CPtrListSingleLink* this = ECX)`, plain `ret`. Node layout: +0 item, +4 next (8 bytes).
- Callers: CEntryExitManager::FindNearestEntryExit/Update, CQuadTreeNode dtor 0x552520, 0x43E1A0 (tail), ... (7). Globals: ms_pPtrNodeSingleLinkPool 0xB74484.
- Behaviour: `node = head; while (node) { next = node->next;` inline DeleteNode: `if (head == node) head = next; else { walk p = head while p && p->next != node; if found p->next = node->next; }` then inline `delete node` (CPool::Delete, elem 8: `idx=(node-objs)>>3; map[idx] |= 0x80; if idx < firstFree: firstFree = idx`); `node = next; }`. The pool pointer is re-read every iteration (0x55243B).
- gta-reversed: equivalent to gr `CPtrListSingleLink::Flush` (DeleteNode loop). Open questions: none.

## 0x552470 CPtrListDoubleLink::Flush  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (CPtrListDoubleLink* this = ECX)`, plain `ret`. Node layout: +0 item, +4 next, +8 prev (12 bytes).
- Callers: CGame::Shutdown, CWorld::RemoveStaticObjects, CWorld::ShutDown (7 sites), sector dtors 0x5630F0/0x563130, ... (9). Globals: ms_pPtrNodeDoubleLinkPool 0xB74488.
- Behaviour: `node = head; while (node) { next = node->next; if (head == node) head = next; if (node->prev) node->prev->next = node->next; if (node->next) node->next->prev = node->prev;` then inline CPool::Delete (elem 12, magic-divide index, `map |= 0x80`, firstFree min) `; node = next; }`.
- gta-reversed: equivalent to gr `CPtrListDoubleLink::Flush`. Open questions: none.

## 0x552830 CQuadTreeNode::CQuadTreeNode  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CQuadTreeNode* (this = ECX, const CRect* rect [esp+4], int32 level [esp+8])`, `ret 8`, returns this.
- Callers: CIplStore::Initialise 0x405EC0, CColStore::Initialise 0x4113F0, CEntryExitManager::Init 0x43F880 (after `new` 0x552C80).
- Behaviour: copy rect dwords +0..+0xC (left, bottom, right, top as stored); `+0x10 = 0` (m_ItemList head); `+0x24 = level`; `+0x14..+0x20 = 0` (m_apChildren[4]).
- gta-reversed: matches QuadTreeNode.h:88. Open questions: none.

## 0x552B50 CPool<CQuadTreeNode>::New (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: tail target of CQuadTreeNode::operator new 0x552C80 with ECX = CQuadTreeNode::ms_pQuadTreeNodePool; returns `m_pObjects + idx*0x28` (sizeof CQuadTreeNode = 0x28, see ctor 0x552830). Byte-identical to 0x5A7350 (COctTree pool). Out-of-line `CPool<T>::New()` (gr Pool.h).
- Convention: `__thiscall T* (CPool* this = ECX)`, plain `ret`, returns EAX (null when full). Pool layout: +0 m_pObjects, +4 m_byteMap (u8 each: bit7 free, bits0-6 id), +8 m_nSize, +0xC m_nFirstFree.
- Behaviour: `wrapped = false; do { firstFree++; if (firstFree == size) { firstFree = 0; if (wrapped) return null; wrapped = true; } } while (!(map[firstFree] & 0x80));` then `map[ff] &= 0x7F; map[ff] = (map[ff] & 0x80) | ((map[ff] + 1) & 0x7F)` (id increment, bit7 kept; 0x552B8C..0x552BA2); `return objs + ff*0x28`.
- Edge: on the failing path m_nFirstFree is left at 0 (stored before the wrapped test at 0x552B66). Compare is equality only (`firstFree == size`).
- gta-reversed: CPool::New equivalent. Open questions: none.

## 0x552C80 CQuadTreeNode::operator new (proposed)  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl void* (size_t)` (size ignored). Body: `ECX = CQuadTreeNode::ms_pQuadTreeNodePool [0xB745BC]; jmp 0x552B50` (CPool<CQuadTreeNode>::New).
- Callers: CIplStore::Initialise, CColStore::Initialise, CEntryExitManager::Init. Open questions: none.

## 0x5617C0 GetMillisecondTime  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl uint64 ()`, returns EDX:EAX. Used as a function pointer by CTimer::Initialise 0x5617E0 (ms_fnGetTimer-style).
- Behaviour: `call RsTimer 0x619410` (u32 ms in EAX); `EDX = 0`; return. Open questions: none.

## 0x5A6E60 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8214BD, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5A72B0 CPool<T>::~CPool (unreferenced instantiation, proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: body is gr Pool.h `CPool<T>::Flush()` as inlined into `~CPool()`; no callers or data references (an instantiation the linker kept), so T is unknown. Part of a family of byte-identical copies.
- Convention: `__thiscall void (CPool* this = ECX)`, plain `ret`. Pool layout: +0 m_pObjects, +4 m_byteMap, +8 m_nSize, +0xC m_nFirstFree, +0x10 bool m_bOwnsAllocations.
- Behaviour: `if (m_nSize > 0 /*signed*/) { if (m_bOwnsAllocations) { free(m_pObjects); free(m_byteMap); } m_pObjects = 0; m_byteMap = 0; m_nSize = 0; m_nFirstFree = 0; }` (CRT free at 0x8214BD, cdecl, two args popped together). Open questions: none (port: generic CPool drop, no per-address code needed).

## 0x5A7570 COctTreeBase::COctTreeBase  (catalogue: specified/REA, port level 0)
- gr OctTreeBase.h:17 (`// 0x5A7570`). Only caller __init_gOctTreeBase 0x84FE20. Convention: `__thiscall COctTreeBase* (this = ECX)`, returns this.
- Behaviour (store order): dword +4 = 0; +0x1C, +0x20, +0x24 = 0; dwords at +0xA, +0xE, +0x12, +0x16 = -1 (unaligned dword stores covering bytes 0xA..0x19: gr COctTree::m_aChildrens[8] int16 = -1); byte +8 = 0; vtable +0 = 0x867178.
- Edge: bytes +9 and +0x1A/+0x1B are not written. gta-reversed: gr default member initialisers equivalent. Open questions: none.

## 0x619440 RsCameraShowRaster  (catalogue: specified/REA, port level 4)
- gr platform.cpp:68. Single `jmp Ps::psCameraShowRaster 0x745240`: same args (`__cdecl RwCamera* (RwCamera*)` per RsCameraShowRaster) and return. 8 callers (App::DoRWStuffEndOfFrame, Game::FrontendIdle, Game::Idle, ...). Open questions: none.

## 0x7170C0 CRGBA::CRGBA (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: out-of-line `CRGBA(uint8 r, uint8 g, uint8 b, uint8 a)` (gr RGBA.h); 85 callers (CDarkel::DrawMessages, CGarages::PrintMessages, CPickups::RenderPickUpText, HUD 58, ...).
- Convention: `__thiscall CRGBA* (this = ECX, u8 r [esp+4], u8 g [esp+8], u8 b [esp+0xC], u8 a [esp+0x10])` (each in a dword slot, low byte used), `ret 0x10`, returns this.
- Behaviour: `this[0]=r; this[1]=g; this[2]=b; this[3]=a` (byte stores). Open questions: none.

## 0x72FC70 CameraSize  (catalogue: specified/REA, port level 3)
- gr app_camera.cpp:53 (gr has a NOTSA_WINDOWED_MODE branch; the bytes are the plain path below).
- Convention: `__cdecl void (RwCamera* camera [esp+4], RwRect* rect [esp+8], float viewWindow [esp+0xC], float aspectRatio [esp+0x10])`. Callers: App::DoRWStuffStartOfFrame, DoRWStuffStartOfFrame_Horizon, Game::FrontendIdle, ... (5).
- Callees: RwEngineGetCurrentVideoMode 0x7F2D20, RwEngineGetVideoModeInfo 0x7F2CF0 (into a local RwVideoMode {width, height, depth, flags}), RwCameraSetViewWindow 0x7EE410.
- Globals written: RsGlobal.maximumWidth 0xC17044, RsGlobal.maximumHeight 0xC17048.
- Behaviour: `if (!camera) return;` get `vm` for the current mode; `fb = camera->frameBuffer (+0x60); w = fb->width (+0xC); h = fb->height (+0x10)`. If `rect == null`: use a local rect `{x=0, y=0, w,h}` where `(w,h) = (vm.flags & 1 /*rwVIDEOMODEEXCLUSIVE*/) ? (vm.width, vm.height) : (fb width, fb height)`. If `fb->width != rect->w || fb->height != rect->h`: `zb = camera->zBuffer (+0x64)` (no null check); `zb->width = fb->width = rect->w; zb->height = fb->height = rect->h` (store order zb.w, fb.w, zb.h, fb.h).
- View window `vw`: if `vm.flags & 1`: `vw.x = viewWindow; vw.y = viewWindow / aspectRatio`. Else if `rect->w > rect->h` (signed int compare): `vw.x = viewWindow; vw.y = (float)rect->h * viewWindow / rect->w` (fild h, fmul, fidiv w); else `vw.y = viewWindow; vw.x = (float)rect->w * viewWindow / rect->h`. `RwCameraSetViewWindow(camera, &vw)`; then `RsGlobal.maximumWidth = rect->w; RsGlobal.maximumHeight = rect->h`.
- Edge: in exclusive (fullscreen) mode the aspect ratio argument is used; in windowed mode it is ignored. Open questions: none.
