#!/usr/bin/env node
// Headless evidence run of the web build (WEB-1): starts Chromium offscreen (its own profile under target/), opens the
// page served by tools/web/serve.sh, and drives it over the DevTools protocol (no puppeteer): screenshots, status
// polling, and real keyboard events (Input.dispatchKeyEvent reaches the page's keydown listener, the same path as a
// physical key).
//
//   node tools/web/headless.mjs --url 'http://localhost:8642/?autostart=1' --out target/web1-evidence/run1 \
//        [--seconds 240] [--every 10] [--walk-after 8 --walk-for 12] [--enter-every 0] [--chrome chromium]
//
// --walk-after S: S seconds after gGameState first reaches IDLE (in game), W goes down for --walk-for seconds; or
// --walk 'w:20,wa:6,w:10,wd:6,w:20': held key sets (letters of W/A/S/D) and seconds, in order (real key events).
// --enter-every S: presses Enter every S seconds until the game is in IDLE (menus) with real key events.
// --walk letters l / r: mouse-look left / right while that segment lasts (a real click on the canvas first locks the
// pointer, then real mouse moves; the page does not pass that click to the game).
// --screencast: records the page (status line included) with Page.startScreencast into OUT/cast/ (JPEG frames +
// frames.jsonl with their times, events.jsonl with the status timeline); tools/web/make_video.py assembles an MP4.
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
const walkPlan = (arg('walk', '') || `w:${walkFor}`).split(',').map((x) => {
  const [k, d] = x.split(':');
  return { keys: [...k.toLowerCase()], secs: Number(d) };
});
const walkTotal = walkPlan.reduce((a, p) => a + p.secs, 0);
const lookPx = Number(arg('look-px', '40'));
const KEYS = { w: ['KeyW', 'w', 87], a: ['KeyA', 'a', 65], s: ['KeyS', 's', 83], d: ['KeyD', 'd', 68] };
let held = new Set();
let locked = false, mx = 0, my = 0;
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
  '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${process.cwd()}/target/web1-chrome-${port}`,
  '--no-first-run', '--no-default-browser-check', '--enable-unsafe-webgpu', '--enable-features=Vulkan',
  '--ignore-gpu-blocklist', ...(arg('gpu', 'swiftshader') === 'swiftshader' ? ['--use-angle=swiftshader', '--use-webgpu-adapter=swiftshader'] : ['--use-angle=vulkan', '--enable-features=Vulkan,VulkanFromANGLE']), '--window-size=820,700', ...extraFlags, 'about:blank',
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
const cast = process.argv.includes('--screencast');
const castDir = `${out}/cast`;
let castN = 0;
if (cast) {
  mkdirSync(castDir, { recursive: true });
  writeFileSync(`${castDir}/frames.jsonl`, '');
  writeFileSync(`${castDir}/events.jsonl`, '');
  handlers.push((m) => {
    if (m.method !== 'Page.screencastFrame' || m.sessionId !== sessionId) return;
    const file = `f${String(castN++).padStart(6, '0')}.jpg`;
    writeFileSync(`${castDir}/${file}`, Buffer.from(m.params.data, 'base64'));
    appendFileSync(`${castDir}/frames.jsonl`, JSON.stringify({ file, ts: m.params.metadata.timestamp ?? Date.now() / 1000 }) + '\n');
    send('Page.screencastFrameAck', { sessionId: m.params.sessionId }, sessionId);
  });
  await s('Page.startScreencast', { format: 'jpeg', quality: 85, maxWidth: 820, maxHeight: 700, everyNthFrame: 1 });
}
const castEvent = (e) => { if (cast) appendFileSync(`${castDir}/events.jsonl`, JSON.stringify({ ts: Date.now() / 1000, ...e }) + '\n'); };
await s('Page.navigate', { url });
castEvent({ event: 'navigate' });

const evalJs = async (expr) => (await s('Runtime.evaluate', { expression: expr, returnByValue: true })).result?.result?.value;
const shot = async (name) => {
  const rect = JSON.parse((await evalJs('JSON.stringify(document.getElementById("screen").getBoundingClientRect())')) ?? '{"x":0,"y":0,"width":800,"height":600}');
  const full = process.argv.includes('--full-page');
  const clip = full ? { x: 0, y: 0, width: 820, height: 700, scale: 1 } : { x: rect.x, y: rect.y, width: rect.width, height: rect.height, scale: 1 };
  // a canvas the compositor cannot show (headless with a real GPU) can stall the capture: give up after 5 s
  const r = await Promise.race([s('Page.captureScreenshot', { format: 'png', clip }), sleep(Number(arg("shot-timeout", "5")) * 1000).then(() => ({}))]);
  if (r.result?.data) {
    writeFileSync(`${out}/${name}.png`, Buffer.from(r.result.data, 'base64'));
    note(`screenshot ${out}/${name}.png`);
  }
};
const key = (type, code, keyName, vk) => s('Input.dispatchKeyEvent', { type, code, key: keyName, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });

// --stack-on-hang S: when the status has not changed for S seconds (a main-loop iteration that does not return), the
// worker is paused through the debugger and its call stack (wasm function names) is written to the log.
const hangSecs = Number(arg('stack-on-hang', '0'));
const workers = [];
handlers.push((m) => {
  if (m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') workers.push(m.params.sessionId);
});
await s('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
async function dumpStacks(tag) {
  const { result } = await send('Target.getTargets');
  for (const ti of result.targetInfos.filter((x) => x.type === 'worker' && !x.attached)) {
    const r = await send('Target.attachToTarget', { targetId: ti.targetId, flatten: true });
    if (r.result?.sessionId) workers.push(r.result.sessionId);
  }
  note(`${tag}: ${workers.length} worker session(s)`);
  for (const ws of workers) {
    const paused = new Promise((res) => handlers.push((m) => { if (m.method === 'Debugger.paused' && m.sessionId === ws) res(m.params); }));
    await send('Debugger.enable', {}, ws);
    await send('Debugger.pause', {}, ws);
    const p = await Promise.race([paused, sleep(120000).then(() => null)]);
    if (!p) { note(`${tag}: worker did not pause`); continue; }
    note(`${tag}: worker stack:\n` + p.callFrames.map((f) => `  ${f.functionName}`).join('\n'));
    await send('Debugger.resume', {}, ws);
  }
}
let lastChange = Date.now(), lastStatusText = '', lastProgressText = '', dumped = false, lastProgressNote = -10;

const t0 = Date.now();
let lastShot = -every, inGameAt = null, walking = false, walked = false, lastEnter = 0, logsSeen = 0, lastState = -1;
while ((Date.now() - t0) / 1000 < seconds) {
  await sleep(500);
  const t = (Date.now() - t0) / 1000;
  const st = (await evalJs('JSON.stringify(window.saGame?.status ?? {})')) ?? '{}';
  const pr = (await evalJs('JSON.stringify(window.saGame?.progress ?? {})')) ?? '{}';
  if (pr !== lastProgressText) { lastProgressText = pr; lastChange = Date.now(); if (t - lastProgressNote >= 5) { lastProgressNote = t; note(`t=${t.toFixed(1)} s: progress ${pr}`); } }
  const logs = (await evalJs(`JSON.stringify((window.saGame?.logs ?? []).slice(Math.max(0, ${logsSeen} - (window.saGame?.logBase ?? 0))))`)) ?? '[]';
  const ls = JSON.parse(logs);
  logsSeen = Math.max(logsSeen, (await evalJs('window.saGame?.logBase ?? 0')) ?? 0) + ls.length;
  for (const l of ls) appendFileSync(`${out}/game-log.txt`, l + '\n');
  const status = JSON.parse(st);
  const panicMsg = await evalJs('window.saGame?.panic ?? null');
  if (panicMsg) {
    note(`t=${t.toFixed(1)} s: PANIC ${panicMsg}`);
    await shot(`panic-t${Math.round(t)}`);
    break;
  }
  if (st !== lastStatusText) { lastStatusText = st; lastChange = Date.now(); castEvent({ status }); }
  if (hangSecs > 0 && !dumped && (Date.now() - lastChange) / 1000 > hangSecs) {
    dumped = true;
    await dumpStacks(`t=${t.toFixed(1)} s: no status change for ${hangSecs} s`);
    await shot(`hang-t${Math.round(t)}`);
  }
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
    castEvent({ event: 'enter' });
    await key('keyDown', 'Enter', 'Enter', 13);
    await sleep(150);
    await key('keyUp', 'Enter', 'Enter', 13);
    note(`t=${t.toFixed(1)} s: Enter (real key event)`);
  }
  if (walkAfter !== null && inGameAt !== null && !walking && !walked && t - inGameAt >= walkAfter) {
    await shot(`walk-before-t${Math.round(t)}`);
    walking = t;
    note(`t=${t.toFixed(1)} s: walk starts (${arg('walk', '') || `w:${walkFor}`}, real key events)`);
  }
  if (walking !== false && !walked) {
    // the key set of the plan segment at this time; keys leave and join with real key events
    let at = t - walking, want = [];
    for (const p of walkPlan) {
      if (at < p.secs) { want = p.keys; break; }
      at -= p.secs;
    }
    const look = want.includes('l') ? -1 : want.includes('r') ? 1 : 0;
    want = want.filter((k) => KEYS[k]);
    if (look && !locked) {
      // a real click on the canvas: the page asks for pointer lock (and does not pass this click to the game)
      const r = JSON.parse(await evalJs('JSON.stringify(document.getElementById("screen").getBoundingClientRect())'));
      mx = r.x + r.width / 2; my = r.y + r.height / 2;
      await s('Input.dispatchMouseEvent', { type: 'mousePressed', x: mx, y: my, button: 'left', clickCount: 1 });
      await s('Input.dispatchMouseEvent', { type: 'mouseReleased', x: mx, y: my, button: 'left', clickCount: 1 });
      await sleep(100);
      locked = await evalJs('document.pointerLockElement === document.getElementById("screen")');
      note(`t=${t.toFixed(1)} s: click on the canvas: pointer lock ${locked ? 'on' : 'refused'}`);
      locked = true; // try the moves either way (once)
    }
    if (look) {
      // pointer-lock movement is the difference between successive positions: plans should alternate l and r so
      // the position stays inside the viewport (leaving it jumps back to the centre, one jerk)
      for (let i = 0; i < 5; i++) {
        mx += look * lookPx / 5;
        if (mx < 20 || mx > 800) { mx = 410; }
        await s('Input.dispatchMouseEvent', { type: 'mouseMoved', x: mx, y: my });
        await sleep(20);
      }
      note(`t=${t.toFixed(1)} s: look ${look > 0 ? 'right' : 'left'} (page saw movementX sum ${await evalJs('window.saGame?.mouseX ?? 0')})`);
    }
    for (const k of [...held]) if (!want.includes(k)) {
      await key('keyUp', ...KEYS[k]);
      held.delete(k);
      note(`t=${t.toFixed(1)} s: ${k.toUpperCase()} up`);
    }
    for (const k of want) if (!held.has(k)) {
      await key('keyDown', ...KEYS[k]);
      held.add(k);
      note(`t=${t.toFixed(1)} s: ${k.toUpperCase()} down`);
    }
    if (t - walking >= walkTotal) {
      walked = true;
      note(`t=${t.toFixed(1)} s: walk done (${walkTotal} s)`);
      await shot(`walk-after-t${Math.round(t)}`);
    }
  }
  // frames read back by the worker itself (the compositor screenshot fails while the GPU process is busy)
  const snaps = JSON.parse((await evalJs('JSON.stringify(window.saGame?.snapshots?.splice(0) ?? [])')) ?? '[]');
  for (const sn of snaps) {
    const name = `frame-t${String(Math.round(t)).padStart(4, '0')}-state${sn.state}-it${sn.iterations}.png`;
    writeFileSync(`${out}/${name}`, Buffer.from(sn.png, 'base64'));
    note(`frame ${out}/${name}`);
  }
  if (t - lastShot >= every) {
    await evalJs('window.saGame?.snapshot?.()');
    lastShot = t;
    await shot(`t${String(Math.round(t)).padStart(4, '0')}-state${status.state}`);
  }
}
note(`final status: ${JSON.stringify(await evalJs('window.saGame?.status'))} progress ${JSON.stringify(await evalJs('window.saGame?.progress'))}`);
await shot('final');
ws.close();
kill();
process.exit(0);
