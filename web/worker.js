// The game's worker (WEB-1): runs sa-game's wasm module, reads the user's game folder synchronously, draws into the
// OffscreenCanvas the page transferred, and turns the page's input messages into platform input.
import init, * as sa from './pkg/sa_game.js';

const post = (type, data) => self.postMessage({ type, ...data });

// console lines (the game's println!) also go to the page's log panel
for (const level of ['log', 'warn', 'error']) {
  const orig = console[level].bind(console);
  console[level] = (...a) => {
    orig(...a);
    post('log', { level, text: a.map(String).join(' ') });
  };
}

// ---------------------------------------------------------------- the game folder (synchronous reads) ---
// files[id] = { size, file } (picked folder: FileReaderSync) or { size, url } (localhost dev server: ranged sync XHR)
const files = [];
const BLOCK = 256 * 1024;
const MAX_BLOCKS = 384; // ~96 MiB of cached blocks
const cache = new Map(); // `${id}:${block}` -> Uint8Array (Map order = LRU order)
const reader = typeof FileReaderSync !== 'undefined' ? new FileReaderSync() : null;
let bytesRead = 0;

function fetchRange(f, start, end) {
  bytesRead += end - start;
  if (f.file) return new Uint8Array(reader.readAsArrayBuffer(f.file.slice(start, end)));
  const xhr = new XMLHttpRequest();
  xhr.open('GET', f.url, false);
  xhr.responseType = 'arraybuffer';
  xhr.setRequestHeader('Range', `bytes=${start}-${end - 1}`);
  xhr.send();
  if (xhr.status !== 206 && xhr.status !== 200) throw new Error(`${f.url}: HTTP ${xhr.status}`);
  return new Uint8Array(xhr.response);
}

function block(id, b) {
  const k = `${id}:${b}`;
  let v = cache.get(k);
  if (v) {
    cache.delete(k);
    cache.set(k, v);
    return v;
  }
  const f = files[id];
  const start = b * BLOCK;
  v = fetchRange(f, start, Math.min(f.size, start + BLOCK));
  cache.set(k, v);
  if (cache.size > MAX_BLOCKS) cache.delete(cache.keys().next().value);
  return v;
}

self.saGameIo = {
  read(id, offset, length) {
    const f = files[id];
    const end = Math.min(f.size, offset + length);
    if (end <= offset) return new Uint8Array(0);
    // large reads (whole files) bypass the block cache
    if (end - offset > 4 * BLOCK) return fetchRange(f, offset, end);
    const out = new Uint8Array(end - offset);
    for (let p = offset; p < end; ) {
      const b = Math.floor(p / BLOCK);
      const blk = block(id, b);
      const from = p - b * BLOCK;
      const n = Math.min(blk.length - from, end - p);
      out.set(blk.subarray(from, from + n), p - offset);
      p += n;
    }
    return out;
  },
};

// ---------------------------------------------------------------- the main loop ---
let pending = () => false;
let stopped = false;

function status(extra) {
  post('status', {
    iterations: sa.iterations(),
    state: sa.game_state(),
    presents: sa.in_game_presents(),
    mib: Math.round(bytesRead / 1048576),
    ...extra,
  });
}

function tick() {
  if (stopped) return;
  const end = performance.now() + 12;
  try {
    do {
      if (!sa.frame()) {
        stopped = true;
        status({ done: 'the main loop stopped (see the log)' });
        return;
      }
    } while (performance.now() < end && !pending());
  } catch (e) {
    stopped = true;
    status({ error: `${e}` });
    console.error(`sa-game trapped: ${e}\n${e.stack || ''}`);
    return;
  }
  status({});
  setTimeout(tick, 0);
}

self.onmessage = async (ev) => {
  const m = ev.data;
  switch (m.type) {
    case 'start':
      try {
        await init();
        for (const [k, v] of Object.entries(m.env || {})) sa.set_env(k, String(v));
        if (m.startInWorld) sa.set_start_in_world();
        for (const f of m.files) {
          const id = files.length;
          files.push(f.file ? { size: f.size, file: f.file } : { size: f.size, url: f.url });
          sa.add_game_file(f.path, id, f.size);
        }
        post('log', { level: 'log', text: `${files.length} game files registered` });
        const gpu = await sa.init_gpu(m.canvas);
        post('log', { level: 'log', text: `GPU: ${gpu}` });
        status({ phase: 'booting' });
        sa.boot();
        status({ phase: 'running' });
        pending = presentPending();
        setTimeout(tick, 0);
      } catch (e) {
        stopped = true;
        console.error(`sa-game: ${e}\n${e.stack || ''}`);
        status({ error: `${e}` });
      }
      break;
    case 'key':
      sa.key(m.code, m.down, !!m.repeat);
      if (m.text) sa.text(m.text);
      break;
    case 'mouse':
      sa.mouse_move(m.dx, m.dy);
      break;
    case 'button':
      sa.mouse_button(m.button, m.down);
      break;
    case 'wheel':
      sa.wheel(m.lines);
      break;
    case 'blur':
      sa.blur();
      break;
  }
};

// true once a frame was presented since the last call (the task then ends, so the browser shows it)
function presentPending() {
  let last = sa.presents();
  return () => {
    const n = sa.presents();
    if (n !== last) {
      last = n;
      return true;
    }
    return false;
  };
}
