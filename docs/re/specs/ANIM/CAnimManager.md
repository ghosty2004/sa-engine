# CAnimManager (ANIM) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x4D3FD0 CAnimManager::RemoveAnimBlockRef  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void(int32 index [esp+4])`, plain `ret`.
- Callers: 38 (CAtomicModelInfo::DeleteRwObject 0x4C4440, CClumpModelInfo::DeleteRwObject 0x4C4E70,
  CAnimBlendAssociation dtor 0x4CECF0, InteriorGroup_c::DereferenceAnims 0x595160, CPlayerPed::ProcessAnimGroups 0x6098F0, ...).
- Globals: `CAnimManager::ms_aAnimBlocks` 0xB5D4A0 (CAnimBlock[], stride 0x20); field +0x12 `m_RefCnt` (u16).
- Behaviour: `ms_aAnimBlocks[index].m_RefCnt -= 1` (`dec word [0xB5D4A0 + index*32 + 0x12]`, 0x4D3FD4-0x4D3FDC).
  No range check, no underflow check (0 wraps to 0xFFFF).
- gta-reversed: same (inline `--ms_aAnimBlocks[index].RefCnt`).

## 0x4D40C0 CLinkList<CAnimBlendHierarchy*>::Insert (proposed; catalogue: unnamed, unknown/REA, port level 0)
- Name evidence: only caller CAnimManager::UncompressAnimation 0x4D41C0 (2 sites), with ECX = `CAnimManager::ms_AnimCache`
  (gr `StaticRef<CLinkList<CAnimBlendHierarchy*>>(0xB5EB20)`); the body is gr `CLinkList<T>::Insert(T const&)`
  (`Core/LinkList.h`). Layout: CLink<T> = {data +0, prev +4, next +8} (0xC); list: usedListHead +0x00,
  usedListTail +0x0C, freeListHead +0x18, freeListTail +0x24.
- Convention: `__thiscall CLink<T>*(T const* data [esp+4])`, `ret 4`, return EAX (null on full).
- Behaviour:
  1. `link = this->freeListHead.next` (`[ecx+0x20]`); if `link == &this->freeListTail` (`ecx+0x24`) return 0 (0x4D40FB).
  2. `link->data = *data` (dword copy, 0x4D40CA-0x4D40D0).
  3. Remove: `link->next->prev = link->prev` (0x4D40D2-0x4D40D9); `link->prev->next = link->next` (0x4D40DC-0x4D40E2).
  4. Insert after usedListHead: `link->next = head.next` (0x4D40E8); `head.next->prev = link` (0x4D40EE);
     `link->prev = &head` (0x4D40F1); `head.next = link` (0x4D40F4). Return link.
- gta-reversed: `CLinkList<T>::Insert(T const&)` - same order.
