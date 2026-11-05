# IKChainManager_c (ANIM) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x617FC0 IKChainManager_c::IKChainManager_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall IKChainManager_c*()`, returns `this` in EAX, plain `ret`. SEH frame (handler 0x83F061) for unwinding
  the members if a later constructor throws. Caller: __init_g_ikChainMan 0x851B50.
- Behaviour:
  1. `eh_vector_constructor_iterator(this, 0x58, 32, ctor 0x617850, dtor 0x617860)` (0x617FD7-0x617FEC): constructs
     `m_IKChains[32]` (IKChain_c, 0x58 each). 0x617850 = IKChain_c ctor = `ListItem_c::ListItem_c` (0x4A8DB0) only;
     0x617860 = jmp 0x4A8DC0 (ListItem_c dtor).
  2. `TList_c ctor` 0x4A8DD0 on `this+0xB00` (m_ActiveList) then on `this+0xB0C` (m_FreeList) (0x617FF1-0x61800F). 0x4A8DD0
     zeroes three dwords (+0 head, +4 tail, +8 count).
  Free-list filling is NOT done here (gr: in Init).
- gta-reversed: `IKChainManager_c() = default` (`Ragdoll/IKChainManager.h:14`), same effect.
