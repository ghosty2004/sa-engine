# SCRIPT: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x6F9EB0 CScriptedEffectPairs::Reset (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: called by the pairs ctor 0x6FA790, CScripted2dEffects::Init 0x6FA6F0 and AddScripted2DEffect 0x6FA7C0; rewrites the four 0x24-byte pair records at this+4 to the state their ctor 0x6FA770 creates, then clears the count.
- Convention: `__thiscall void (this = ECX)`, plain `ret`.
- Behaviour: for each pair p at this+4+k*0x24 (k=0..3): dwords p+0, p+4, p+0xC, p+0x10, p+8, p+0x14, p+0x1C = -1, byte p+0x20 = 0 (p+0x18 untouched, as in the ctor); finally `dword this+0 = 0`. Stores are fully unrolled (0x6F9EB5..0x6F9F1B).
- Open question: as for 0x6FA770, the meaning of the untouched +0x18 of each pair.

## 0x6FA6D0 CScripted2dEffects ms_userLists element ctor (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only reference __init_CScripted2dEffects__ms_userLists 0x8545B0 (element ctor of the static user-list array; gr ScriptedEffects/Scripted2dEffects.h `ms_userLists`).
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: for i = 0..3: `dword[this + i*4] = -1; dword[this + 0x10 + i*4] = -1`; then `byte +0x20 = 0`. Open questions: gr struct name for this element (tUserList_c?) not confirmed.

## 0x6FA770 CScriptedEffectPair::CScriptedEffectPair (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: element ctor (size 0x24, count 4) used by 0x6FA790 for CScripted2dEffects::ms_effectPairs.
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: dwords +0, +4, +0xC, +0x10, +8, +0x14, +0x1C = -1 (that store order); byte +0x20 = 0. +0x18 is not written. Open questions: meaning of the +0x18 gap (no write at 0x6FA783/0x6FA786).

## 0x6FA790 CScriptedEffectPairs::CScriptedEffectPairs (proposed)  (catalogue: unknown/REA, port level 1)
- Name evidence: only reference __init_CScripted2dEffects__ms_effectPairs 0x8545E0.
- Convention: `__thiscall T* (this = ECX)`, returns this. Behaviour: `eh_vector_constructor_iterator(this+4, 0x24, 4, 0x6FA770, 0x6F9E30 /*empty dtor*/)`; then `0x6F9EB0(this)` (thiscall helper also used by CScripted2dEffects::Init/AddScripted2DEffect; resets the pair set). Open questions: 0x6F9EB0 not specified in this batch.
