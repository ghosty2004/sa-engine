# CAcquaintance (AI) - RE-SPEC-3 group I2

## 0x608780 CAcquaintance::~CAcquaintance  (catalogue: specified/REA, port level 0)
- `__thiscall void ()`: body is a single `ret` (trivial). Referenced as the dtor fn pointer by CPedType::Initialise / Shutdown
  (eh_vector iterators). gr: `= default`, matches.

## 0x608AE0 CAcquaintance::CAcquaintance  (catalogue: specified/REA, port level 0)
- `__thiscall this ()`: zeroes 5 dwords (+0..+0x10, the five relationship bitmasks) and returns this (EAX). gr Acquaintance.cpp:16 matches.
