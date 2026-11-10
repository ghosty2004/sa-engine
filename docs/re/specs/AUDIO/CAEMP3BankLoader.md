# CAEMP3BankLoader (AUDIO) - full specs (RE-SPEC-3, group U1)

Inherits CAEBankLoader: +0 CAEBankSlot* m_BankSlots (stride 0x12D4; +0x14 CAEBankSlotItem Sounds[400], item stride 0xC:
+0 BankOffsetBytes, +4 LoopStartOffset, +8 u16 SampleFrequency, +0xA i16 Headroom), +0x14 u8 m_IsInitialised.

## 0x4E0380 CAEMP3BankLoader::GetLoopOffset  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall i32 (this = ECX; u16 soundId [esp+4], i16 bankSlot [esp+8])`, `ret 8`. Caller:
  CAEAudioHardware::PlaySound 0x4D86B0.
- Behaviour: if `+0x14 == 0` return -1 (0x4E0387). Else return the dword at `m_BankSlots + (i16)bankSlot * 0x12D4 +
  ((u16)soundId + 2) * 12` = `m_BankSlots[slot].Sounds[soundId].LoopStartOffset` (0x4E038F-0x4E03AA). soundId is
  zero-extended, bankSlot sign-extended; no bounds checks.
- gta-reversed: same (its assert row :105 is debug-only, harmless). Open questions: none.
