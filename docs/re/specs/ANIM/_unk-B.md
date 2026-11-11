# ANIM: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x617850 IKChain_c::IKChain_c (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: element ctor referenced only by IKChainManager_c::IKChainManager_c 0x617FC0 (IKChain_c array; gr IKChainManager_c.h `IKChain_c m_IKChains[32]`); body is the ListItem_c base ctor only.
- Convention: `__thiscall IKChain_c* (this = ECX)`, returns this. Behaviour: `ListItem_c::ListItem_c(this)` (0x4A8DB0). Open questions: none.
