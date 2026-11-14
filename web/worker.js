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

// ---------------------------------------------------------------- audio (WEB-AUDIO) ---
// sa-game's DirectSound mixer (crates/sa-game/src/dsound.rs) calls these once per main-loop iteration. With the
// shared ring buffer (web/audio-worklet.js) and a running AudioContext, the mix tops the ring up to TARGET frames, so
// the DirectSound play cursors advance by exactly what the AudioContext consumed; otherwise they advance in real time
// and the mix goes to the page in chunks (fallback) or nowhere (no output yet).
// Frames kept queued: just over the longest recent gap between two mixes (a main-loop iteration; between iterations
// a 4 ms timer also mixes), at least AUDIO_MIN (~23 ms), at most AUDIO_MAX; the peak decays with a 2 s time constant.
const AUDIO_MIN = 1024;
const AUDIO_MAX = 4096;
let audioTarget = AUDIO_MIN;
let lastMix = 0;
let gapPeak = 0;
function updateTarget() {
  const now = performance.now();
  if (lastMix) {
    const dt = now - lastMix;
    gapPeak = Math.max(dt * 44.1, gapPeak * Math.exp(-dt / 2000));
    audioTarget = Math.min(AUDIO_MAX, Math.max(AUDIO_MIN, Math.ceil(gapPeak * 1.25 + 256)));
  }
  lastMix = now;
}
let ring = null; // { hdr: Int32Array [write, read, running, underruns, target, 0, 0, 0], data: Int16Array, cap }
let audioPort = false; // fallback: chunks posted to the page
let audioRunning = false; // fallback: the page's AudioContext state
let audioTap = null; // ?audiotap=1: copies of every chunk pushed while the output runs, drained by the page
let tapFrames = 0;
let realPlayed = 0;
const ringRunning = () => ring && Atomics.load(ring.hdr, 2) === 1;
self.saGameAudio = {
  want(realFrames) {
    if (ringRunning()) {
      updateTarget();
      Atomics.store(ring.hdr, 4, audioTarget);
      return Math.max(0, audioTarget - self.saGameAudio.queued());
    }
    return Math.round(realFrames);
  },
  push(pcm) {
    const frames = pcm.length / 2;
    if (ringRunning()) {
      const { hdr, data, cap } = ring;
      let w = Atomics.load(hdr, 0);
      const free = cap - ((w - Atomics.load(hdr, 1)) | 0);
      const n = Math.min(frames, free);
      for (let i = 0; i < n; i++) {
        const p = ((w + i) >>> 0) % cap;
        data[p * 2] = pcm[i * 2];
        data[p * 2 + 1] = pcm[i * 2 + 1];
      }
      Atomics.store(hdr, 0, (w + n) | 0);
    } else if (audioPort && audioRunning) {
      const c = pcm.slice();
      post('pcm', { pcm: c });
    } else {
      realPlayed += frames;
      return;
    }
    // pcm is a view of the module's memory, valid during this call only: the tap copies it
    if (audioTap && tapFrames < 44100 * 600) {
      audioTap.push(pcm.slice());
      tapFrames += frames;
    }
  },
  played() {
    return ring ? Atomics.load(ring.hdr, 1) >>> 0 : realPlayed;
  },
  queued() {
    return ring ? Math.max(0, (Atomics.load(ring.hdr, 0) - Atomics.load(ring.hdr, 1)) | 0) : 0;
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
    blocked,
    lastBlocked,
    state: sa.game_state(),
    uiCursor: sa.ui_cursor?.() ?? false,
    presents: sa.in_game_presents(),
    allPresents: sa.presents(),
    mib: Math.round(bytesRead / 1048576),
    ...extra,
  });
}

// an unported callee leaves its main-loop iteration through a JS exception (wasm32 cannot unwind; sa-game's
// web::unported): the shadow stack pointer is put back to its value before the call and sa.recover() ends the
// iteration as natively (a blocked step), then the loop goes on
const BLOCKED_TAG = 'sa-game blocked: ';
let wasm = null;
let blocked = 0;
let lastBlocked = '';
function stackPointer() {
  return wasm && wasm.__stack_pointer instanceof WebAssembly.Global ? wasm.__stack_pointer : null;
}

// the game's audio threads (smooth fades, the stream decoder) run cooperatively after each iteration; an unported
// callee in one ends that thread only (natively its panic ends the thread and the game goes on)
function audioThreads(sp, sp0) {
  try {
    sa.audio_threads();
  } catch (e) {
    const msg = String(e && e.message !== undefined ? e.message : e);
    if (!msg.startsWith(BLOCKED_TAG)) throw e;
    if (sp) sp.value = sp0;
    if (!sa.audio_thread_blocked()) throw new Error(`audio thread left the platform lock taken: ${msg}`);
  }
}

function tick() {
  if (stopped) return;
  const end = performance.now() + 12;
  const sp = stackPointer();
  const sp0 = sp ? sp.value : 0;
  try {
    do {
      let ok;
      try {
        ok = sa.frame();
      } catch (e) {
        const msg = String(e && e.message !== undefined ? e.message : e);
        if (!msg.startsWith(BLOCKED_TAG)) throw e;
        if (sp) sp.value = sp0;
        blocked++;
        // "NAME is not ported; ... (0xADDR)" -> "NAME (0xADDR)"
        const m = msg.slice(BLOCKED_TAG.length).match(/^(\S+) is not ported.*\((0x[0-9a-f]+)/);
        lastBlocked = m ? `${m[1]} (${m[2]})` : msg.slice(BLOCKED_TAG.length, BLOCKED_TAG.length + 120);
        if (!sa.recover()) {
          stopped = true;
          status({ done: `stopped after a blocked iteration: ${msg.slice(BLOCKED_TAG.length)}` });
          return;
        }
        ok = true;
      }
      if (ok) audioThreads(sp, sp0);
      if (!ok) {
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
        wasm = await init();
        if (!stackPointer()) console.warn('the module does not export __stack_pointer: blocked iterations leak their stack');
        for (const [k, v] of Object.entries(m.env || {})) sa.set_env(k, String(v));
        if (m.startInWorld) sa.set_start_in_world();
        if (m.audio?.sab) {
          const data = new Int16Array(m.audio.sab, 32);
          ring = { hdr: new Int32Array(m.audio.sab, 0, 8), data, cap: data.length / 2 };
        } else if (m.audio) {
          audioPort = true;
        }
        if (m.audioTap) audioTap = [];
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
        // between iterations the mix keeps the ring near its target (the module is idle then)
        setInterval(() => {
          if (!stopped && ringRunning()) sa.audio_pump();
        }, 4);
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
    case 'mousepos':
      sa.mouse_pos(m.x, m.y);
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
    case 'audioState':
      audioRunning = !!m.running;
      break;
    case 'audioTap': {
      // the PCM pushed since the last request (evidence runs, ?audiotap=1)
      const chunks = audioTap ?? [];
      const n = chunks.reduce((a, c) => a + c.length, 0);
      const pcm = new Int16Array(n);
      let o = 0;
      for (const c of chunks) {
        pcm.set(c, o);
        o += c.length;
      }
      if (audioTap) audioTap = [];
      const underruns = ring ? Atomics.load(ring.hdr, 3) : 0;
      self.postMessage({ type: 'audioTap', pcm, underruns, state: sa.game_state() }, [pcm.buffer]);
      break;
    }
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
