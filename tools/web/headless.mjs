#!/usr/bin/env node
// Headless evidence run of the web build (WEB-1): starts Chromium offscreen (its own profile under target/), opens the
// page served by tools/web/serve.sh, and drives it over the DevTools protocol (no puppeteer): screenshots, status
// polling, and real keyboard events (Input.dispatchKeyEvent reaches the page's keydown listener, the same path as a
// physical key).
//
//   node tools/web/headless.mjs --url 'http://localhost:8642/?autostart=1' --out target/web1-evidence/run1 \
//        [--seconds 240] [--every 10] [--walk-after 8 --walk-for 12] [--enter-every 0] [--chrome chromium]
//
// --walk-after S: S seconds after gGameState first reaches IDLE (in game), W goes down for --walk-for seconds.
// --enter-every S: presses Enter every S seconds until the game is in IDLE (menus) with real key events.
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync, appendFileSync } from 'node:fs';
import { setTimeout as sleep } from 'node:timers/promises';

const arg = (k, d) => {
  const i = process.argv.indexOf(`--${k}`);
  return i > 0 ? process.argv[i + 1] : d;
};
const url = arg('url', 'http://localhost:8642/?autostart=1');
const out = arg('out', 'target/web1-evidence/run');
const seconds = Number(arg('seconds', '240'));
const every = Number(arg('every', '10'));
const walkAfter = arg('walk-after') !== undefined ? Number(arg('walk-after')) : null;
const walkFor = Number(arg('walk-for', '10'));
const enterEvery = Number(arg('enter-every', '0'));
const chrome = arg('chrome', 'chromium');
const port = Number(arg('port', '9333'));
const extraFlags = (arg('flags', '') || '').split(' ').filter(Boolean);
mkdirSync(out, { recursive: true });
const logFile = `${out}/log.txt`;
writeFileSync(logFile, '');
const note = (s) => {
  console.log(s);
  appendFileSync(logFile, s + '\n');
};

const flags = [
  '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${process.cwd()}/target/web1-chrome`,
  '--no-first-run', '--no-default-browser-check', '--enable-unsafe-webgpu', '--enable-features=Vulkan',
  '--ignore-gpu-blocklist', '--window-size=820,700', ...extraFlags, 'about:blank',
];
const proc = spawn(chrome, flags, { stdio: ['ignore', 'ignore', 'pipe'] });
proc.stderr.on('data', (d) => appendFileSync(`${out}/chrome-stderr.txt`, d));
const kill = () => { try { proc.kill('SIGTERM'); } catch {} };
process.on('exit', kill);

let ver;
for (let i = 0; i < 50 && !ver; i++) {
  await sleep(200);
  try { ver = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json(); } catch {}
}
if (!ver) { note('chromium did not start'); process.exit(1); }
const ws = new WebSocket(ver.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener('open', r, { once: true }));
let nextId = 1;
const waiting = new Map();
const handlers = [];
ws.addEventListener('message', (ev) => {
  const m = JSON.parse(ev.data);
  if (m.id && waiting.has(m.id)) {
    waiting.get(m.id)(m);
    waiting.delete(m.id);
  } else for (const h of handlers) h(m);
});
const send = (method, params = {}, sessionId) => new Promise((res) => {
  const id = nextId++;
  waiting.set(id, res);
  ws.send(JSON.stringify({ id, method, params, sessionId }));
});

const { result: { targetId } } = await send('Target.createTarget', { url: 'about:blank' });
const { result: { sessionId } } = await send('Target.attachToTarget', { targetId, flatten: true });
const s = (m, p) => send(m, p, sessionId);
await s('Page.enable');
await s('Runtime.enable');
handlers.push((m) => {
  if (m.method === 'Runtime.exceptionThrown') note(`page exception: ${JSON.stringify(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text)}`);
});
await s('Emulation.setDeviceMetricsOverride', { width: 820, height: 700, deviceScaleFactor: 1, mobile: false });
await s('Page.navigate', { url });

const evalJs = async (expr) => (await s('Runtime.evaluate', { expression: expr, returnByValue: true })).result?.result?.value;
const shot = async (name) => {
  const r = await s('Page.captureScreenshot', { format: 'png', clip: { x: 10, y: 50, width: 800, height: 600, scale: 1 } });
  if (r.result?.data) {
    writeFileSync(`${out}/${name}.png`, Buffer.from(r.result.data, 'base64'));
    note(`screenshot ${out}/${name}.png`);
  }
};
const key = (type, code, keyName, vk) => s('Input.dispatchKeyEvent', { type, code, key: keyName, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });

const t0 = Date.now();
let lastShot = -every, inGameAt = null, walking = false, walked = false, lastEnter = 0, logsSeen = 0, lastState = -1;
while ((Date.now() - t0) / 1000 < seconds) {
  await sleep(500);
  const t = (Date.now() - t0) / 1000;
  const st = (await evalJs('JSON.stringify(window.saGame?.status ?? {})')) ?? '{}';
  const logs = (await evalJs(`JSON.stringify((window.saGame?.logs ?? []).slice(${logsSeen}))`)) ?? '[]';
  const ls = JSON.parse(logs);
  logsSeen += ls.length;
  for (const l of ls) appendFileSync(`${out}/game-log.txt`, l + '\n');
  const status = JSON.parse(st);
  if (status.state !== lastState) {
    note(`t=${t.toFixed(1)} s: gGameState ${lastState} -> ${status.state} (${JSON.stringify(status)})`);
    if (lastState !== -1) await shot(`state-${status.state}-t${Math.round(t)}`);
    lastState = status.state;
  }
  if (status.error || status.done) {
    note(`t=${t.toFixed(1)} s: ${status.error ?? status.done}`);
    await shot(`end-t${Math.round(t)}`);
    break;
  }
  if (status.state === 9 && inGameAt === null) inGameAt = t;
  if (enterEvery > 0 && status.state !== 9 && t - lastEnter >= enterEvery && status.phase === 'running') {
    lastEnter = t;
    await key('keyDown', 'Enter', 'Enter', 13);
    await sleep(150);
    await key('keyUp', 'Enter', 'Enter', 13);
    note(`t=${t.toFixed(1)} s: Enter (real key event)`);
  }
  if (walkAfter !== null && inGameAt !== null && !walking && !walked && t - inGameAt >= walkAfter) {
    await shot(`walk-before-t${Math.round(t)}`);
    await key('keyDown', 'KeyW', 'w', 87);
    walking = t;
    note(`t=${t.toFixed(1)} s: W down (real key event)`);
  }
  if (walking !== false && !walked && t - walking >= walkFor) {
    await key('keyUp', 'KeyW', 'w', 87);
    walked = true;
    note(`t=${t.toFixed(1)} s: W up`);
    await shot(`walk-after-t${Math.round(t)}`);
  }
  if (t - lastShot >= every) {
    lastShot = t;
    await shot(`t${String(Math.round(t)).padStart(4, '0')}-state${status.state}`);
  }
}
note(`final status: ${JSON.stringify(await evalJs('window.saGame?.status'))}`);
await shot('final');
ws.close();
kill();
process.exit(0);
