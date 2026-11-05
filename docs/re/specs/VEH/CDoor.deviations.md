# CDoor: gta-reversed deviation notes (VEH)

Source: `re/gta-reversed/source/game_sa/Door.cpp` (pin 20d8d2a). Layout (size 0x18): +0x00 `m_openAngle`, +0x04
`m_closedAngle`, +0x08 `m_dirn` u16 (low nibble = direction 0 X, 1 Y, 2 Z, 3 -X, 4 -Y, 5 -Z; 0x10 BASED, 0x20
LOW_GRAVITY, 0x40 CHASSIS, 0x80 FIXEDSTATE, 0x100 FIRETRUCK), +0x0A `m_axis` u8, +0x0B `m_doorState` u8, +0x0C
`m_angle`, +0x10 `m_prevAngle`, +0x14 `m_angVel`. Both functions: `__thiscall bool (CDoor* this, CVehicle* veh,
const CVector& oldMoveSpeed, const CVector& oldTurnSpeed, const CVector& offset)`, AL result, `ret 0x10`, SEH frame
for a local CMatrix built from `veh->m_matrix` (CMatrix(const CMatrix&) 0x59BCF0).
In vanilla data every door uses `m_axis` X (0) or Z (2) (Automobile.cpp:204-289), so the axis-Y paths below are
unreachable with the shipped game.

## 0x6F4040 CDoor::Process  (gr_deviations = 2 rows, 1 counted)
Original direction vector (0x6F4146-0x6F41A7): `d = (0,0,0)`, then by `m_dirn & 0xF` via table 0x6F4524:
0 -> d.x = 1, 1 -> d.y = 1, 2 -> d.z = 1, 3 -> d.x = -1, 4 -> d.y = -1, 5 -> d.z = -1; values > 5 leave d = 0.
`s = f32(fsin(m_angle))`, `c = f32(fcos(m_angle))` (0x6F41AF-0x6F41C4). Then by `m_axis` (0x6F41B6-0x6F41CD):
- axis 2 (Z): `x' = c*d.x - s*d.y`, `y' = c*d.y + s*d.x` (computed as `t1 = c*d.y + s*d.x; t2 = c*d.x - s*d.y`,
  0x6F41D3-0x6F4205), z unchanged; `ang = Cross(accel, d').z` (0x6F422A-0x6F422F).
- axis 0 (X): `y' = c*d.y - s*d.z`, `z' = c*d.z + s*d.y` (0x6F4237-0x6F426C), x unchanged; `ang = Cross(accel, d').x`.
- any other axis: no rotation and `ang = s` (`fld [esp+0x10]` reloads the stored sine, 0x6F429C).

Row Door.cpp:60 (`default: NOTSA_UNREACHABLE()` in the direction switch).
- Original bytes: `ja 0x6F41AF` with d left at (0,0,0).
- Verdict: CONFIRMED (gr aborts / UB).
- Class: keep-original (unreachable with vanilla data).

Row Door.cpp:67 (`assert(m_axis != DOOR_AXIS_Y)`).
- Original bytes: axis values other than 0 and 2 take the `ang = sin(m_angle)` path above. gr (release) applies its
  generic `(axis+1)%3 / (axis+2)%3` rotation instead.
- Verdict: CONFIRMED as a behavioural difference, unreachable with vanilla data.
- Class: keep-original.

Found in passing (CONFIRMED, keep-original; reachable for every door): gr's rotation has the wrong sign on the first
component. gr: `newA = sin*d[b] - cos*d[a]`, `newB = sin*d[a] + cos*d[b]` with a = (axis+1)%3, b = (axis+2)%3. The
original computes `newA = cos*d[a] - sin*d[b]` (fsubp st(1) at 0x6F4205 / 0x6F426C: `c*d.x - s*d.y` for axis Z,
`c*d.y - s*d.z` for axis X). For a door with direction -Y and axis Z (all side doors) gr gets x' = -sin where the
original gets +sin, so `ang` (and the door swing) differ. The rest of the function (apply rates 0.025f / 0.0f
[0x872328 / 0x87232C], chassis damping `pow(0.98f|0.95f, timestep)` via CRT pow 0x822130 then spring 0.06f, boat
0.98f, FIRELA (model 0x220) 0.92f and spring 0.01f, damping 0.97f, caps 0.02f / 0.05f / DOOR_SPEED_MAX_CAPPED
[0x8D3950], the end-stop logic with -0.8f / -0.9f / -0.4f and slam-shut at |angVel| > 0.1f) matches gr's order.

## 0x6F4540 CDoor::ProcessImpact  (gr_deviations = 3 rows, 2 counted)
Original: `accel = veh->GetSpeed(offset) - (Cross(oldTurnSpeed, offset) + oldMoveSpeed)` (0x6F4585-0x6F45EA); the
direction `dir` is a column of `veh->m_matrix` by `m_dirn & 0xF` (table 0x6F4770: 0 right, 1 forward, 2 up, 3 -right,
4 -forward, 5 -up; negation by fchs per component).

Row Door.cpp:172 (`default: NOTSA_UNREACHABLE()` in the direction switch).
- Original bytes: `ja 0x6F46A4` skips the store, `dir` (esp+0x18..0x20) stays uninitialised stack data.
- Verdict: CONFIRMED (both are undefined; unreachable with vanilla data).
- Class: keep-original (harmless in practice).

Row Door.cpp:176 (`assert(m_axis != DOOR_AXIS_Y)`).
- Original bytes: axis 2 -> `ang = Cross(accel, dir).z`, axis 0 -> `ang = Cross(accel, dir).x` (0x6F46A4-0x6F46E9);
  any other axis -> `ang` is the stack slot esp+0x24, which holds the f32 `oldTurnSpeed x offset .y + oldMoveSpeed.y`
  written at 0x6F45BA (a leftover temporary). gr computes `cross[m_axis]`.
- Verdict: CONFIRMED as a behavioural difference, unreachable with vanilla data.
- Class: keep-original.

Row Door.cpp:180 (rng-helper `GetRandomNumberInRange(0.75f, 1.5f) * POP_OPEN_DOOR_ACCEL_LIMIT`).
- Original bytes (inline, 0x6F46EC-0x6F470B): `limit = (((float)rand() * 3.0518509e-05f [0x858C7C]) * 0.75f
  [0x858F34] + 0.75f) * [0x8D3968]` (0x8D3968 = 0.1f, data). Then if `m_openAngle < m_closedAngle` (jnp at 0x6F471B):
  return `ang < -limit`; else return `ang > limit` (NaN -> false in both).
- Verdict: CONFIRMED (gr lerp `1.5*t + 0.75*(1-t)` rounds differently from `t*0.75 + 0.75`).
- Class: keep-original.
