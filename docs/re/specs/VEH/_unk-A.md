# VEH: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x41bd00 identity cast getter (returns this) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, EAX = this, no stores (0x41bd00).

## 0x41bdd0 CVehicle flag setter (bit 4 of +0x428, guarded by bit 1 of +0x42d) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void (bool on)`, `ret 4`; body 0x1567620.
- `if (byte[this+0x42d] & 2)` -> `byte[this+0x428] &= 0xef` (bit 4 forced off, 0x1567645); else bit 4 of +0x428 = bit 0 of `on` (`((on << 4) ^ b) & 0x10 ^ b`, 0x1567633-0x156763c).
- Plausibly CVehicle::SetEngineOn with the "engine broken" guard (gr vehicle flags: +0x428 bit 4 bEngineOn, +0x42d bit 1 bEngineBroken). Open question: confirm flag names against gr Vehicle.h (0x1567620).

## 0x4217b0 identity cast getter (returns this) (proposed)  (catalogue: unknown/REA, port level 0)
- Same body as 0x404350: `__thiscall`, EAX = this. Open question: which inline (0x4217b0).

## 0x4c88f0 RpAtomic callback returning its argument (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl`, returns EAX = first stack argument (0x4c88f0); no other effect.

## 0x4f4ca0 CVehicle flag getter (bit 5 of byte +0x428) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall bool ()`: AL = `(byte[this+0x428] >> 5) & 1` (0x4f4ca0-0x4f4ca9); upper EAX bytes untouched.
- Open question: gr name of this vehicle flag bit (CVehicle flags at +0x428, bit 5) (0x4f4ca6).
