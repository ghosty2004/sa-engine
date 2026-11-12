// The page (WEB-1): picks the game folder, starts the worker that runs the game, forwards keyboard and mouse input.
// URL query: ?inworld=0 (play the intro), ?backend=gl|webgpu, ?auto=1 (scripted Enter / SA_GAME_WALK, for unattended runs), any SA_* name
// (SA_GAME_STUB, SA_GAME_WALK, SA_GAME_SCRIPT_SKIP, ...) is passed to the game as a development variable.
// &stub=ADDR=RET/N,... replaces the default exploration stubs (the play package's, DEFAULT_STUBS); &stub=none drops them.
const $ = (id) => document.getElementById(id);
const canvas = $('screen');
const logEl = $('log');
const statusEl = $('status');
const params = new URLSearchParams(location.search);
let worker = null;

// the exploration defaults of the native play package (target/play/play.sh): unported mission-script opcodes are
// skipped, and these functions return a constant (ADDRESS=RETURN/stack dwords popped; a leading ! replaces a port:
// 0x5efca0 turns off ped attractors, whose task tree is still being ported). Web only: 0x517bf0
// (CIdleCam::ProcessFOVZoom, thiscall, one float) does nothing, so the idle camera that starts when the player stands
// still for a while does not block every iteration (the loop stops after three identical blocked iterations).
const DEFAULT_STUBS = '0x421a50=0/0,!0x5efca0=0/4,0x517bf0=0/1';

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

function start(files) {
  if (worker) return;
  worker = new Worker('worker.js', { type: 'module' });
  worker.onmessage = (ev) => {
    const m = ev.data;
    if (m.type === 'logs') {
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
    }
  };
  worker.onerror = (e) => log(`worker error: ${e.message}`, 'error');
  const env = {};
  env.SA_GAME_SCRIPT_SKIP = '1';
  const stub = params.get('stub');
  if (stub !== 'none') env.SA_GAME_STUB = stub ?? DEFAULT_STUBS;
  for (const [k, v] of params) if (k.startsWith('SA_')) env[k] = v;
  if (params.get('auto') === '1') env.SA_WEB_AUTO_INPUT = '1';
  const off = canvas.transferControlToOffscreen();
  worker.postMessage({ type: 'start', canvas: off, files, env, startInWorld: $('inworld').checked, backend: params.get('backend') ?? '' }, [off]);
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

// mouse: a click locks the pointer (DirectInput exclusive mode: relative motion). That click is not a game button
// press (natively sa-game captures the pointer itself, no click reaches the game for it): otherwise capturing the mouse
// in the world would also punch. Buttons reach the game only while the pointer is locked.
canvas.addEventListener('mousedown', (e) => {
  if (!worker) return;
  if (document.pointerLockElement !== canvas) {
    canvas.requestPointerLock?.();
    return;
  }
  worker.postMessage({ type: 'button', button: e.button, down: true });
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
