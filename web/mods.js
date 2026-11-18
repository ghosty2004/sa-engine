// The browser engine's glue for WebAssembly mods (MOD-2, docs/arch/ADR-004-mods.md; sa-game built with
// FEATURES=mods): the worker's `self.saGameMods`, called by crates/sa-mod-host/src/backend/web.rs. It knows no API
// function: the host passes the import names (sa-mod-api's IMPORTS order) and every import calls back into the
// host's `sa_mod_host_import(index, args)`. The host has already checked and fuel-instrumented the module
// (`__sa_fuel`: set before each call; a trap with the counter below 0 is a fuel overrun).
import * as sa from './pkg/sa_game.js';

const instances = [];

// The mods' storage (`storage = true`): every value of the IndexedDB database `sa-mod-storage` (object store `kv`,
// keys `MOD_ID/KEY`), copied here before the game runs (`loadStorage`); the host reads this copy synchronously and
// every change is written back in the background.
const stored = new Map();
let db = null;

function idb(mode, f) {
  if (!db) return;
  const tx = db.transaction('kv', mode);
  f(tx.objectStore('kv'));
  tx.onerror = () => console.warn(`mod storage: ${tx.error}`);
}

self.saGameMods = {
  async loadStorage() {
    if (typeof indexedDB === 'undefined') return console.warn('mod storage: no IndexedDB; kept for the session only');
    try {
      db = await new Promise((res, rej) => {
        const open = indexedDB.open('sa-mod-storage', 1);
        open.onupgradeneeded = () => open.result.createObjectStore('kv');
        open.onsuccess = () => res(open.result);
        open.onerror = () => rej(open.error);
      });
      await new Promise((res, rej) => {
        const req = db.transaction('kv').objectStore('kv').openCursor();
        req.onsuccess = () => {
          const c = req.result;
          if (!c) return res();
          stored.set(c.key, new Uint8Array(c.value));
          c.continue();
        };
        req.onerror = () => rej(req.error);
      });
      console.log(`mod storage: ${stored.size} values from IndexedDB`);
    } catch (e) {
      db = null;
      console.warn(`mod storage: IndexedDB unavailable (${e}); kept for the session only`);
    }
  },
  storageGet(id, key) {
    return stored.get(`${id}/${key}`);
  },
  storagePut(id, key, bytes) {
    const value = bytes.slice();
    stored.set(`${id}/${key}`, value);
    idb('readwrite', (kv) => kv.put(value, `${id}/${key}`));
  },
  storageDelete(id, key) {
    const had = stored.delete(`${id}/${key}`);
    if (had) idb('readwrite', (kv) => kv.delete(`${id}/${key}`));
    return had;
  },
  storageList(id) {
    const prefix = `${id}/`;
    let out = '';
    for (const [k, v] of stored) if (k.startsWith(prefix)) out += `${k.slice(prefix.length)}\t${v.length}\n`;
    return out;
  },
  // a mod file from the page's server (synchronous: the game thread waits), undefined when missing
  fetch(url) {
    const xhr = new XMLHttpRequest();
    xhr.open('GET', url, false);
    xhr.responseType = 'arraybuffer';
    xhr.send();
    return xhr.status === 200 ? new Uint8Array(xhr.response) : undefined;
  },
  instantiate(bytes, names) {
    const handle = instances.length;
    const imports = { sa: {} };
    names.split(',').forEach((name, index) => {
      imports.sa[name] = (...args) => {
        const r = sa.sa_mod_host_import(index, Float64Array.from(args));
        if (r === undefined) throw new Error(`host function ${name}: invalid call`);
        return r;
      };
    });
    // synchronous compile and instantiation (allowed in a worker); copies the bytes out of the game's memory
    const module = new WebAssembly.Module(bytes.slice());
    instances.push(new WebAssembly.Instance(module, imports));
    return handle;
  },
  call(handle, name, args, fuel) {
    const ex = instances[handle].exports;
    ex.__sa_fuel.value = fuel;
    try {
      return ex[name](...args);
    } catch (e) {
      if (ex.__sa_fuel.value < 0) throw 'fuel';
      throw String(e);
    }
  },
  read(handle, ptr, len) {
    const buf = instances[handle].exports.memory.buffer;
    if (ptr + len > buf.byteLength) return undefined;
    return new Uint8Array(buf, ptr, len).slice();
  },
  write(handle, ptr, bytes) {
    const buf = instances[handle].exports.memory.buffer;
    if (ptr + bytes.length > buf.byteLength) return false;
    new Uint8Array(buf, ptr, bytes.length).set(bytes);
    return true;
  },
};
