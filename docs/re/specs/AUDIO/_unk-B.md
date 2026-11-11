# AUDIO: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x5B9330 CAESound::CAESound (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: element ctor referenced only by CAESoundManager::Initialise 0x5B9690 (vector construction of the sound array); empty body (`mov eax, ecx; ret`), same bytes as FxMemoryPool_c::FxMemoryPool_c.
- Convention: `__thiscall T* (this = ECX)`, returns this, writes nothing. Open question: element type (CAESound is the likely one; the array size/stride is in the caller 0x5B9690).
