# _unattributed-FA (AUDIO) - RE-SPEC-FINAL group A

## 0x4A9F60 audio entity dtor (vtable 0x859988, CAESound unregister)  (catalogue: unknown/REA, port level 2)
Proposed name: `CAEAudioEntity::~CAEAudioEntity` (inlined copy) (1 eh-unwind caller).
- Convention: `__thiscall void (CAEAudioEntity* this)`. Body: `vtable = 0x859988` (CAEAudioEntity), then tail-jump to
  `CAESound::~CAESound(this + 8)` 0x4EF660 (the m_tempSound member).
- gta-reversed: CAEAudioEntity dtor - matches. Open questions: none.

## 0x4DA510 ped hit volume helper (log10 of table ...)  (catalogue: unknown/REA, port level 0)
Proposed: `static float CAEPedAudioEntity::GetHitVolume(uint8 index)` (only caller CAEPedAudioEntity::HandlePedHit 0x4E1CC0, 2 sites).
- Convention: `__cdecl float (uint8 i [esp+4])`, result ST0, plain `ret`.
- Behaviour: `v = (float)*(int32*)(0x8AD914 + 16*i)` (fild of a dword in a 16-byte-stride table) `* [0x858C58]=0.01f`; x87 `fldlg2; fxch;
  fyl2x` = `log10(v)` (log10(2) * log2(v), extended precision, PC 24-bit); `* [0x858BA4]=20.0f` -> returns 20*log10(v/100) (dB).
  v <= 0 gives -inf/NaN per x87 rules.
- gta-reversed: absent. Open questions: name of the table 0x8AD914 (16-byte records, first dword = percentage).

## 0x4DC550 audio data stream dtor (vtable 0x85f0bc, CloseHandle)  (catalogue: unknown/REA, port level 1)
Proposed: `CAEDataStream::scalar deleting destructor` (vtable 0x85F0BC is CAEDataStream's (dtor 0x4DC490 per gr AEDataStream.cpp:23); this is the
deleting variant with the dtor inlined; no code callers).
- Convention: `__thiscall CAEDataStream* (this, uint8 flags [esp+4])`, `ret 4`, returns this.
- Behaviour: `vtable = 0x85F0BC`; if `m_bIsOpen (+0xC)`: `m_bIsOpen = 0` then `CloseHandle(m_pFileHandle (+4))` (IAT [0x858078]; flag cleared
  BEFORE the call); if `m_pszFilename (+8)` { `operator delete(+8)` 0x8214BD; `+8 = 0` }; if `flags & 1` `operator delete(this)`.
- gta-reversed: AEDataStream.cpp:24 (Close() + delete[]) - equivalent. Open questions: none.

## 0x4E3350 tPedSpeechSlot reset (sub_4e3350)  (catalogue: unknown/REA, port level 0)
- Convention: `__thiscall void (tPedSpeechSlot* this)`, plain `ret`. Callers: CAEPedSpeechAudioEntity::StopCurrentSpeech 0x4E3FB0 (2).
- Behaviour (store order): `+0 (i16) = 0`; `+4 = 0`; `+8 = 0`; `+0xC (i16) = -1`; `+0xE (i16) = -1`; `+0x10 = 0`; `+0x14 (i16) = -1`;
  `+0x16 (i16) = -1`; `+0x19 = 0`; `+0x1A = 0`; `+0x18 = 0` (bytes).
- gta-reversed: tPedSpeechSlot default init (AEPedSpeechAudioEntity.h) - same values. Open questions: none.

## 0x4E34C0 audio entity dtor (vtable 0x859988, CAESound unregister)  (catalogue: unknown/REA, port level 2)
Proposed name: `CAEAudioEntity::~CAEAudioEntity` (inlined copy) (SEH unwind action of CPed::Constructor 0x5E8030 - the CAEPedAudioEntity/ped speech member).
- Convention: `__thiscall void (CAEAudioEntity* this)`. Body: `vtable = 0x859988` (CAEAudioEntity), then tail-jump to
  `CAESound::~CAESound(this + 8)` 0x4EF660 (the m_tempSound member).
- gta-reversed: CAEAudioEntity dtor - matches. Open questions: none.

## 0x4E7B80 CAEMFDecoder helper (sub_4e7b80)  (catalogue: unknown/REA, port level 0)
Proposed: the read callback registered by CAEMFDecoder::Initialise 0x4E7CB0 (address taken there) - `static bool ReadCB(void* out, Decoder* d)`.
- Convention: `__cdecl bool (T** out [esp+4], Decoder* d [esp+8])` (out receives `d`), plain `ret`, AL = d->+0x58.
- Callees: 0x81A3C0 (external decoder API, cdecl, 12 args, returns a 16-bit status in AX).
- Behaviour: if `d->+0x4C >= d->+0x50` (signed; position >= end): `+0x58 = 0` (valid flag), `+0x14 = +0x10 = +0x24 = +0x30 = 0`.
  If `+0x58 == 0`: `*out = d`; return 0. Else call `0x81A3C0(d->+0x48, d->+0x44, d->+0x54, &a, pos (+0x4C), &b, &c, 0, 0, 0, &e, 0)` (12 pushed
  dwords: locals a,b,c,e); if the returned AX != 0 or `c == 0`: clear the same five fields. Then `+0x10 = b * a` (imul of the two locals),
  `+0x4C = e + +0x10`, `+0x24 = c`; if `+0x59` (flag) `+0x30 = c / a` (signed idiv). `*out = d`; return `+0x58`.
- gta-reversed: absent (decoder internals). Open questions: identity/argument meaning of the external call 0x81A3C0 (likely an MP3/WMA
  decoder library entry); exact local mapping of a/b/c/e at 0x4E7BAE-0x4E7BD3 is as listed by stack slot (a = [esp+0x14] after pushes ...)
  - verify against the bytes before porting.

## 0x4F4620 CAEUserRadioTrackManager track-list helper (sub_4f4620)  (catalogue: unknown/REA, port level 7)
Proposed: `std::vector<T*>::push_back(T* const& v)` (MSVC STL instance; `this` = vector {+4 _Myfirst, +8 _Mylast, +0xC _Myend}; caller
CAEUserRadioTrackManager::WriteUserTracksFile 0x4F4690, 2 sites).
- Convention: `__thiscall void (std::vector* this, T* const* v [esp+4])`, `ret 4`.
- Behaviour: `size = first ? (last - first) >> 2 : 0`; if `first != 0 && size < (end - first) >> 2` (unsigned `jae`): construct one element at
  `last` via 0x4F3B30 (cdecl (last, 1, v, this, v) = uninitialized fill of 1 copy) and `last += 4`; else `_Insert_n(this, last, 1, v)` 0x4F4320
  (iterator passed by value in a stack slot).
- gta-reversed: uses std::vector - equivalent. Open questions: none.

## 0x506B20 CAudioEngine member dtor (sub_506b20)  (catalogue: unknown/REA, port level 4)
Proposed: destructor of the CAudioEngine member whose vtable is 0x862E54 (an audio entity owning two CAETwinLoopSoundEntity members at +0x9C
and +0x144; caller CAudioEngine::Destructor 0x506CD0 and 2 unwind sites). Class name UNCLEAR (likely CAEGlobalWeaponAudioEntity-like
"collision/script" entity of CAudioEngine).
- Convention: `__thiscall void (this)`, plain `ret`. SEH frame whose handler address is computed by a HOODLUM chunk (0x401533: `push ecx; pushfd;
  ecx = [0x1560648] ^ [0x6541B2]; popfd; xchg [esp], ecx` -> pushes the handler, 0x83C576 with this image's values) - no behavioural effect.
- Behaviour: `vtable = 0x862E54`; SEH 1; `CAETwinLoopSoundEntity::~CAETwinLoopSoundEntity(this+0x144)` 0x4F2B80; SEH 0;
  `~CAETwinLoopSoundEntity(this+0x9C)`; SEH -1; `vtable = 0x859988` (CAEAudioEntity); `CAESound::~CAESound(this+8)` 0x4EF660.
- gta-reversed: member dtors are compiler-generated - same order (reverse of construction). Open questions: class name.

## 0x506BA0 CAudioEngine member dtor (sub_506ba0)  (catalogue: unknown/REA, port level 3)
Proposed: destructor of the CAudioEngine member with vtable 0x862E58 that embeds two further CAEAudioEntity sub-objects at +0x114 and +0x19C
(caller CAudioEngine::Destructor 0x506CD0 + 2 unwind sites). Class name UNCLEAR.
- Convention: `__thiscall void (this)`, plain `ret`. SEH frame (handler 0x83C596).
- Behaviour: `vtable = 0x862E58`; SEH 1; `+0x19C = 0x859988`; `CAESound::~CAESound(this+0x1A4)`; SEH 0; `+0x114 = 0x859988`;
  `~CAESound(this+0x11C)`; SEH -1; `vtable(+0) = 0x859988`; `~CAESound(this+8)` (0x4EF660 each).
- gta-reversed: compiler-generated - same order. Open questions: class name.

## 0x506C30 audio entity dtor (vtable 0x859988, CAESound unregister)  (catalogue: unknown/REA, port level 2)
Proposed name: `CAEAudioEntity::~CAEAudioEntity` (inlined copy) (SEH unwind action of CAudioEngine ctor/dtor 0x507670/0x506CD0).
- Convention: `__thiscall void (CAEAudioEntity* this)`. Body: `vtable = 0x859988` (CAEAudioEntity), then tail-jump to
  `CAESound::~CAESound(this + 8)` 0x4EF660 (the m_tempSound member).
- gta-reversed: CAEAudioEntity dtor - matches. Open questions: none.

## 0x506C40 audio entity dtor (vtable 0x859988, CAESound unregister)  (catalogue: unknown/REA, port level 2)
Proposed name: `CAEAudioEntity::~CAEAudioEntity` (inlined copy) (SEH unwind action of CAEGlobalWeaponAudioEntity ctor/dtor 0x5075B0/0x506C50).
- Convention: `__thiscall void (CAEAudioEntity* this)`. Body: `vtable = 0x859988` (CAEAudioEntity), then tail-jump to
  `CAESound::~CAESound(this + 8)` 0x4EF660 (the m_tempSound member).
- gta-reversed: CAEAudioEntity dtor - matches. Open questions: none.

## 0x506CC0 audio entity dtor (vtable 0x859988, CAESound unregister)  (catalogue: unknown/REA, port level 2)
Proposed name: `CAEAudioEntity::~CAEAudioEntity` (inlined copy) (SEH unwind action of CAudioEngine ctor/dtor 0x507670/0x506CD0).
- Convention: `__thiscall void (CAEAudioEntity* this)`. Body: `vtable = 0x859988` (CAEAudioEntity), then tail-jump to
  `CAESound::~CAESound(this + 8)` 0x4EF660 (the m_tempSound member).
- gta-reversed: CAEAudioEntity dtor - matches. Open questions: none.
