# CAEPedlessSpeechAudioEntity (AUDIO) - full specs (RE-SPEC-3, group U1)

## 0x4E6080 CAEPedlessSpeechAudioEntity::AddScriptSayEvent  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (this; 5 dword args: event, scriptId, overrideSilence, isForceAudible, isFrontEnd)`, body is a
  single `ret 0x14` (pops 5 args). Virtual override (reached from CPed::SayScript 0x5EFFB0 through the vtable).
- Behaviour: none. gta-reversed: `/* NOP */` override - same. Open questions: none.
