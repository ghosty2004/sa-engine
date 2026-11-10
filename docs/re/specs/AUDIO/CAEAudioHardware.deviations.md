# CAEAudioHardware (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Hardware/AEAudioHardware.cpp` (pin 20d8d2a). Instance AEAudioHardware 0xB5F8B8.
Fields used: +2/+3 u8 m_PrevMaxSecondary/GlobalSlowFadeout, +6 u16 m_awChannelFlags[64], +0x8E u16 m_nNumChannels,
+0x90 u16 m_anNumChannelsInSlot[64], +0x110 f32 m_afChannelVolumes[64], +0x210 f32 m_ChannelGains[64], +0x310 f32
m_afChannelsFrqScalingFactor[64], +0x410 MusicMaster, +0x414 EffectMaster, +0x418 MusicFader, +0x41C EffectsFader,
+0x420 NonStreamFader, +0x424 StreamFader, +0x428 m_PrevMaxVolumeSecondary, +0x42C m_PrevMaxVolumeGlobal, +0xD98
CAEMP3BankLoader*, +0xDA0 IDirectSound8*, +0xDA8 m_n3dEffectsQueryResult, +0xE0C IDirectSound3DListener*, +0xE64
CAEAudioChannel* m_aChannels[64].

## 0x4D86B0 CAEAudioHardware::PlaySound  (gr_deviations = 3)
Original: `__thiscall void (this; i16 channel [esp+4], u16 slot [+8], i16 soundIdInSlot [+0xC], i16 bankSlot [+0x10],
i16 playPos [+0x14], u16 flags [+0x18], f32 speed [+0x1C])`, `ret 0x1C`.
Row AEAudioHardware.cpp:266 `assert(false)` (channel+slot range): deviates=no.
- Original bytes: the range check is `cmp cx,0x40; ja return` (0x4D871C) - index 64 (== MAX_NUM_AUDIO_CHANNELS) is ACCEPTED and
  reads `m_aChannels[64]` (= +0xF64, the first dword of gBeatInfo). gr rejects `>= 64`.
- Verdict: CONFIRMED (off-by-one in the original's bound; gr is stricter). Class: keep-original (index 64 reads +0xF64 as a
  channel pointer).
Row AEAudioHardware.cpp:279 `NOTSA_UNREACHABLE("Setting audio buffer failed!")`.
- Original bytes: `SetAudioBuffer` 0x4F0C40 returning 0 just returns (0x4D8750 `je 0x4D8800`), no abort.
- Verdict: CONFIRMED (gr aborts/logs in debug). Class: keep-original (silent return).
Row AEAudioHardware.cpp:288 `assert(std::in_range<int16>(length))`: deviates=no.
- Original bytes: `len = channel->GetLength()` (vtable +0x10, full EAX, 0x4D8765); clamp is `if ((i32)(i16)pos > (i32)len)
  pos = len` (0x4D87A6-0x4D87AD) and `pos` is then pushed as a full dword to Play - when `len > 32767` the full 32-bit
  length is passed; gr truncates via `std::min<int16>`.
- Verdict: CONFIRMED (only for lengths > 32767 ms). Class: keep-original.
Found in passing (CONFIRMED, keep-original):
- Start-percentage (flags & 8 and pos > 0): `pos = _ftol(floorf(pos * 0.01f [0x858C58] * len))`: x87 `fild pos;
  fmul 0.01f; fimul len`, stored f32 as the arg of 0x406CE0 (floorf), then `_ftol` 0x821B40 (0x4D8782-0x4D879B). gr uses
  `(float)pos / 100.0f * (float)len` (division instead of multiply by 0.01f - can differ in the last bit).
- gr omits the two stores after Play: `m_awChannelFlags[channel + slot] = flags` (u16, 0x4D87F3) and
  `m_aChannels[channel + slot]->m_nFlags (+0x28) = (i32)(i16)flags` (0x4D87FD). These feed RescaleChannelVolumes.
  gr logic error - the port must keep them.
- Play is the virtual slot +0x14 called with `(pos as dword, flags as full dword, speed)` (0x4D87E7).

## 0x4D8990 CAEAudioHardware::RescaleChannelVolumes  (gr_deviations = 6)
Original: `__thiscall void (this = ECX)`, plain `ret`. Three loops over `i = 0 .. m_nNumChannels-1` (u16 at +0x8E).
Rows 506, 514, 515, 530, 532, 552 are gr comments claiming bit-exactness. Checked:
- Row 506 (pow in double): original `fld qword 10.0 [0x859EA8]; fld vol(f32 mem); fmul 0.05f [0x858C28]; call 0x822130`
  (CRT `_CIpow`, x87 in/out), then `fst gain` (0x4D8B58-0x4D8B6B). Verdict: REFUTED (gr matches). BUT: for FLAG_UNDUCKABLE
  (flags & 2) the original adds the UNROUNDED ST0 pow result to fullSum (`fadd fullSum` at 0x4D8B76 right after `fst`), gr adds
  the f32-rounded `m_ChannelGains[i]` - CONFIRMED minor precision difference (keep-original). Other branches add the f32 value
  from memory (0x4D8BC8 / 0x4D8C0C).
- Row 514 (GetPlayTime twice): original calls vtable +0xC (GetPlayTime) at 0x4D8B9A, +0x10 (GetLength) 0x4D8BA9, +0xC again
  0x4D8BBB; the check is `test eax,eax; jl` on the full EAX. Verdict: REFUTED. Class: harmless.
- Row 515 (`static_cast<int16>` length): `movsx ebp, ax` at 0x4D8BB2. The second GetPlayTime result is used as full EAX
  (`sub ebp, eax`, 0x4D8BBE, no sign-extension). Verdict: REFUTED (same while GetPlayTime returns a properly extended int16).
- Row 530 (operation order): `fld fullSum; fmul 0.8f [0x858C98]; fsubr 6.4f [0x85F080]; fmul 16383.0f [0x85F07C]; fdiv
  scaleSum` then `if !(x <= 16383)` (fcomp `test ah,0x41`) x = 16383 (0x4D8C79-0x4D8CA4). Verdict: REFUTED (gr matches,
  including NaN keeping x).
- Row 532 (`scaleSum == 0`): `fcomp 0.0; test ah,0x44; jp` (0x4D8C44-0x4D8C6D): exactly-equal gives 0, NaN computes.
  Verdict: REFUTED.
- Row 552 (`gain == 0`): `test ah,0x44; jp` at 0x4D8D20: equal -> SetVolume(-100.0f) 0x4D7C60; NaN/other -> `SetVolume(
  AudioLog10(gain * 6.103888e-05f [0x85F078]) * 20.0f [0x858BA4])` (0x4D8D40-0x4D8D60). Verdict: REFUTED. Note the factor order
  of the music/effects scaling: `fld +0x418; fmul +0x410` (fader * master) then `fmul gain` - commutative products, same result.
Class for all six: harmless (gr already reproduces them), plus the one keep-original precision note above.

## 0x4D9640 CAEAudioHardware::InitDirectSoundListener  (gr_deviations = 1)
Original: `__thiscall bool (this; u32 numChannels [esp+4], u32 samplesPerSec [+8], u32 bitsPerSample [+0xC])`, `ret 0xC`.
Row AEAudioHardware.cpp:200 (`#ifdef FIX_BUGS soundbuf->Release()` when SetFormat fails).
- Original bytes: SetFormat (vtable +0x38) failure jumps to 0x4D96F8 and returns 0 without releasing the primary buffer
  (0x4D96F6). Only the QueryInterface failure path releases it (0x4D9727).
- Verdict: CONFIRMED (gr deviates only when built with FIX_BUGS; the default gr path matches). Class: keep-original (leak).
Found in passing: `nBlockAlign = (u16)((bits >> 3) * numChannels)` (`shr ax,3; imul eax,ecx`, 0x4D96B6/0x4D96D3), gr
`numChannels * bits / 8` - equal for multiples of 8 (always 16 in practice). `nAvgBytesPerSec = (u16)nBlockAlign * sps`.
