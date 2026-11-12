// The page (WEB-1): picks the game folder, starts the worker that runs the game, forwards keyboard and mouse input.
// URL query: ?inworld=0 (play the intro), ?auto=1 (scripted Enter / SA_GAME_WALK, for unattended runs), any SA_* name
// (SA_GAME_STUB, SA_GAME_WALK, SA_GAME_SCRIPT_SKIP, ...) is passed to the game as a development variable.
const $ = (id) => document.getElementById(id);
const canvas = $('screen');
const logEl = $('log');
const statusEl = $('status');
const params = new URLSearchParams(location.search);
let worker = null;

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
window.saGame = { status: {}, logs: [] };

function start(files) {
  if (worker) return;
  worker = new Worker('worker.js', { type: 'module' });
  worker.onmessage = (ev) => {
    const m = ev.data;
    if (m.type === 'log') {
      log(m.text, m.level);
      window.saGame.logs.push(m.text);
      if (window.saGame.logs.length > 2000) window.saGame.logs.shift();
    } else if (m.type === 'status') {
      window.saGame.status = m;
      const st = STATES[m.state] ?? m.state;
      statusEl.textContent = m.error ? `error: ${m.error}` : m.done ? m.done :
        `${m.phase ?? ''} gGameState ${st}, ${m.iterations} iterations, ${m.presents} in-game frames, ${m.mib} MiB read`;
      statusEl.className = m.error ? 'err' : '';
    }
  };
  worker.onerror = (e) => log(`worker error: ${e.message}`, 'error');
  const env = {};
  for (const [k, v] of params) if (k.startsWith('SA_')) env[k] = v;
  if (params.get('auto') === '1') env.SA_WEB_AUTO_INPUT = '1';
  const off = canvas.transferControlToOffscreen();
  worker.postMessage({ type: 'start', canvas: off, files, env, startInWorld: $('inworld').checked }, [off]);
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

// mouse: click locks the pointer (DirectInput exclusive mode: relative motion)
canvas.addEventListener('click', () => { if (worker && document.pointerLockElement !== canvas) canvas.requestPointerLock?.(); });
document.addEventListener('mousemove', (e) => {
  if (worker && document.pointerLockElement === canvas) worker.postMessage({ type: 'mouse', dx: e.movementX, dy: e.movementY });
});
canvas.addEventListener('mousedown', (e) => worker?.postMessage({ type: 'button', button: e.button, down: true }));
window.addEventListener('mouseup', (e) => worker?.postMessage({ type: 'button', button: e.button, down: false }));
canvas.addEventListener('contextmenu', (e) => e.preventDefault());
canvas.addEventListener('wheel', (e) => {
  e.preventDefault();
  worker?.postMessage({ type: 'wheel', lines: -Math.sign(e.deltaY) });
}, { passive: false });
