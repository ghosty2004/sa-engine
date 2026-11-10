# CAEStreamThread (AUDIO) - full specs (RE-SPEC-3, group U1)

## 0x4F11D0 CAEStreamThread::CAEStreamThread  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CAEStreamThread* (this = ECX)`, returns this (`mov eax, ecx; ret`). Caller:
  CAEAudioHardware::Constructor 0x4D83E0. Behaviour: none (no field is initialised).
- gta-reversed: default ctor (gr may have member initialisers; the original writes nothing). Open questions: none.
