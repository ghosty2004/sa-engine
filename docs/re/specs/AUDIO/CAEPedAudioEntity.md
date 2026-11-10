# CAEPedAudioEntity (AUDIO) - full specs (RE-SPEC-3, group U1)

Layout (gr AEPedAudioEntity.h, size 0x15C): CAEAudioEntity base (+0 vptr 0x86C2A8, +4 m_Entity, +8 CAESound m_tempSound;
CAESound offsets: +0xC m_Event, +0x10 m_ClientVariable, +0x14 m_Volume, +0x18 m_RollOffFactor, +0x1C m_Speed, +0x20
m_SpeedVariance, +0x56 u16 m_Flags, so entity+0x14 = Event, +0x18 = ClientVariable, +0x24 = Speed, +0x28 = SpeedVariance),
+0x7C u8 m_bCanAddEvent, +0x7E i16 m_nSfxId, +0x80 u32 m_LastSwimWakeTriggerTimeMs, +0x84/+0x88/+0x8C f32 m_fVolume1..3,
+0x90 f32 m_JetPackSoundSpeedMult, +0x94 CPed* m_pPed, +0x98 u8 m_bJetPackPlaying, +0x9C/+0xA0/+0xA4 CAESound* jet-pack sounds,
+0xA8 CAETwinLoopSoundEntity (vptr 0x85F438; its m_IsInUse at +0x130, m_Sounds at +0x148/+0x14C), +0x150 CAESound*,
+0x154/+0x158 f32. Ped fields: +0x14 m_matrix (pos +0x30) / +4 placement pos (GetPosition, no allocation), +0x5A0 CWeapon
m_aWeapons[13] (stride 0x1C, +0 type), +0x718 i8 m_nActiveWeaponSlot.
Common callees: CAESound::Initialise 0x4EFE50 (thiscall, 12 args: bankSlot, sfx, entity, CVector pos (3 dwords), volume,
rollOff, speed, doppler, u8 frameDelay, u16 flags, f32 speedVariance, i16 playTime; callee pops), RegisterWithPhysicalEntity
0x4EF820, SetFlags 0x4EF2B0 (u16 flag, u16 enable), AESoundManager(0xB62CB0)::RequestNewSound 0x4EFB10,
AEAudioHardware(0xB5F8B8)::IsSoundBankLoaded 0x4D88C0 (u16 bank, i16 slot), LoadSoundBank 0x4D88A0 (bank, slot),
AudioEngine(0xB6BC90)::IsLoadingTuneActive 0x506D90. `evVol(e)` = `(f32)(i8)m_pAudioEventVolumes[e]` (*(i8**)0xBD00F8, fild).
"delay stamp" = `(f32)(u32)(CTimer::m_snTimeInMilliseconds + delay)` (fild, + 4294967296.0f [0x858C54] if the sum is negative
as i32).

## 0x4E1A40 CAEPedAudioEntity::HandlePedSwing  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void (this = ECX; i32 event [esp+4], i32 hitEvent [esp+8], u32 delay [esp+0xC])`, `ret 0xC`.
  (gr names: `(event, a3, volume)` - the third arg is a time delay, not a volume.) Caller: AddAudioEvent 0x4E2BB0 (AE_PED_SWING).
- Behaviour:
  1. `w = &ped->m_aWeapons[(i8)ped->+0x718]` (0x4E1A48-0x4E1A59); `sfx = -1`, `volOff = 0.0f` ([esp+0x14]), `speed = 1.0f`.
     If `w == null` return (never in practice).
  2. If `!IsSoundBankLoaded(0x8F, 5)`: if `AudioEngine.IsLoadingTuneActive()` return; else `LoadSoundBank(0x8F, 5)`; return.
  3. `V = evVol(event)` (x87 ST0, 0x4E1AD4). Weapon type `t = w->type` (u32): if `t > 0x2D` -> no change (sfx -1).
     Table (byte 0x4E1C7C / dword 0x4E1C60):
     - 2, 15: sfx 0x2F; 5, 6: sfx 0x56, speed 0.67f (0x3F2B851F); 3, 7: sfx 0x56, speed 0.84f (0x3F570A3D);
       4, 8: sfx 0x58, volOff -3.0f [0x859024]; 10..13: sfx 0x56, volOff -6.0f [0x85A5C0], speed 0.84f;
       19, 20, 21, 35, 36: unchanged (sfx -1, volOff 0);
       all others 0..45 (0, 1, 9, 14, 16-18, 22-34, 37-45): sfx 0x57, volOff -6.0f, speed 0.84f.
  4. `hitEvent - 0x40` (table 0x4E1CAC): 0x40/0x41/0x42 -> sfx 0x57, volOff -6.0f, speed 0.84f; 0x43/0x44 -> sfx 0x57, volOff
     -2.0f [0x858B18], speed 0.84f (these replace the weapon choice and skip the next test). Otherwise: if `(i16)sfx < 0` return.
  5. `pos = ped->GetPosition()`; `vol = V + volOff` (x87 `fxch; fadd st1`, stored f32);
     `m_tempSound.Initialise(5, sfx, this, pos, vol, 1.0f, speed, 1.0f, 0, 0x80, 0.04f (0x3D23D70A), 0)`;
     `m_tempSound.RegisterWithPhysicalEntity(ped)`.
  6. If `delay != 0` (unsigned): `m_tempSound.m_SpeedVariance = 0`, `.m_Speed = 0` (0x4E1C0B/0x4E1C0E), `.m_ClientVariable =
     delay stamp(delay)`, `.m_Event = event`, `SetFlags(4, 1)`.
  7. `RequestNewSound(&m_tempSound)`.
- gta-reversed: `plugin::CallMethod` only. Open questions: none.

## 0x4E1CC0 CAEPedAudioEntity::HandlePedHit  (catalogue: specified/REA, port level 7)
- Convention: `__thiscall void (this = ECX; i32 event [esp+4], CPhysical* other [esp+8], u8 surfaceId [esp+0xC], f32 volume
  [esp+0x10], u32 delay [esp+0x14])`, `ret 0x14`. Caller: AddAudioEvent 0x4E2BB0 (events 0x3D..0x44). The body passes
  through the .text stub 0x4067B5 (`cmp word [esp+0x34],0; jmp 0x4E1EBA`) - it tests sfx1 (below), same behaviour inline.
- Extra callees: GetRandomNumberInRange<int32> 0x4D9C10 (`Rnd`), 0x4DA510 (cdecl `f32 (u8 surface)` -> ST0, unnamed collision
  helper), AudioEngine.ReportCollision 0x506EB0 (thiscall, 10 args: `(CEntity* a, CEntity* b, u8 surfA, u8 surfB, CVector* pos,
  CVector* normal, f32 impulse, f32 relSpeed, bool, bool)`).
- Locals: `volA = 0.0f`, `volB = -3.0f`, `slot2 = 5`, `spVar1 = 0.0588f` (0x3D70D845), `base`; `sfx1` reuses the `volume` arg
  slot [esp+0x34]; `sfx2` = EBX.
- Behaviour:
  1. Weapon `w` and the bank check exactly as HandlePedSwing (bank 0x8F slot 5; IsLoadingTuneActive; LoadSoundBank; return).
  2. `base = evVol(event) + volume` (fild; fadd arg; stored f32, 0x4E1D61-0x4E1D6E).
  3. `sfx1` by weapon type `t` (table 0x4E2268 / 0x4E2234; `t > 0x2D` -> -1): 0, 44, 45 -> 0x3A; 1, 16-18, 22-34, 37-43 -> 0x2C;
     2 -> 0x30; 3 -> 0x23; 4 -> 0x39 and `spVar1 = 0.03f`; 5 -> 0x22; 6 -> 0x4B; 7 -> 0x43; 8 -> 0x38; 10-13 -> 0x32;
     14 -> 0x36 and `spVar1 = 0.07f`; 15 -> 0x2E; 9, 19, 20, 21, 35, 36 -> -1.
  4. If `other != null && (other->+0x36 & 7) == 3` (a ped), switch `event - 0x3D` (table 0x4E2298; > 7 -> `sfx2 = -1`, go to 6):
     - 0x3D: by `t - 1` (0..13, table 0x4E22D0; else default): 1 -> `volA = 1.0f`, `sfx2 = Rnd(40, 43)`; 5 -> `volA = -10.0f`,
       `sfx2 = Rnd(40, 43)`; 4, 8 -> `sfx2 = 0x51`, `volB = -9.0f`, go to 7 (second sound only); 9 -> CHAINSAW (below);
       14 -> `sfx2 = -1`; default (2, 3, 6, 7, 10-13, 0, >14) -> `volA = -15.0f`, `sfx2 = Rnd(40, 43)`. Then step 6.
     - 0x3E: same structure with table 0x4E22F8 but `Rnd(36, 39)` (1 -> volA 1.0f, 5 -> volA -10.0f, default -> volA -15.0f).
     - 0x3F: by `t - 4` (0..10, table 0x4E231C; else default): 4, 8 -> 0x51/volB -9 (go to 7); 9 -> CHAINSAW; 14 -> -1;
       5 -> `volA = -10.0f, sfx2 = 0x52, volB = -6.0f`; default (6, 7, 10-13 and t < 4 or > 14) -> `volA = -15.0f, sfx2 = 0x52,
       volB = -6.0f`. Then step 6.
     - 0x40: `sfx2 = 0x52, volB = -6.0f`, go to 7.
     - 0x41, 0x43: `sfx1 = 0x3A`, `sfx2 = Rnd(40, 43)`, go to 6b (first sound played without the sign test).
     - 0x42: `sfx1 = 0x3A`, `sfx2 = Rnd(36, 39)`, go to 6b.
     - 0x44: `sfx2 = Rnd(59, 62)`, go to 6c.
     CHAINSAW (0x4E1F17): if `!IsSoundBankLoaded(0x1B, 3)` return; `slot2 = 3`; `sfx2 = Rnd(7, 9)`; go to 6c.
  5. Otherwise (no ped hit, 0x4E1FCA): `sfx2 = -1`; switch `event - 0x3D` (table 0x4E2328; > 7 -> step 6):
     - 0x3D/0x3E/0x3F: `volA = 0x4DA510(surfaceId)` (ST0 stored f32); `ReportCollision(ped, other, 0x3E, surfaceId, &pedPos, NULL,
       1.0f, 1.0f, 1, 0)`; step 6.
     - 0x40/0x44: `ReportCollision(ped, other, 0x3E, surfaceId, &pedPos, NULL, 1.0f, 1.0f, 1, 0)`; RETURN (no sound).
     - 0x41/0x42/0x43: `sfx1 = 0x3A`; `volA = 0x4DA510(surfaceId)`; ReportCollision(...) as above; go to 6b.
  6. If `(i16)sfx1 < 0` go to 6c. 6b: first sound: `m_tempSound.Initialise(5, sfx1, this, pedPos, base + volA, 1.0f, 1.0f, 1.0f,
     0, 0x80, spVar1, 0)`; RegisterWithPhysicalEntity(ped); if `delay != 0`: SpeedVariance = 0, Speed = 0, ClientVariable =
     delay stamp, Event = event, SetFlags(4, 1); RequestNewSound. 6c: if `(i16)sfx2 < 0` return.
  7. Second sound: `Initialise(slot2, sfx2, this, pedPos, base + volB, 1.0f, 1.0f, 1.0f, 0, 0x80, 0.0588f, 0)`; register; same
     delay block; RequestNewSound.
  (x87: each sound volume is `fld base; fadd volX`, rounded to f32 by the Initialise argument store.)
- Edge cases: `ped` = `this->m_pPed` (+0x94) is used without a null check (AddAudioEvent checks it). The initial `slot2 = 5`
  store and `volB = -3.0f` matter for the paths that do not overwrite them (e.g. 0x3D -> Rnd path keeps volB -3.0).
- gta-reversed: `plugin::CallMethod` only (its parameter named `maxVol` is the delay). Open questions: none.

## 0x4E2350 CAEPedAudioEntity::HandlePedJacked  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void (this = ECX; i32 event [esp+4])`, `ret 4`. Caller: AddAudioEvent 0x4E2BB0 (events 0x45..0x49).
- Behaviour:
  1. If `!IsSoundBankLoaded(0x8F, 5)`: if `AudioEngine.IsLoadingTuneActive()` return; else `LoadSoundBank(0x8F, 5)`; return
     (nothing is played, not even the footstep).
  2. `vol = evVol(event)` (stored f32), `now = CTimer::m_snTimeInMilliseconds`, `footT = now`. Switch `event - 0x45` (table
     0x4E2680; > 4 -> step 4 with `footT = now`):
     - 0x45 CAR_PUNCH: `sfxA = 0x3A`, `sfxB = Rnd(40, 43)`, `t = now + 500`, `footT = now + 2133` (0x855).
     - 0x46 CAR_HEAD_BANG: `sfxA = 0x32`, `sfxB = Rnd(36, 39)`, `t = now + 933` (0x3A5), `footT = now + 2633` (0xA49).
     - 0x47 CAR_KICK: `sfxB = Rnd(40, 43)`, `t = now + 900`, `footT = now + 2800` (0xAF0); sound A is skipped.
     - 0x48 BIKE: `sfxA = 0x3A`, `sfxB = Rnd(40, 43)`, `t = now + 33`, `footT = now + 366` (0x16E).
     - 0x49 DOZER: `sfxB = Rnd(36, 39)`, `t = now + 266` (0x10A), `footT = now + 833` (0x341); sound A skipped.
  3. Sound A (cases 0x45, 0x46, 0x48): `m_tempSound.Initialise(5, sfxA, this, pedPos, vol, 2.0f (rollOff), 0.0f (speed), 1.0f,
     0, 0x84, 0.0f, 0)` (0x4E24B7-0x4E24F5); RegisterWithPhysicalEntity(ped); `m_ClientVariable = (f32)(u32)t`; `m_Event = event`;
     RequestNewSound. Sound B (all five cases, skipped if `(i16)sfxB < 0`, 0x4E252B): identical with `sfxB` and the same `t`.
  4. Footstep: if `!IsSoundBankLoaded(0, 0x29)`: `LoadSoundBank(0, 0x29)`, return. Else `Initialise(0x29, 0, this, pedPos, vol,
     2.0f, 0.0f, 1.0f, 0, 0x84, 0.0f, 0)`; register; `m_ClientVariable = (f32)(u32)footT`; `m_Event = event`; RequestNewSound.
- gta-reversed: unhooked C++ (`.Reversed = false`). Differences (CONFIRMED, keep-original):
  1. gr passes `.Speed = 2.0f` with the default roll-off 1.0; the original sets roll-off 2.0f and speed 0.0f (0x4E24CB/0x4E24CD).
  2. gr ignores the bank state (`EnsureSoundBankIsLoaded` result unused) and always plays; the original returns without any
     sound when bank 0x8F is not loaded (and when the footstep bank is not loaded it loads it and skips the footstep).
  3. gr's footstep path has no IsLoadingTuneActive test - matches the original (none there either).
  Timings and sound ids match. Open questions: none.

## 0x4E2EE0 CAEPedAudioEntity::Service  (catalogue: specified/REA, port level 6)
- Convention: `__thiscall void (this = ECX)`, plain `ret` or tail `jmp` 0x4F2960. Caller: CPed::ProcessControl 0x5E8CD0.
  Only the shirt-flap / wind-rush loop of the player is handled here.
- Callees: CPed::IsPlayer 0x5DF8F0, CVehicle::IsOpenTopCar (virtual, vtable +0x9C), CPedIntelligence::GetUsingParachute
  0x6011B0, RpAnimBlendClumpGetAssociation(clump, name) 0x4D6870 (cdecl), CAEAudioUtility::AudioLog10 0x4D9E50 (cdecl f32 ->
  ST0), PlayShirtFlap 0x4E2A90 (thiscall `(f32 volume, f32 speed)`), CAETwinLoopSoundEntity::StopSoundAndForget 0x4F2960.
- Fields: ped (+0x94): +0x18 RpClump*, +0x44 m_vecMoveSpeed, +0xFC m_pAttachedTo, +0x46C flags (bit 0x100 bInVehicle), +0x47C
  CPedIntelligence*, +0x530 m_nPedState, +0x58C m_pVehicle. Vehicle: +0x36 type bits, +0x44 m_vecMoveSpeed, +0x1B8 u8
  (m_vehicleAudio.m_AuSettings.VehicleAudioType, gr), +0x590 m_nVehicleType. this +0x158 f32 (gr field_158 = current flap volume).
  Statics (set by the static initialisers, see _static-init-U1.md): 0xB613BC (= 1.2f - 0.8f), 0xB613C0 (= 1.6f - 0.8f),
  0xB613CC (= 1.0f - 0.5f). Tuning in .data: 0x8AE644 = 0.5, 0x8AE648 = 4.0, 0x8AE64C = -6.0, 0x8AE650 = -10.0, 0x8AE654 = -17.0,
  0x8AE658 = 0.8, 0x8AE660 = -16.0, 0x8AE664 = -7.0, 0x8AE668 = 0.8, 0x8AE670 = 5.0, 0x8AE674 = 2.0, 0x8AE69C = 0.5,
  0x8AE6A0 = -20.0, 0x8AE6A4 = 0.7. Names: 0x85F1E4 "FALL_SkyDive_accel", 0x85F1D8 "Para_decel".
- Locals: `vol = -100.0f`, `f = 1.0f`, `onFoot = 1`, `para = 0`, `sky = 0.0f`, `dec = 0.0f`, `base` (uninitialised until set).
  `clamp01(x)`: `if (1.0f < x) x = 1.0f; else if (!(0.0f <= x)) x = 0.0f` (fcomp + `test ah,5`/`test ah,0x41`; NaN -> 0).
  `mag(v)` = `sqrt((x*x + y*y) + z*z)` in that x87 order (0x4E2FA1-0x4E2FB1).
- Behaviour:
  1. If `!m_pPed->IsPlayer()` return.
  2. `onFoot = 0` if (`ped->+0xFC != 0` and `(attached->+0x36 & 7) == 2`) or `ped->+0x46C & 0x100` (0x4E2F21-0x4E2F47; the
     attached test short-circuits the flag test).
  3. In-vehicle (0x4E2F4C): if `ped->+0x530 == 0x32` and `veh = ped->+0x58C` != 0 and (`veh->+0x590 == 9` or (`veh->+0x590 == 0`
     and `veh->IsOpenTopCar()`)): `m = |mag(veh->+0x44)|` (fchs if < 0); if `m > 0.0f` (strict; NaN no): `base = (veh->+0x1B8 == 2)
     ? -17.0f : -10.0f`; `f = clamp01(m)`; `vol = AudioLog10(f) * 20.0f [0x858BA4] + base` (stored f32); `f = [0xB613BC] * f + 0.8f`.
  4. `para = ped->+0x47C ? GetUsingParachute() : 0`.
  5. If `onFoot`: `f = mag(ped->+0x44) / 0.7f` (fdiv, stored f32, 0x4E30AF).
     - no parachute: `base = -16.0f`; if `f < 0.5f` (strict): `f = 0` and go to 5c; else `f = (f - 0.5f) / [0xB613CC]` (stored),
       go to 5b.
     - parachute: `base = -7.0f`; `a = GetAssociation(clump, "FALL_SkyDive_accel")`: if a, `sky = clamp01(a->+0x18)`;
       `b = GetAssociation(clump, "Para_decel")`: if b, `dec = clamp01(b->+0x18)`; go to 5b.
     5b: `f = clamp01(f)`. 5c: `vol = AudioLog10(f) * 20.0f + base`; `f = [0xB613C0] * f + 0.8f` (0x4E322E).
  6. (0x4E3242, always) `f = (0.5f * sky + 1.0f) * f`; `target = (4.0f * sky + vol) + (-6.0f * dec)` (x87 in that order, kept
     unrounded in ST0).
  7. Step `cur = this->+0x158` toward `target`: if `target > cur`: `n = cur + 5.0f`; `cur = (n < target) ? n : target`.
     Else if not unordered: `n = cur - 2.0f`; `cur = (n > target) ? n : target` (equal gives target). (NaN: no store.)
     Then if `target < -20.0f` and `cur < -20.0f` (both strict): `cur = target` (0x4E32C4-0x4E32E4).
  8. If `cur <= -100.0f` [0x859014] (`test ah,0x41; jp`; NaN plays): `m_sTwinLoopSoundEntity(+0xA8).StopSoundAndForget()` (tail).
     Else `PlayShirtFlap(cur, f)`.
- Edge cases: if neither 3 nor 5 sets `vol`, the target is `-100 + 4*sky - 6*dec` = -100 (sky = dec = 0) and the loop stops
  once `cur` falls to -100 (stepping down 2.0 per frame, snapping when below -20). `f < 0.5` on foot gives AudioLog10(0).
- gta-reversed: `plugin::CallMethod` only. Open questions: none.

## 0x5DE8D0 CAEPedAudioEntity::Constructor  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CAEPedAudioEntity* (this = ECX)`, returns this, plain `ret`. No direct callers (the CPed ctor
  inlines it; this is the out-of-line copy).
- Behaviour (store order): `+0x10 = 0` (m_tempSound.m_PhysicalEntity), `+0 = 0x86C2A8`, `+0xB8 = 0` (twin-loop's
  m_tempSound.m_PhysicalEntity), `+0xA8 = 0x85F438` (twin-loop vptr), `+0x130 = 0` (u16, twin-loop m_IsInUse), `+0x148 = 0`,
  `+0x14C = 0` (twin-loop sounds), `+0x94 = 0` (m_pPed), `+0x7C = 0` (u8 m_bCanAddEvent), `+0x9C = +0xA0 = +0xA4 = 0` (jet-pack
  sounds). Nothing else.
- gta-reversed: ctor commented out of the hooks (`.Reversed = false`); gr's C++ ctor writes the same five members plus
  default member initialisers - the original leaves the rest untouched. Open questions: none.

## 0x5DE920 CAEPedAudioEntity::~CAEPedAudioEntity  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void (this = ECX)`, plain `ret`. MSVC EH frame; the `push <handler>` is obfuscated through the
  .text stub 0x4084CA (`push eax; pushfd; clc; mov eax,[0x15608D0]; adc eax,[0x7034B0]; popfd; xchg [esp],eax; jmp 0x5DE927`)
  which pushes `0x30A209FF + 0xCFE1CDE9 = 0x0083D7E8` (the EH handler) with all registers and flags preserved.
  Callers: CPed::Destructor 0x5E8620, CPed::Constructor unwind.
- Behaviour: `+0 = 0x86C2A8`; `CAETwinLoopSoundEntity::~CAETwinLoopSoundEntity(this + 0xA8)` 0x4F2B80; base: `+0 = 0x859988`,
  `CAESound::~CAESound(this + 8)` 0x4EF660.
- gta-reversed: implicit. Open questions: none.
