// The game's worker (WEB-1): runs sa-game's wasm module, reads the user's game folder synchronously, draws into the
// OffscreenCanvas the page transferred, and turns the page's input messages into platform input.
import init, * as sa from './pkg/sa_game.js';

const post = (type, data) => self.postMessage({ type, ...data });
Error.stackTraceLimit = 60;

// console lines (the game's println!) also go to the page's log panel, in batches (one message per tick)
let pendingLogs = [];
let lastFlush = 0;
for (const level of ['log', 'warn', 'error']) {
  const orig = console[level].bind(console);
  console[level] = (...a) => {
    orig(...a);
    pendingLogs.push({ level, text: a.map(String).join(' ') });
    if (pendingLogs.length > 500) pendingLogs.splice(0, pendingLogs.length - 500);
    // inside one long iteration nothing else flushes
    if (performance.now() - lastFlush > 500) flushLogs();
  };
}
function flushLogs() {
  lastFlush = performance.now();
  if (pendingLogs.length) {
    post('logs', { lines: pendingLogs });
    pendingLogs = [];
  }
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
  const t0 = performance.now();
  post('progress', { mib: Math.round(bytesRead / 1048576), pending: `${f.url} ${start}-${end}` });
  xhr.send();
  const dt = performance.now() - t0;
  if (dt > 1000) console.warn(`slow read: ${f.url} ${start}-${end}: ${Math.round(dt)} ms`);
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

// progress while one long main-loop iteration runs (CGame::Initialise loads the world in one): JS-only state, no
// call back into the module (it is running)
let lastProgress = 0;
function progress() {
  const t = performance.now();
  if (t - lastProgress < 500) return;
  lastProgress = t;
  flushLogs();
  post('progress', { mib: Math.round(bytesRead / 1048576) });
}

self.saGameIo = {
  // a Rust panic (the module aborts right after): straight to the page
  panic(message) {
    flushLogs();
    post('panic', { message });
  },
  read(id, offset, length) {
    progress();
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
// snapshot requests (evidence runs, the page's "save frame"): the canvas content right after the next present, read
// before the task ends (a WebGPU canvas keeps its drawing buffer until then)
let canvas = null;
let snapshotWanted = false;
let presentedNow = () => false;
function takeSnapshot() {
  snapshotWanted = false;
  canvas.convertToBlob({ type: 'image/png' }).then((b) => b.arrayBuffer()).then((buf) => {
    post('snapshot', { png: buf, state: sa.game_state(), iterations: sa.iterations() });
  }).catch((e) => console.warn(`snapshot: ${e}`));
}
let stopped = false;
let phase = 'starting';
let lastStatus = 0;

function status(extra) {
  flushLogs();
  lastStatus = performance.now();
  // after a trap the module's state is torn (a panic aborts mid-call): no more calls into it
  if (extra.error) {
    post('status', { phase, mib: Math.round(bytesRead / 1048576), ...extra });
    return;
  }
  post('status', {
    phase,
    iterations: sa.iterations(),
    state: sa.game_state(),
    presents: sa.in_game_presents(),
    allPresents: sa.presents(),
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
      if (snapshotWanted && sa.presents() > 0 && presentedNow()) takeSnapshot();
    } while (performance.now() < end && !pending());
  } catch (e) {
    stopped = true;
    // the wasm stack (function names) of the trap, then the status (which flushes the log to the page)
    const frames = (e.stack || '').split('\n').filter((l) => l.includes('wasm')).map((l) => l.trim().replace(/^at /, '').split(' (')[0]);
    console.error(`sa-game trapped: ${e}\n  ${frames.join('\n  ')}`);
    status({ error: `${e} in ${frames[0] ?? '?'}` });
    return;
  }
  if (performance.now() - lastStatus > 100) status({});
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
        console.log(`${files.length} game files registered`);
        canvas = m.canvas;
        const gpu = await sa.init_gpu(m.canvas, m.backend || '');
        console.log(`GPU: ${gpu}`);
        phase = 'booting';
        status({});
        sa.boot();
        phase = 'running';
        status({});
        pending = presentPending();
        presentedNow = presentPending();
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
    case 'snapshot':
      snapshotWanted = true;
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
