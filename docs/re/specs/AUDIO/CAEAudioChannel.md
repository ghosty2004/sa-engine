# CAEAudioChannel (AUDIO) - full specs (RE-SPEC-3, group U1)

Layout (gr AEAudioChannel.h, VALIDATE_SIZE 0x60): +0 vptr, +4 IDirectSound*, +8 IDirectSoundBuffer*, +0xC
IDirectSound3DBuffer*, +0x28 u32 m_nFlags, +0x34 f32 m_Volume, +0x38 u8 m_bPaused, +0x3A u16 m_nChannelId, +0x3C u32
m_nFrequency, +0x40 u32 m_nOriginalFrequency, +0x44 u8 m_bLooped, +0x45 u8 m_bShouldStop, +0x47 WAVEFORMATEX (packed:
+0x47 wFormatTag, +0x49 nChannels, +0x4B nSamplesPerSec, +0x4F nAvgBytesPerSec, +0x53 nBlockAlign, +0x55 wBitsPerSample,
+0x57 cbSize), +0x5C u32 m_nBufferStatus. Base vtable 0x85F03C. COM Release = vtable slot +8, `__stdcall (this pushed)`.
Global `g_numSoundChannelsUsed` 0xB5F898 (i32).

## 0x4D7890 CAEAudioChannel::CAEAudioChannel  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CAEAudioChannel* (this = ECX; IDirectSound* ds [esp+4], u16 channelId [+8], u32 samplesPerSec
  [+0xC], u16 bitsPerSample [+0x10])`, returns this in EAX, `ret 0x10`. Callers: CAEStaticChannel ctor 0x4F0B10,
  CAEStreamingChannel ctor 0x4F1800.
- Behaviour (store order as in bytes): `+0x55 = bits` (0x4D789C); `+4 = ds`; `+0x3A = channelId`; `+0x4F = (bits >> 3) *
  samplesPerSec` (bits zero-extended, `shr 3; imul`, 0x4D78B3); `+0 = 0x85F03C`; `+0x28 = 0`; `+0x5C = 0`; `+0x3C =
  samplesPerSec`; `+0x57 = 0` (u16); `+0x53 = 2` (u16); `+0x49 = 1` (u16); `+0x4B = samplesPerSec`; `+0x40 = samplesPerSec`;
  `+0x47 = 1` (u16, WAVE_FORMAT_PCM); `+0x34 = -100.0f` (imm 0xC2C80000); `+8 = 0`; `+0xC = 0`; `+0x38 = 0`; `+0x44 = 0`;
  `+0x45 = 0`. Fields not listed (+0x10..+0x27, +0x2C, +0x30, +0x39, +0x46) are not written.
- gta-reversed: same values. Open questions: none.

## 0x4D7910 CAEAudioChannel::~CAEAudioChannel  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (this = ECX)`, plain `ret`. Callers: CAEStaticChannel dtor 0x4F0B50, CAEStreamingChannel
  dtor 0x4F2200 (also from their EH unwind funclets).
- Behaviour: `+0 = 0x85F03C` (0x4D7918); if `+8 != 0`: `--g_numSoundChannelsUsed` (0x4D7920). If `+8 != 0` (re-read):
  `(+8)->Release()` (vtable +8, 0x4D7930). `+8 = 0` (unconditional). If `+0xC != 0`: `(+0xC)->Release()`. `+0xC = 0`.
- gta-reversed: matches. Open questions: none.

## 0x4D7C00 CAEAudioChannel::~CAEAudioChannel (scalar deleting destructor)  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall CAEAudioChannel* (this = ECX, u32 flags [esp+4])`, returns this in EAX, `ret 4`. No direct
  callers (vtable slot 0 of the base vtable 0x85F03C).
- Behaviour: the body of 0x4D7910 inlined (same order, 0x4D7C00-0x4D7C3C), then if `flags & 1`: `_free(this)` 0x8214BD
  (cdecl, caller pops). Returns this.
- gta-reversed: compiler-generated (virtual dtor). Open questions: none.

## 0x4F1020 CAEAudioChannel::~CAEAudioChannel (scalar deleting destructor of CAEStaticChannel)  (catalogue: specified/REA, port level 2)
- Name correction: this is the deleting destructor of the derived CAEStaticChannel (it calls the CAEStaticChannel
  destructor 0x4F0B50, which installs vtable 0x85F3CC); the catalogue's class name is the base.
- Convention: `__thiscall CAEStaticChannel* (this = ECX, u32 flags [esp+4])`, returns this, `ret 4`. No direct callers
  (vtable 0x85F3CC slot 0).
- Behaviour: call 0x4F0B50 (this); if `flags & 1`: `_free(this)` 0x8214BD. Return this.
- gta-reversed: compiler-generated. Open questions: none.
