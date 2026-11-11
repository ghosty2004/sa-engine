# CMemoryHeap (CALLED-ONLY) — RE-SPEC-REST

## 0x72E8E0 CMemoryHeap::IntegrityCheck  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this → EDX), no args, plain `ret`; void. Callers: none in game code (gr wrapper only).
- Behaviour: only READS memory (no global or heap writes; the only writes go to a 0x50-byte local array), so the observable effect is
  nil unless the heap is corrupt (then it may loop forever or fault):
  1. Walk the list starting at `[this+0x18]` following `node+0x10` until the node equals `this+0x20` (0x72E8E2-0x72E8F5).
  2. If `[this+0x38] != 0`: for 64 buckets (`ecx = [this+0x38]+0x18`, step 0x3C): walk from `[bucket-8]` via `+0x10` until == bucket.
  3. Zero 20 local dwords; for blocks from `[this+0]` while block != `[this+4]`: if `block.used (+4, u8)` and `block.memId (+6, i16) != -1`:
     `local[memId] += block.size (+0) + 0x10`; `block += block.size + 0x10` (0x72E933-0x72E960). memId is not range-checked against 20.
  4. Walk `[end+0xC]` chain via `+0xC` until NULL (0x72E962-0x72E975).
- Port: a no-op is behaviourally identical for a valid heap (CMemoryHeap is not used by the PC game's normal allocation path).
- gta-reversed: `plugin::CallMethod` only.
