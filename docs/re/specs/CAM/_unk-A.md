# CAM: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x4d58a0 ConvertPedNode2BoneTag (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl i32 (i32 node)`: default -1 (`or eax,-1`); `switch (node - 1)` unsigned `<= 0x11` via table 0x4d5920.
- Mapping (from the table): 1->3, 2->5, 3->32, 4->22, 5->34, 6->24, 7->41, 8->51, 9->43, 10->53, 11->52, 12->42, 13->33, 14->23, 15->31, 16->21, 17->4, 18->8; anything else -> -1.
- Note entries 11 and 12 are crossed in the table (11 -> 0x4d58f6 = 52, 12 -> 0x4d58f0 = 42).

## 0x5097b0 CCamera member ctor (stores 0 to [this]) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void* ()`: `[this] = 0`, EAX = this (0x5097b2). Same as 0x43e3f0 (CPtrList ctor); which member of CCamera is constructed is an open question.

## 0x5097c0 empty function (ret) (proposed)  (catalogue: unknown/REA, port level 0)
- Empty: single `ret` (0x5097c0). Port: no-op.

## 0x5097d0 empty function (ret) (proposed)  (catalogue: unknown/REA, port level 0)
- Empty: single `ret` (0x5097d0). Port: no-op.

## 0x509800 empty function (ret) (proposed)  (catalogue: unknown/REA, port level 0)
- Empty: single `ret` (0x509800). Port: no-op.

## 0x50a0a0 round-to-decimals helper (sub_50a0a0) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl float (float v [esp+4], i32 d [esp+8])`, ST0 return.
- `p = _CIpow(10.0 [0x859ea8] (f64), (double)(d+1))` 0x822130; `x = p * v` (0x50a0bb); `if (v < 0.0f [0x858b50])` (`fcomp; test ah,5; jp` not taken, 0x50a0ce) `x -= 5.0f [0x858c80]` else (v >= 0 or NaN) `x += 5.0f [0x858c80]` (0x50a0d8); `x *= 0.1f [0x858b1c]`; `modf(x, &ip)` 0x8232f0 (fraction discarded); `r = ip` (f64 from memory; an f32 copy is stored over the v slot at 0x50a0fa but ST0 keeps r); `q = _CIpow(10.0, (double)d)`; return `r / q` (`fdivp`, 0x50a111).
- Effect: round half away from zero to d decimals, via integer part of (v*10^(d+1) +- 5)/10. Note f64 `fld qword 10.0` and int `fild` feed the CRT pow; the quotient is returned in ST0 without a store.

## 0x50a120 CVector round-to-decimals (applies 0x50a0a0 to x,y,z) (proposed)  (catalogue: unknown/REA, port level 1)
- `__cdecl void (CVector* v, i32 d)`: `v.x = f(v.x, d)`, `v.y = f(v.y, d)`, `v.z = f(v.z, d)` with f = 0x50a0a0 (ST0 stored as f32).

## 0x50e080 empty function (ret) (proposed)  (catalogue: unknown/REA, port level 0)
- Empty: single `ret` (0x50e080). Port: no-op.

## 0x50e180 CCam heli-chase settings struct init (sub_50e180) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`; immediate stores: `+0x18 = 50.0f`, `+0x1c = 30.0f`, `+0x20 = 55.0f`, `+0x24 = 50.0f`, `+0x28 = 0.05f`, `+0x2c = 70.0f`, `+0x30 = 22.0f`, `+0x34 = 1.0f`, `+0x38 = 0`, `+0x3c = 10.0f`, `byte+0x40 = 0`, `+0x44 = 30` (int), `+0x48 = 30` (int), `+0x58 = 8`, `byte+0x5c = 0`, `+0x60 = 60`, `+0x64 = 60`, `+0x68 = 100.0f`, `+0x6c = 110.0f`, `+0x70 = 10.0f`, `+0x74 = 5.0f`, `+0x78 = 12.0f`, `+0x7c = 48.0f`, `byte+0x80 = byte+0x81 = 0`, `byte+0x88 = 0`, `+0x94 = 0.75f`, `+0x98 = 4000` (int). Store order as at 0x50e185-0x50e231 (no reads, order irrelevant).
- Open question: owning struct/class (fields not in gr CCam) (0x50e180).

## 0x50e470 CCamera member struct ctor (sub_50e470) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void* ()`, EAX = this: `[+0] = -1`; `[+4] = 1.0f`; `[+8] = 1.0f` (immediate 0x3f800000); `[+0xc] = 0`.
- Open question: which CCamera member (0x50e470).

## 0x514010 CCamera::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 3)
- `__thiscall void* (u32 flags)`, `ret 4`: `CCamera::~CCamera 0x50a870(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x514010).
