#!/usr/bin/env node
// Checks that the web build leaves the legacy address range free (WEB-1): every active data segment of the module
// starts at or above 0x1600000 (crates/sa-game/src/arena.rs WASM_LOW_END), and the stack pointer starts above it.
//   node tools/web/check_layout.mjs [web/pkg/sa_game_bg.wasm]
import { readFileSync } from 'node:fs';

const LOW_END = 0x1600000;
const b = readFileSync(process.argv[2] ?? 'web/pkg/sa_game_bg.wasm');
let p = 8;
const leb = () => {
  let r = 0, s = 0, x;
  do { x = b[p++]; r += (x & 0x7f) * 2 ** s; s += 7; } while (x & 0x80);
  return r;
};
const sleb = () => {
  let r = 0, s = 0, x;
  do { x = b[p++]; r |= (x & 0x7f) << s; s += 7; } while (x & 0x80);
  if (s < 32 && (x & 0x40)) r |= -1 << s;
  return r;
};
let ok = true, segs = 0, lowest = Infinity, sp = null;
while (p < b.length) {
  const id = b[p++];
  const size = leb();
  const end = p + size;
  if (id === 6) { // globals: the first i32 mutable global is __stack_pointer
    const n = leb();
    for (let i = 0; i < n; i++) {
      const ty = b[p++]; const mut = b[p++];
      const op = b[p++];
      const v = op === 0x41 ? sleb() : (op === 0x42 ? sleb() : null);
      while (b[p++] !== 0x0b);
      if (sp === null && ty === 0x7f && mut === 1) sp = v;
    }
  }
  if (id === 11) {
    const n = leb();
    for (let i = 0; i < n; i++) {
      const kind = leb();
      if (kind === 0) {
        const op = b[p++];
        const off = sleb() >>> 0;
        p++; // end
        const len = leb();
        p += len;
        segs++;
        lowest = Math.min(lowest, off);
        if (op !== 0x41 || off < LOW_END) ok = false;
      } else if (kind === 1) { const len = leb(); p += len; }
      else { leb(); const op = b[p++]; sleb(); p++; const len = leb(); p += len; }
    }
  }
  p = end;
}
console.log(`${segs} active data segments, lowest at 0x${lowest.toString(16)}; initial stack pointer 0x${(sp >>> 0).toString(16)}; legacy range 0..0x${LOW_END.toString(16)} ${ok && sp >>> 0 > LOW_END ? 'free' : 'NOT free'}`);
process.exit(ok && sp >>> 0 > LOW_END ? 0 : 1);
