// The page (WEB-1): picks the game folder, starts the worker that runs the game, forwards keyboard and mouse input.
// URL query: ?inworld=0 (play the intro), ?backend=gl|webgpu, ?auto=1 (scripted Enter / SA_GAME_WALK, for unattended runs), any SA_* name
// (SA_GAME_STUB, SA_GAME_WALK, SA_GAME_SCRIPT_SKIP, ...) is passed to the game as a development variable.
// ?samp=HOST:PORT[&proxy=ws://127.0.0.1:7781][&name=NAME]: join a SA:MP / open.mp server through sa-udp-proxy (build with FEATURES=samp).
// ?audiotap=1 keeps a copy of the audio output for evidence runs (window.saGame.audioTap(), tools/web/headless.mjs --audio).
// &stub=ADDR=RET/N,... replaces the default exploration stubs (the play package's, DEFAULT_STUBS); &stub=none drops them.
const $ = (id) => document.getElementById(id);
const canvas = $('screen');
const logEl = $('log');
const statusEl = $('status');
const params = new URLSearchParams(location.search);
let worker = null;

// the exploration defaults of the native play package (target/play/play.sh): unported mission-script opcodes are
// skipped, and these functions return a constant (ADDRESS=RETURN/stack dwords popped; a leading ! replaces a port:
// 0x5efca0 turns off ped attractors, whose task tree is still being ported).
const DEFAULT_STUBS = '0x421a50=0/0,!0x5efca0=0/4';

if (params.get('inworld') === '0') $('inworld').checked = false;

function log(text, level = 'log') {
  const line = document.createElement('div');
  if (level === 'error') line.className = 'err';
  line.textContent = text;
  logEl.appendChild(line);
  while (logEl.childNodes.length > 400) logEl.removeChild(logEl.firstChild);
  logEl.scrollTop = logEl.scrollHeight;
}

const STATES = ['INITIAL', 'LOGO', 'PLAYING_LOGO', 'TITLE', 'PLAYING_INTRO', 'FRONTEND_LOADING', 'FRONTEND_LOADED',
  'FRONTEND_IDLE', 'LOADING_STARTED', 'IDLE', 'EXIT'];
window.saGame = { status: {}, logs: [], logBase: 0, snapshots: [], snapshot: () => worker?.postMessage({ type: 'snapshot' }) };

// ---------------------------------------------------------------- audio (WEB-AUDIO) ---
// The game's DirectSound mix plays through an AudioWorklet (web/audio-worklet.js). With cross-origin isolation (the
// dev server sends COOP/COEP) the worker writes a SharedArrayBuffer ring the worklet reads, and the game's play
// cursors follow the AudioContext's consumption; otherwise the worker posts chunks here and they go to the worklet's
// port. Browsers start an AudioContext only after a user gesture: the first click (the pointer-lock click) or key
// resumes it.
const RING_FRAMES = 32768; // power of two (the indices wrap at 2^32)
const audio = { ctx: null, gain: null, node: null, sab: null, mode: 'off', tapWaiters: [] };
let volume = Number(localStorage.getItem('saGameVolume') ?? '80');
let muted = localStorage.getItem('saGameMuted') === '1';

function applyVolume() {
  if (audio.gain) audio.gain.gain.value = muted ? 0 : (volume / 100) ** 2;
  $('mute').textContent = muted ? 'unmute' : 'mute';
  $('vol').value = String(volume);
}

function audioStatus() {
  const st = audio.ctx ? audio.ctx.state : 'off';
  const running = st === 'running';
  if (audio.sab) Atomics.store(new Int32Array(audio.sab, 0, 8), 2, running ? 1 : 0);
  else worker?.postMessage({ type: 'audioState', running });
  const shown = st === 'running' ? 'running' : st === 'off' ? 'off' : 'suspended (click the picture)';
  const d = audioDelay();
  $('audio').textContent = `audio: ${shown}${audio.ctx ? ` (${audio.mode}${running && d ? `, delay ${d.totalMs} ms` : ''})` : ''}`;
  $('audio').className = running ? 'on' : 'off';
  window.saGame.audio = { state: st, mode: audio.mode, ...d };
}

// end-to-end delay of a sound the game starts now: the frames queued in the ring (mixed, not yet played) plus the
// context's own latency (baseLatency: the worklet quantum / graph; outputLatency: the device)
function audioDelay() {
  if (!audio.ctx || !audio.sab) return null;
  const h = new Int32Array(audio.sab, 0, 8);
  const ringMs = (((Atomics.load(h, 0) - Atomics.load(h, 1)) | 0) / 44.1);
  const baseMs = (audio.ctx.baseLatency ?? 0) * 1000;
  const outMs = (audio.ctx.outputLatency ?? 0) * 1000;
  return { ringMs: Math.round(ringMs), targetMs: Math.round(Atomics.load(h, 4) / 44.1), baseMs: Math.round(baseMs),
    outputMs: Math.round(outMs), totalMs: Math.round(ringMs + baseMs + outMs), underruns: Atomics.load(h, 3) };
}
setInterval(() => { if (audio.ctx) audioStatus(); }, 1000);

// called at start: the context starts suspended unless the browser allows autoplay
async function initAudio() {
  try {
    audio.ctx = new AudioContext({ sampleRate: 44100, latencyHint: 'interactive' });
  } catch (e) {
    log(`audio: no AudioContext (${e})`, 'error');
    return null;
  }
  audio.ctx.onstatechange = audioStatus;
  try {
    await audio.ctx.audioWorklet.addModule('audio-worklet.js');
  } catch (e) {
    log(`audio: the worklet did not load (${e})`, 'error');
    audio.ctx = null;
    return null;
  }
  let opts = {};
  if (self.crossOriginIsolated && typeof SharedArrayBuffer !== 'undefined') {
    audio.sab = new SharedArrayBuffer(32 + RING_FRAMES * 4);
    opts = { sab: audio.sab };
    audio.mode = 'shared ring';
  } else {
    audio.mode = 'postMessage';
  }
  audio.node = new AudioWorkletNode(audio.ctx, 'sa-game-audio', { numberOfInputs: 0, outputChannelCount: [2], processorOptions: opts });
  audio.gain = audio.ctx.createGain();
  audio.node.connect(audio.gain).connect(audio.ctx.destination);
  applyVolume();
  audioStatus();
  return audio.sab ? { sab: audio.sab } : { port: true };
}

function resumeAudio() {
  if (audio.ctx && audio.ctx.state !== 'running') audio.ctx.resume().then(audioStatus, () => {});
}
window.addEventListener('pointerdown', resumeAudio, true);
window.addEventListener('keydown', resumeAudio, true);
$('mute').onclick = () => {
  muted = !muted;
  localStorage.setItem('saGameMuted', muted ? '1' : '0');
  applyVolume();
};
$('vol').oninput = () => {
  volume = Number($('vol').value);
  localStorage.setItem('saGameVolume', String(volume));
  applyVolume();
};
applyVolume();

async function start(files) {
  if (worker) return;
  worker = new Worker('worker.js', { type: 'module' });
  // evidence runs (?audiotap=1): the PCM produced since the last call, as base64 little-endian 16-bit stereo
  window.saGame.audioTap = () => new Promise((res) => {
    audio.tapWaiters.push(res);
    worker.postMessage({ type: 'audioTap' });
  });
  worker.onmessage = (ev) => {
    const m = ev.data;
    if (m.type === 'pcm') {
      audio.node?.port.postMessage(m.pcm, [m.pcm.buffer]);
    } else if (m.type === 'audioTap') {
      const bytes = new Uint8Array(m.pcm.buffer);
      let bin = '';
      for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
      audio.tapWaiters.shift()?.({ pcm: btoa(bin), underruns: m.underruns, state: m.state, ctx: audio.ctx?.state ?? 'off' });
    } else if (m.type === 'logs') {
      for (const l of m.lines) {
        log(l.text, l.level);
        window.saGame.logs.push(l.text);
      }
      const extra = window.saGame.logs.length - 4000;
      if (extra > 0) {
        window.saGame.logs.splice(0, extra);
        window.saGame.logBase += extra;
      }
    } else if (m.type === 'panic') {
      log(m.message, 'error');
      window.saGame.logs.push(m.message);
      window.saGame.panic = m.message;
      statusEl.textContent = m.message.split('\n')[0];
      statusEl.className = 'err';
    } else if (m.type === 'snapshot') {
      const bytes = new Uint8Array(m.png);
      let bin = '';
      for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
      const b64 = btoa(bin);
      window.saGame.snapshots.push({ state: m.state, iterations: m.iterations, png: b64 });
    } else if (m.type === 'progress') {
      window.saGame.progress = m;
      statusEl.textContent = `loading... ${m.mib} MiB read`;
    } else if (m.type === 'status') {
      window.saGame.status = m;
      const st = STATES[m.state] ?? m.state;
      statusEl.textContent = m.error ? `error: ${m.error}` : m.done ? m.done :
        `${m.phase ?? ''} gGameState ${st}, ${m.iterations} iterations${m.blocked ? ` (${m.blocked} blocked)` : ''}, ${m.presents} in-game frames, ${m.mib} MiB read`;
      statusEl.className = m.error ? 'err' : '';
      $('ministatus').textContent = m.error ? `error: ${m.error}` : m.blocked ? `${m.blocked} blocked` : '';
      inMenu = m.state !== 9;
      canvas.classList.toggle('menu', inMenu && document.pointerLockElement !== canvas);
      $('blocked').textContent = m.blocked ? `blocked iterations: ${m.blocked}, last: ${m.lastBlocked} (not ported yet)` : '';
    }
  };
  worker.onerror = (e) => log(`worker error: ${e.message}`, 'error');
  const env = {};
  env.SA_GAME_SCRIPT_SKIP = '1';
  const stub = params.get('stub');
  if (stub !== 'none') env.SA_GAME_STUB = stub ?? DEFAULT_STUBS;
  for (const [k, v] of params) if (k.startsWith('SA_')) env[k] = v;
  if (params.get('auto') === '1') env.SA_WEB_AUTO_INPUT = '1';
  // SA:MP: ?samp=HOST:PORT&proxy=ws://127.0.0.1:7781&name=NAME joins that server through sa-udp-proxy
  // (docs/samp/web-proxy.md); the auth table (third-party data) comes from the local server (serve.sh --samp-auth FILE)
  const samp = params.get('samp');
  if (samp) {
    env.SA_SAMP = samp;
    env.SA_SAMP_PROXY = params.get('proxy') ?? 'ws://127.0.0.1:7781';
    env.SA_SAMP_NAME = params.get('name') ?? 'WebCJ';
    try {
      const r = await fetch(params.get('auth') ?? 'samp-auth.txt');
      if (r.ok) env.SA_SAMP_AUTH = await r.text();
      else log(`SA:MP auth table: HTTP ${r.status} (start the server with --samp-auth FILE)`, 'error');
    } catch (e) {
      log(`SA:MP auth table: ${e}`, 'error');
    }
  }
  const audioOut = await initAudio();
  const off = canvas.transferControlToOffscreen();
  worker.postMessage({ type: 'start', canvas: off, files, env, audio: audioOut, audioTap: params.get('audiotap') === '1', startInWorld: $('inworld').checked, backend: params.get('backend') ?? '' }, [off]);
  canvas.focus();
}

// a folder picked in the page: paths relative to the chosen folder
$('pick').addEventListener('change', (e) => {
  const list = [...e.target.files].map((f) => ({
    path: f.webkitRelativePath.split('/').slice(1).join('/'),
    size: f.size,
    file: f,
  }));
  if (!list.some((f) => f.path.toLowerCase() === 'gta_sa.exe')) {
    log('that folder has no gta_sa.exe (choose the GTA San Andreas folder itself)', 'error');
    return;
  }
  start(list);
});

// the folder tools/web/serve.sh --game serves (localhost only)
async function served() {
  const r = await fetch('game-index.json');
  const idx = await r.json();
  return idx.map((f) => ({ path: f.path, size: f.size, url: 'game/' + f.path.split('/').map(encodeURIComponent).join('/') }));
}
fetch('game-index.json').then((r) => r.ok ? r.json() : []).then((idx) => {
  if (idx.length) {
    $('served').hidden = false;
    $('served').onclick = async () => start(await served());
    if (params.get('autostart') === '1') served().then(start);
  }
}).catch(() => {});

// ---------------------------------------------------------------- input ---
// keyboard: physical key codes (layout-independent, as DirectInput / the scan codes see them)
const PASS = new Set(['F5', 'F11', 'F12']);
function onKey(down) {
  return (e) => {
    if (!worker || PASS.has(e.code)) return;
    e.preventDefault();
    worker.postMessage({ type: 'key', code: e.code, down, repeat: e.repeat, text: down && e.key.length === 1 ? e.key : '' });
  };
}
window.addEventListener('keydown', onKey(true));
window.addEventListener('keyup', onKey(false));
window.addEventListener('blur', () => worker?.postMessage({ type: 'blur' }));

// full window (?full=1) and fullscreen: CSS scales the canvas (the game keeps its video mode, letterboxed at 4:3)
let inMenu = true;
function setFull(on) {
  document.body.classList.toggle('full', on);
  $('unfull').textContent = document.fullscreenElement ? 'Exit fullscreen' : 'Exit full window';
  canvas.focus();
}
$('fullwin').onclick = () => setFull(true);
$('unfull').onclick = () => (document.fullscreenElement ? document.exitFullscreen() : setFull(false));
$('fullscreen').onclick = () => $('stage').requestFullscreen?.().then(() => setFull(false)).catch((e) => log(`fullscreen: ${e}`, 'error'));
document.addEventListener('fullscreenchange', () => {
  $('unfull').textContent = document.fullscreenElement ? 'Exit fullscreen' : 'Exit full window';
  canvas.focus();
});
if (params.get('full') === '1') setFull(true);

// the pointer position in the game's client pixels (the canvas is scaled by CSS)
function gamePos(e) {
  const r = canvas.getBoundingClientRect();
  return [Math.round((e.clientX - r.left) * canvas.width / r.width), Math.round((e.clientY - r.top) * canvas.height / r.height)];
}

// menus (gGameState other than IDLE): no pointer lock; the absolute position goes to the game as WM_MOUSEMOVE (the
// menu draws its own cursor there, the OS cursor is hidden over the picture) and clicks are menu clicks.
canvas.addEventListener('mousemove', (e) => {
  if (worker && inMenu && document.pointerLockElement !== canvas) {
    const [x, y] = gamePos(e);
    worker.postMessage({ type: 'mousepos', x, y });
  }
});

// mouse: in the world a click locks the pointer (DirectInput exclusive mode: relative motion). That click is not a game button
// press (natively sa-game captures the pointer itself, no click reaches the game for it): otherwise capturing the mouse
// in the world would also punch. Buttons reach the game only while the pointer is locked.
canvas.addEventListener('mousedown', (e) => {
  if (!worker) return;
  if (document.pointerLockElement !== canvas) {
    if (inMenu) {
      const [x, y] = gamePos(e);
      worker.postMessage({ type: 'mousepos', x, y });
      worker.postMessage({ type: 'button', button: e.button, down: true });
      return;
    }
    canvas.requestPointerLock?.();
    return;
  }
  worker.postMessage({ type: 'button', button: e.button, down: true });
});
// the capture indicator (pointer lock comes and goes with clicks, Esc and focus changes)
document.addEventListener('pointerlockchange', () => {
  const on = document.pointerLockElement === canvas;
  $('mouse').textContent = `mouse captured: ${on ? 'yes' : 'no'}`;
  $('mouse').className = on ? 'on' : 'off';
  window.saGame.mouseCaptured = on;
});
document.addEventListener('pointerlockerror', () => {
  $('mouse').textContent = 'mouse captured: no (the browser refused the pointer lock; click the picture again)';
});
document.addEventListener('mousemove', (e) => {
  if (worker && document.pointerLockElement === canvas) {
    worker.postMessage({ type: 'mouse', dx: e.movementX, dy: e.movementY });
    window.saGame.mouseX = (window.saGame.mouseX ?? 0) + e.movementX;
  }
});
window.addEventListener('mouseup', (e) => worker?.postMessage({ type: 'button', button: e.button, down: false }));
canvas.addEventListener('contextmenu', (e) => e.preventDefault());
canvas.addEventListener('wheel', (e) => {
  e.preventDefault();
  worker?.postMessage({ type: 'wheel', lines: -Math.sign(e.deltaY) });
}, { passive: false });
