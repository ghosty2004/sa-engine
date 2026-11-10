# CAEVehicleAudioEntity (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Entities/AEVehicleAudioEntity.cpp` (pin 20d8d2a), cited as VAE.cpp:line. Fields
used: +4 m_Entity (vehicle), +0x80 i8 m_AuSettings.VehicleAudioType (0 CAR, 1 BIKE, 2 BMX, 3 BOAT, 4 HELI, 5 PLANE,
6 SEAPLANE, 7 ONE_GEAR, 8 TRAIN, 9 SPECIAL, 10 NO_VEHICLE), +0xA4 m_IsInitialized, +0xA7 u8, +0xA8 m_IsWreckedVehicle, +0xA9 i8
m_State, +0xC4 f32 m_HornVolume, +0xC8 u8 m_HasSiren, +0xD8 f32 m_EventVolume, +0xDC/+0xDE banks, +0xE0 i16 m_DummySlot, +0xE8
m_EngineSounds[] (stride 8, sound pointer first), +0x178 m_HornSound. CAESound: +0x14 m_Volume, +0x1C m_Speed, +0x56 m_Flags.
"rbf GenericFrameRate" rows: gr optionally scales the per-frame step by the time step; the original always uses a fixed
per-frame step.

## 0x4F4EE0 CAEVehicleAudioEntity::DisableHelicoptors  (gr_deviations = 1)
Original: `mov byte [0xB6B994], 1; ret` (s_HelicoptorsDisabled = 1). Row :299 (address-swap comment): gr's assignment of this
address to DisableHelicoptors matches the bytes. REFUTED, harmless.

## 0x4F4EF0 CAEVehicleAudioEntity::EnableHelicoptors  (gr_deviations = 1)
Original: `mov byte [0xB6B994], 0; ret`. Row :304: REFUTED (matches), harmless.

## 0x4F56D0 CAEVehicleAudioEntity::UpdateVehicleEngineSound  (gr_deviations = 2)
Original: `__thiscall void (this; i16 st [esp+4], f32 speed [esp+8], f32 volume [esp+0xC])`, `ret 0xC`, no return value.
`v = volume + m_EventVolume` (`fld volume; fadd [+0xD8]`); `s = m_EngineSounds[st].Sound (+0xE8 + st*8)`; if `s`: `s->m_Volume = v`,
`s->m_Speed = speed` (raw dword copy). Rows :1401 / :4883 (gr returns bool, not hooked): CONFIRMED signature-only difference.
Class: harmless (no caller of the original reads EAX).

## 0x4F5700 CAEVehicleAudioEntity::JustGotInVehicleAsDriver  (gr_deviations = 1)
Original: `+0xA7 = 0`; `s_pPlayerDriver (0xB6B990) = vehicle->+0x460`; dispatch on +0x80: 0/1 -> 0x4F5A05, 4/5/6 -> 0x4F5957, 2 BMX
0x4F5753, 3 BOAT 0x4F57FF, 8 TRAIN 0x4F584B, 9 SPECIAL 0x4F58E5; any other type (7 ONE_GEAR, 10+) returns (0x4F5B0C).
Row :1758 (`default: NOTSA_UNREACHABLE()`): CONFIRMED (original returns silently after the two stores). Class: keep-original.

## 0x4F5EB0 CAEVehicleAudioEntity::UpdateGasPedalAudio  (gr_deviations = 1)
Original: `void (CVehicle* veh [esp+4], i32 type [esp+8])`, `ret 8`, ECX unused. `cur = |veh->+0x49C|` (stored f32). Field: type 0 ->
`veh+0x964`, type 9 -> `veh+0x808`, ANY OTHER TYPE -> return without update (0x4F5ECB). Step: if `cur > v`: `v = min(v + 0.09f
[0x8CBC24], cur)`; else `v = max(v - 0.07f [0x8CBC28], cur)` (equal -> stays).
Row :406 (rbf GenericFrameRate): CONFIRMED only with the bugfix enabled. Class: keep-original (fixed per-frame steps).
Found in passing (CONFIRMED, keep-original): gr writes the automobile field for every non-bike type; the original skips
types other than 0 and 9.

## 0x4F5F30 CAEVehicleAudioEntity::GetVehicleDriveWheelSkidValue  (gr_deviations = 1)
Original: `__thiscall f32 (this; veh, i32 wheelState [esp+8], f32 revs [esp+0xC], cTransmission&, f32 speed [esp+0x14])` -> ST0,
`ret 0x14`. If `*(u8*)0x8CBD80 == 0` return 0. State 1 (SPINNING): `revs > 0.4f` ? `(revs - 0.4f) * 1.6666666f [0x85F0A8] * 0.75f *
[0x8CBD88]` : `0.0f * [0x8CBD88]`. State 2 (SKIDDING): `min(|speed|, 1.0f) * 0.75f * [0x8CBD84]`. State 3 (FIXED): `|speed| <= 0.04f`
-> `0.0f * [0x8CBD8C]`; else `min(|speed| * 5.0f, 1.0f) * [0x8CBD8C]`. Any other state (0 or invalid): returns 0.0f.
Row :2213 (NOTSA_UNREACHABLE): CONFIRMED (original returns 0.0f). Class: keep-original.
Found in passing (CONFIRMED, minor): gr's `invLerp(0.4, 1, x)` divides by 0.6 where the original multiplies by 1.6666666f;
at exactly `|speed| == 0.04f` gr computes a value, the original returns 0.

## 0x4F6000 CAEVehicleAudioEntity::GetVehicleNonDriveWheelSkidValue  (gr_deviations = 1)
Original (`ret 0x10`): if `*(u8*)0x8CBD81 == 0` return 0. State 2: `min(|v|, 1.0f) * 0.75f * [0x8CBD84]`, then `* 0.2f` if `+0x80 == 2`
(BMX factor applied LAST). State 3: `|v| <= 0.04f` -> 0.0f, else `min(|v| * 5.0f, 1.0f) * [0x8CBD8C]`. Other states -> 0.0f.
Row :2243 (NOTSA_UNREACHABLE): CONFIRMED (returns 0.0f). Class: keep-original.
Found in passing (CONFIRMED gr logic error, keep-original): for FIXED wheels with `|v| < 0.04` gr returns `|v| * factor`, the
original returns 0 (0x4F603B). BMX factor order differs (gr multiplies 0.2 before the config factor).

## 0x4F61E0 CAEVehicleAudioEntity::GetHornState  (gr_deviations = 1)
Original: `__thiscall void (this; bool* out [esp+4], tVehicleParams* vp [esp+8])`, `ret 8`. Pattern re-roll when `m_HornCounter
(+0x514) == 44` (after clamping to 44): `veh->+0x518 = (u8)CAEAudioUtility::GetRandomNumberInRange<int32>(0, 7)` (0x4F6248); output
`table[0x862B9C + pattern*44 - counter]` (byte); wreck/alarm/status logic matches gr.
Row :1979 (`CGeneral::GetRandomNumberInRange(0u, HORN_NUM_PATTERNS)`): CONFIRMED (different RNG helper/formula). Class: keep-original.

## 0x4F62A0 CAEVehicleAudioEntity::GetSirenState  (gr_deviations = 1)
Row :1940 (`#ifdef NOTSA_DEBUG` keyboard override): debug-only; the remaining logic matches the bytes (0x4F62A0-0x4F6311:
wrecked or no siren or `!(veh->+0x42D & 0x80)` -> siren 0; status 4 (abandoned) -> siren 0; else siren 1 and fast = counter != 0
&& model != 0x1A7). REFUTED, harmless.

## 0x4F6420 CAEVehicleAudioEntity::AddAudioEvent  (gr_deviations = 1)
Row :888 (`EventID = IsFixBugs() ? AE_PED_DRIVE_OVER : AE_PED_KNOCK_DOWN`): original stores `m_Event = 0x79` (KNOCK_DOWN,
0x4F6C03). CONFIRMED only when FIX_BUGS is on. Class: keep-original.
Found in passing (CONFIRMED, keep-original): the drive-over sound sets `m_Flags = 0x20` (SOUND_START_PERCENTAGE) and
`m_PlayTime = 0` (0x4F6BF7/0x4F6BFE); gr passes no flags. Volume = evVol(0x78) - 3.0f (type 1) / - 6.0f (type 2).

## 0x4F7A50 CAEVehicleAudioEntity::RequestNewPlayerCarEngineSound  (gr_deviations = 2)
Original: if `m_DummySlot == -1` return. First switch `st - 1 > 4` skips the bank checks (0x4F7AA4); `v = volume + m_EventVolume`;
if `m_EngineSounds[st]` exists and `st != 5` -> CancelVehicleEngineSound(st) 0x4F55C0; second switch `st - 1 > 4` goes straight
to 0x4F7EAA: `m_EngineSounds[st] = RequestNewSound(&local)` with the local CAESound NEVER initialised (only its +8 cleared).
Rows :1445 / :1535 (NOTSA_UNREACHABLE): CONFIRMED - the original requests a garbage sound for invalid `st` (original bug).
Class: keep-original (unreachable with valid callers).

## 0x4F8070 CAEVehicleAudioEntity::GetFreqForPlayerEngineSound  (gr_deviations = 1)
Row :2687 (default NOTSA_UNREACHABLE): original `st - 1 > 4` jumps to 0x4F82E1 and continues with `f = [esp+0x14]` = the raw
dword of the `st` argument slot reinterpreted as f32, then applies the drowning (`* 0.7f [0x8CBC48]` if `veh->+0x42B & 0x40`) and
nitro (`* (0.12f [0x8CBC50] * ratio + 1)`) factors. CONFIRMED (gr aborts). Class: keep-original.

## 0x4F8F10 CAEVehicleAudioEntity::ProcessVehicleSkidding  (gr_deviations = 1)
Row :2144: base vehicle type 0 -> automobile fields (+0x964..), 9 -> bike fields (+0x808..), anything else returns (0x4F8F52 ->
0x4F92A9). CONFIRMED, keep-original.

## 0x4F93C0 CAEVehicleAudioEntity::PlayAircraftSound  (gr_deviations = 2)
Original: existing sound -> `m_Volume = volume` (raw arg) and `m_Speed = speed` (0x4F9637-0x4F964F). New sound: each case calls
Initialise (volume argument 0) and then the common tail 0x4F9602 sets `m_Volume = volume` (RAW, without m_EventVolume),
`m_Speed = speed`, `m_Flags = 4`, RequestNewSound. `st - 1 > 6` jumps to that tail with the local CAESound uninitialised.
Row :3244 (VolumeFix bugfix): CONFIRMED only with the bugfix. Row :3270 (default unreachable): CONFIRMED (original requests a
garbage sound). Found in passing (CONFIRMED gr logic error, keep-original): gr's new sounds use `m_EventVolume + volume`, the
original uses the raw `volume`.

## 0x4F9710 CAEVehicleAudioEntity::PlayBicycleSound  (gr_deviations = 2)
Original: if `m_DummySlot == -1` return; bank checks for st 1/2/3; existing sound -> raw volume/speed update (0x4F9986); new sound
volume = raw `volume` (0x4F9949-0x4F9971), flags 4. Other `st` without a sound: return (0x4F9885); with a sound: updated.
Row :4340 (VolumeFix): CONFIRMED only with the bugfix (default gr also uses the raw volume). Row :4401 (unreachable): CONFIRMED,
keep-original.

## 0x4F99D0 CAEVehicleAudioEntity::PlayHornOrSiren  (gr_deviations = 1)
Row :2025 (`m_HornVolume - pow(BikeBellFadeOut, timeStep)`): original `m_HornVolume = max(m_HornVolume - 1.5f [0x8CBECC], -100.0f)`
(fixed per-frame step, 0x4F9B08-0x4F9B2F); then, only if `m_HornSound != 0`: if `> -100` set its volume, else stop.
CONFIRMED (gr changes the formula; equal only at time step 1). Class: keep-original.

## 0x4F9E90 CAEVehicleAudioEntity::UpdateBoatSound  (gr_deviations = 1)
Row :4332: original (body continues via stub 0x401B00) updates any existing sound; without one, st 2/3/6 create, others return
(0x4F9EEC -> 0x4FA0AC). CONFIRMED, keep-original.

## 0x4FA1C0 CAEVehicleAudioEntity::UpdateTrainSound  (gr_deviations = 1)
Row :4203: st 1/2/3 create; others without a sound return (0x4FA205 -> 0x4FA3DC). CONFIRMED, keep-original.

## 0x4FB070 CAEVehicleAudioEntity::ProcessNitro  (gr_deviations = 1)
Row :2492 (rbf GenericFrameRate): original steps `m_CurrentNitroRatio (+0x248)` per frame (step down 0.085f [0x8CC044] at 0x4FB133).
CONFIRMED only with the bugfix. Class: keep-original. Open question: the step-up constant was not re-read (0x4FB0FD-0x4FB13D).

## 0x4FB260 CAEVehicleAudioEntity::ProcessMovingParts  (gr_deviations = 2)
Rows :2536 (MODEL_CEMENT case comment) and :2545 (`notsa::step_to` of m_MovingPartSmoothedSpeed): not re-verified against the
bytes in this pass. UNCLEAR - open question at 0x4FB260 (model switch and step block).

## 0x4FBB10 CAEVehicleAudioEntity::ProcessPlayerVehicleEngine  (gr_deviations = 4)
Row :3109 (invalid m_State): original `(i8)+0xA9 > 6` returns (0x4FBD17 -> 0x4FC9E5). CONFIRMED, keep-original.
Rows :2774 / :2775 (asserts): debug-only. REFUTED, harmless.
Row :2746 (non bike/automobile `NOTSA_UNREACHABLE`): the original prologue (0x4FBB10-0x4FBC3B) reads the gear table and
`veh->+0x4B4` without a vehicle-type split at that point. UNCLEAR - open question at 0x4FBB4E (where/whether the original
selects bike vs automobile wheel data).

## 0x4FCA10 CAEVehicleAudioEntity::ProcessDummyStateTransition  (gr_deviations = 8)
Original: `+0x144 = arg`; branches on old state `+0xA9` (1, 2, ...) and the new-state argument (i16 compares at 0x4FCA33, 0x4FCB44);
unmatched combinations fall through to the common epilogue / reset path (0x4FCBDF: `+0xA9 = 0`, stop sounds +0xF8/+0xF0).
Rows :1114, :1122, :1183, :1191 (rbf GenericFrameRate on m_FadeIn/m_FadeOut): CONFIRMED only with the bugfix (fixed per-frame
steps in the original). Rows :1157, :1213, :1250, :1255 (NOTSA_UNREACHABLE for invalid states): UNCLEAR - not traced in this pass;
open question at 0x4FCA28 / 0x4FCB44 (what the original does for unexpected old/new states).

## 0x4FCF40 CAEVehicleAudioEntity::JustGotOutOfVehicleAsDriver  (gr_deviations = 1)
Row :1637: types 0/1 -> 0x4FCFFE, 4/5/6 -> 0x4FCFC8, 8 -> PlayTrainBrake path, 9 -> special path; 2 (BMX), 3, 7, 10+ go to
0x4FD097 which only clears `s_pPlayerDriver (0xB6B990) = 0` and returns. CONFIRMED (gr aborts for unlisted types). keep-original.

## 0x4FF900 CAEVehicleAudioEntity::ProcessGenericJet  (gr_deviations = 1)
Row :3822 (`step_to(m_CurrentRotorFrequency, targetFreq, FreqStep)`): not re-verified. UNCLEAR - open question at 0x4FF900.

## 0x5003A0 CAEVehicleAudioEntity::ProcessBoatEngine  (gr_deviations = 2)
Rows :4243 / :4252 (rbf GenericFrameRate, steps 0.02f / 3.0f): CONFIRMED only with the bugfix; step constants not re-read
(open question at 0x5003A0). Class: keep-original.

## 0x501480 CAEVehicleAudioEntity::ProcessDummyVehicleEngine  (gr_deviations = 1)
Row :4589 (NUM_STATES handled with the idle case): state `(i8)+0xA9` 0..10 through byte table 0x501640 = {0, 0, 1, 2, 2, 2, 2, 2, 2,
2, 0}: states 0, 1 and 10 share the first case (0x50159C), as gr keeps it. REFUTED (gr matches). States > 10 go to 0x50161E
(ProcessDummyStateTransition). Class: harmless.

## 0x501E10 CAEVehicleAudioEntity::ProcessVehicle  (gr_deviations = 1)
Row :4796 (default NOTSA_UNREACHABLE): `(i8)+0x80 > 9` (incl. 10 NO_VEHICLE) returns (0x501EE5 -> 0x502240). CONFIRMED, keep-original.

## 0x6D0A10 CAEVehicleAudioEntity::Destructor  (gr_deviations = 1)
Original: `+0 = 0x862CEC`; if `+0xA4` Terminate 0x4FB8C0; `~CAETwinLoopSoundEntity(this + 0x184)`; base: `+0 = 0x859988`,
`~CAESound(this + 8)`. Row :70 (GenericUB bugfix `CancelSoundsOwnedByAudioEntity`): CONFIRMED only with the bugfix (the original
leaves sounds owned by the entity running - original use-after-free hazard). Class: keep-original.
