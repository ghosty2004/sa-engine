// The game folder's loader (RENDER-2 R2a): fetches the served game folder in 256 KiB blocks with ranged requests
// (several in flight, never blocking anyone), keeps every block it fetched in the browser's persistent storage (OPFS,
// else IndexedDB; keyed by path, size and mtime from game-index.json, so a changed file is fetched again) and hands
// blocks to the game's worker over a MessagePort. The game's worker never waits for it: it asks for the blocks a
// streaming read needs and runs that read once they arrived (crates/sa-game/src/bind.rs `coop`).
//
// Page -> loader: {type:'init', files:[{path,size,mtime,url}], port}, {type:'downloadAll', on}
// Loader -> page: {type:'cache', ...}: the store, cached / total bytes, network bytes, requests in flight
// Game worker -> loader (port): {type:'get', id, blocks, prio}, {type:'put', id, block, data}, {type:'manifest', list}
// Loader -> game worker (port): {type:'hello', store, startup:[[id, block]...], manifestKnown}, {type:'block', id,
// block, data}, {type:'failed', id, block, error}
const BLOCK = 256 * 1024;
const MAX_FLIGHT = 6; // the browser's per-host connection limit over HTTP/1.1
const MERGE = [4, 4, 8, 8]; // blocks one request may cover, per priority
// priorities: 0 the game waits for these (a streaming read), 1 read-ahead after a synchronous read, 2 start-up
// prefetch, 3 the background "download everything"
const P_URGENT = 0, P_AHEAD = 1, P_STARTUP = 2, P_ALL = 3;

let files = [];
let port = null;
let sync = null; // shared memory for the game worker's synchronous reads (see onSync)
let store = null;
const queues = [[], [], []]; // P_URGENT..P_STARTUP: [id, block]
const state = new Map(); // `${id}:${block}` -> { flight: bool, deliver: bool, prio }
let flight = 0;
let netBytes = 0;
let downloadAll = false;
let allCursor = { id: 0, block: 0 };

// ---------------------------------------------------------------- keys ---
function fnv(s, seed) {
  let h = seed >>> 0;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(16).padStart(8, '0');
}
const fileKey = (f) => {
  const s = `${f.path.toLowerCase()}|${f.size}|${f.mtime ?? 0}`;
  return fnv(s, 0x811c9dc5) + fnv(s, 0x01234567);
};

// ---------------------------------------------------------------- stores ---
// A store: async init(files) (fills f.have), read(f, b) -> ArrayBuffer | null, write(f, b, buf), getMeta(name),
// putMeta(name, text). Writes are ordered: a block's bitmap byte is written after its data.
async function opfsStore() {
  if (!navigator.storage?.getDirectory) return null;
  const root = await navigator.storage.getDirectory();
  const dir = await root.getDirectoryHandle('sa-game-cache', { create: true });
  // a probe: sync access handles exist in dedicated workers of secure contexts only
  const probe = await dir.getFileHandle('probe', { create: true });
  if (!probe.createSyncAccessHandle) return null;
  (await probe.createSyncAccessHandle()).close();
  const handles = new Map(); // key -> Promise<{data, map}>
  const open = (f) => {
    let p = handles.get(f.key);
    if (!p) {
      p = (async () => {
        const data = await (await dir.getFileHandle(`${f.key}.bin`, { create: true })).createSyncAccessHandle();
        const map = await (await dir.getFileHandle(`${f.key}.map`, { create: true })).createSyncAccessHandle();
        return { data, map };
      })();
      handles.set(f.key, p);
    }
    return p;
  };
  return {
    kind: 'OPFS',
    async init(list) {
      const keys = new Map(list.map((f) => [f.key, f]));
      for await (const [name, h] of dir.entries()) {
        if (h.kind !== 'file' || name === 'probe' || name.startsWith('meta-')) continue;
        const key = name.replace(/\.(bin|map)$/, '');
        const f = keys.get(key);
        if (!f) {
          await dir.removeEntry(name).catch(() => {});
          continue;
        }
        if (name.endsWith('.map')) {
          const bytes = new Uint8Array(await (await h.getFile()).arrayBuffer());
          f.have.set(bytes.subarray(0, f.have.length));
        }
      }
      // stale index files (another game-index.json): only the current one is kept
      const metaName = `meta-${fnv(list.map((f) => f.key).join(','), 7)}`;
      this.metaName = metaName;
      for await (const [name] of dir.entries()) {
        if (name.startsWith('meta-') && !name.startsWith(metaName)) await dir.removeEntry(name).catch(() => {});
      }
    },
    async read(f, b) {
      const { data } = await open(f);
      const n = Math.min(BLOCK, f.size - b * BLOCK);
      const buf = new Uint8Array(n);
      if (data.read(buf, { at: b * BLOCK }) !== n) return null;
      return buf.buffer;
    },
    async write(f, b, buf) {
      const { data, map } = await open(f);
      data.write(new Uint8Array(buf), { at: b * BLOCK });
      data.flush();
      map.write(new Uint8Array([1]), { at: b });
      map.flush();
    },
    async getMeta(name) {
      try {
        return await (await (await dir.getFileHandle(`${this.metaName}-${name}`)).getFile()).text();
      } catch {
        return null;
      }
    },
    async putMeta(name, text) {
      const h = await (await dir.getFileHandle(`${this.metaName}-${name}`, { create: true })).createSyncAccessHandle();
      const bytes = new TextEncoder().encode(text);
      h.truncate(0);
      h.write(bytes, { at: 0 });
      h.flush();
      h.close();
    },
  };
}

function idbReq(r) {
  return new Promise((res, rej) => {
    r.onsuccess = () => res(r.result);
    r.onerror = () => rej(r.error);
  });
}

async function idbStore() {
  if (typeof indexedDB === 'undefined') return null;
  const open = indexedDB.open('sa-game-cache', 1);
  open.onupgradeneeded = () => {
    open.result.createObjectStore('blocks');
    open.result.createObjectStore('meta');
  };
  const db = await idbReq(open);
  const tx = (name, mode) => db.transaction(name, mode).objectStore(name);
  return {
    kind: 'IndexedDB',
    async init(list) {
      const keys = new Map(list.map((f) => [f.key, f]));
      const all = await idbReq(tx('blocks', 'readonly').getAllKeys());
      const stale = [];
      for (const k of all) {
        const [key, b] = String(k).split(':');
        const f = keys.get(key);
        if (f) f.have[Number(b)] = 1;
        else stale.push(k);
      }
      if (stale.length) {
        const s = tx('blocks', 'readwrite');
        for (const k of stale) s.delete(k);
      }
      this.metaName = fnv(list.map((f) => f.key).join(','), 7);
    },
    async read(f, b) {
      const v = await idbReq(tx('blocks', 'readonly').get(`${f.key}:${b}`));
      return v ?? null;
    },
    async write(f, b, buf) {
      await idbReq(tx('blocks', 'readwrite').put(buf, `${f.key}:${b}`));
    },
    async getMeta(name) {
      return (await idbReq(tx('meta', 'readonly').get(`${this.metaName}-${name}`))) ?? null;
    },
    async putMeta(name, text) {
      await idbReq(tx('meta', 'readwrite').put(text, `${this.metaName}-${name}`));
    },
  };
}

// ---------------------------------------------------------------- fetching ---
const key = (id, b) => `${id}:${b}`;

// wants block b of file id at priority prio; deliver: hand it to the game's worker once there
function want(id, b, prio, deliver) {
  const f = files[id];
  if (!f || b < 0 || b >= f.nblocks) return;
  const k = key(id, b);
  const s = state.get(k);
  if (s) {
    s.deliver ||= deliver;
    if (prio < s.prio && !s.flight) {
      s.prio = prio;
      queues[prio].push([id, b]);
    }
    return;
  }
  if (f.have[b]) {
    if (!deliver) return;
    state.set(k, { flight: true, deliver, prio, cached: true });
    store.read(f, b).then((buf) => {
      state.delete(k);
      if (buf) send(id, b, buf);
      else {
        f.have[b] = 0;
        want(id, b, prio, deliver);
      }
    }, () => {
      state.delete(k);
      f.have[b] = 0;
      want(id, b, prio, deliver);
    });
    return;
  }
  state.set(k, { flight: false, deliver, prio });
  queues[prio].push([id, b]);
  pump();
}

function send(id, b, buf) {
  port.postMessage({ type: 'block', id, block: b, data: buf }, [buf]);
}

// the next request: the most urgent queued block and the queued blocks right after it (same file, same queue)
function next() {
  for (let p = 0; p < queues.length; p++) {
    const q = queues[p];
    while (q.length) {
      const [id, b] = q.shift();
      const s = state.get(key(id, b));
      if (!s || s.flight || s.prio !== p) continue;
      const run = [b];
      while (run.length < MERGE[p]) {
        const nb = b + run.length;
        const i = q.findIndex(([qi, qb]) => qi === id && qb === nb);
        const ns = state.get(key(id, nb));
        if (i < 0 || !ns || ns.flight) break;
        q.splice(i, 1);
        run.push(nb);
      }
      return { id, run };
    }
  }
  if (!downloadAll) return null;
  // download everything: the next run of blocks no store holds, in file order
  while (allCursor.id < files.length) {
    const f = files[allCursor.id];
    const run = [];
    while (allCursor.block < f.nblocks && run.length < MERGE[P_ALL]) {
      const b = allCursor.block;
      if (f.have[b] || state.has(key(allCursor.id, b))) {
        if (run.length) break;
        allCursor.block++;
        continue;
      }
      state.set(key(allCursor.id, b), { flight: false, deliver: false, prio: P_ALL });
      run.push(b);
      allCursor.block++;
    }
    if (run.length) return { id: allCursor.id, run };
    allCursor = { id: allCursor.id + 1, block: 0 };
  }
  downloadAll = false;
  return null;
}

function pump() {
  while (flight < MAX_FLIGHT) {
    const r = next();
    if (!r) break;
    flight++;
    for (const b of r.run) state.get(key(r.id, b)).flight = true;
    fetchRun(r.id, r.run).finally(() => {
      flight--;
      pump();
    });
  }
  report();
}

async function fetchRun(id, run, tries = 0) {
  const f = files[id];
  const start = run[0] * BLOCK;
  const end = Math.min(f.size, (run[run.length - 1] + 1) * BLOCK);
  try {
    const r = await fetch(f.url, { headers: { Range: `bytes=${start}-${end - 1}` }, cache: 'no-store' });
    if (r.status !== 206 && !(r.status === 200 && start === 0 && end === f.size)) throw new Error(`HTTP ${r.status}`);
    const buf = await r.arrayBuffer();
    if (buf.byteLength !== end - start) throw new Error(`short answer (${buf.byteLength} of ${end - start} bytes)`);
    netBytes += buf.byteLength;
    for (const b of run) {
      const from = b * BLOCK - start;
      const part = buf.slice(from, Math.min(buf.byteLength, from + BLOCK));
      await arrived(id, b, part);
    }
  } catch (e) {
    if (tries < 3) {
      await new Promise((res) => setTimeout(res, 200 * (tries + 1)));
      return fetchRun(id, run, tries + 1);
    }
    for (const b of run) {
      const s = state.get(key(id, b));
      state.delete(key(id, b));
      if (s?.deliver) port.postMessage({ type: 'failed', id, block: b, error: String(e) });
    }
    console.warn(`loader: ${f.path} ${start}-${end}: ${e}`);
  }
}

// a block arrived (fetched, or from the game's worker after a synchronous read): persisted, then delivered
async function arrived(id, b, buf, fromGame = false) {
  const f = files[id];
  const k = key(id, b);
  const s = state.get(k);
  if (!f.have[b]) {
    try {
      await store?.write(f, b, buf);
      if (store) f.have[b] = 1;
    } catch (e) {
      // quota or a closed handle: keep going without persisting this block
      if (!storeError) console.warn(`loader: ${store.kind} write failed: ${e}`);
      storeError = String(e);
    }
  }
  if (s && !(fromGame && s.flight)) state.delete(k);
  if (s?.deliver && !fromGame) send(id, b, buf);
}
let storeError = null;

// ---------------------------------------------------------------- start-up list ---
// The blocks the game reads before it is in the world: the list the game's worker recorded on an earlier load
// (meta "startup"), else gta_sa.exe, every file under data/ and text/ and the directories of the IMG archives.
async function startupList() {
  const known = await store?.getMeta('startup').catch(() => null);
  if (known) {
    const byPath = new Map(files.map((f, i) => [f.path.toLowerCase(), i]));
    const out = [];
    for (const [path, b] of JSON.parse(known)) {
      const id = byPath.get(path);
      if (id !== undefined) out.push([id, b]);
    }
    return { list: out, known: true };
  }
  const out = [];
  const whole = (id) => {
    for (let b = 0; b < files[id].nblocks; b++) out.push([id, b]);
  };
  for (let id = 0; id < files.length; id++) {
    const p = files[id].path.toLowerCase();
    if (p === 'gta_sa.exe' || ((p.startsWith('data/') || p.startsWith('text/')) && files[id].size <= 4 << 20)) whole(id);
  }
  // IMG directories (VER2: "VER2", entry count, 32-byte entries): the header block first, then the rest
  await Promise.all(files.map(async (f, id) => {
    if (!f.path.toLowerCase().endsWith('.img') || f.size < 8) return;
    try {
      const r = await fetch(f.url, { headers: { Range: 'bytes=0-7' }, cache: 'no-store' });
      const h = new DataView(await r.arrayBuffer());
      const dirEnd = h.getUint32(0, true) === 0x32524556 ? 8 + 32 * h.getUint32(4, true) : 0;
      for (let b = 0; b * BLOCK < Math.min(dirEnd, f.size); b++) out.push([id, b]);
    } catch {}
  }));
  return { list: out, known: false };
}

// ---------------------------------------------------------------- page status ---
let lastReport = 0;
let reportTimer = null;
function report(force) {
  const now = performance.now();
  if (!force && now - lastReport < 250) {
    reportTimer ??= setTimeout(() => {
      reportTimer = null;
      report(true);
    }, 260);
    return;
  }
  lastReport = now;
  let cached = 0;
  let total = 0;
  for (const f of files) {
    total += f.size;
    let n = 0;
    for (let b = 0; b < f.nblocks; b++) n += f.have[b];
    cached += Math.min(f.size, n * BLOCK);
  }
  self.postMessage({ type: 'cache', store: store?.kind ?? 'none', storeError, cached, total, net: netBytes, flight,
    queued: queues.reduce((a, q) => a + q.length, 0), downloadAll });
}

// ---------------------------------------------------------------- messages ---
self.onmessage = async (ev) => {
  const m = ev.data;
  if (m.type === 'init') {
    files = m.files.map((f) => ({ ...f, key: fileKey(f), nblocks: Math.ceil(f.size / BLOCK), have: new Uint8Array(Math.ceil(f.size / BLOCK)) }));
    port = m.port;
    if (m.sab) sync = { hdr: new Int32Array(m.sab, 0, 4), data: new Uint8Array(m.sab, 16) };
    try {
      store = (await opfsStore().catch(() => null)) ?? (await idbStore().catch(() => null));
      await store?.init(files);
    } catch (e) {
      console.warn(`loader: no persistent cache (${e})`);
      store = null;
    }
    const { list, known } = await startupList();
    port.onmessage = onGame;
    port.postMessage({ type: 'hello', store: store?.kind ?? 'none', startup: list, manifestKnown: known });
    report(true);
  } else if (m.type === 'downloadAll') {
    downloadAll = !!m.on;
    allCursor = { id: 0, block: 0 };
    pump();
  }
};

// a synchronous read of the game worker (it waits on sync.hdr[0]): blocks [first, last] of file id from the store,
// the missing ones fetched at once, copied into the shared memory
async function onSync(m) {
  const f = files[m.id];
  let n = 0;
  let err = 0;
  try {
    const parts = await Promise.all(range(m.first, m.last).map(async (b) => {
      const buf = f.have[b] ? await store.read(f, b).catch(() => null) : null;
      return buf ? { b, buf, cached: true } : null;
    }));
    const missing = range(m.first, m.last).filter((b) => !parts[b - m.first]);
    if (missing.length) {
      const start = missing[0] * BLOCK;
      const end = Math.min(f.size, (missing[missing.length - 1] + 1) * BLOCK);
      const r = await fetch(f.url, { headers: { Range: `bytes=${start}-${end - 1}` }, cache: 'no-store' });
      if (r.status !== 206 && !(r.status === 200 && start === 0 && end === f.size)) throw new Error(`HTTP ${r.status}`);
      const buf = await r.arrayBuffer();
      netBytes += buf.byteLength;
      for (const b of missing) {
        const from = (b - missing[0]) * BLOCK;
        if (from >= buf.byteLength) throw new Error('short answer');
        parts[b - m.first] = { b, buf: buf.slice(from, Math.min(buf.byteLength, from + BLOCK)), cached: false };
      }
    }
    for (const p of parts) {
      const bytes = new Uint8Array(p.buf);
      sync.data.set(bytes, n);
      n += bytes.length;
    }
    // answered first, kept after (the game worker is waiting)
    Atomics.store(sync.hdr, 1, n);
    Atomics.store(sync.hdr, 2, 0);
    Atomics.store(sync.hdr, 0, 1);
    Atomics.notify(sync.hdr, 0);
    for (const p of parts) if (!p.cached) await arrived(m.id, p.b, p.buf, true);
    report();
    return;
  } catch (e) {
    console.warn(`loader: synchronous read of ${f.path} blocks ${m.first}-${m.last}: ${e}`);
    err = 1;
  }
  Atomics.store(sync.hdr, 1, n);
  Atomics.store(sync.hdr, 2, err);
  Atomics.store(sync.hdr, 0, 1);
  Atomics.notify(sync.hdr, 0);
}
const range = (a, b) => Array.from({ length: b - a + 1 }, (_, i) => a + i);

function onGame(ev) {
  const m = ev.data;
  if (m.type === 'sync') {
    onSync(m);
  } else if (m.type === 'get') {
    for (const b of m.blocks) want(m.id, b, m.prio ?? P_URGENT, true);
    pump();
  } else if (m.type === 'put') {
    arrived(m.id, m.block, m.data, true).then(() => report());
  } else if (m.type === 'manifest') {
    const list = m.list.map(([id, b]) => [files[id].path.toLowerCase(), b]);
    store?.putMeta('startup', JSON.stringify(list)).catch((e) => console.warn(`loader: start-up list not kept: ${e}`));
  }
}
