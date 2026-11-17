#!/usr/bin/env python3
"""Local development server for the web build (WEB-1), bound to 127.0.0.1 only.

Serves web/ (index.html, the worker, web/pkg from tools/web/build.sh) and, with --game DIR, the user's own GTA San
Andreas folder under /game/ (byte ranges, for the worker's synchronous reads) plus /game-index.json (path and size of
every file). The game files never leave this machine: the server refuses any non-loopback bind address.

With --mods DIR (MOD-2), the mod folders in DIR (each with mod.toml and its .wasm, e.g. from examples/mods/build.sh)
are served under /mods/ for the page's ?mods=NAME,... (sa-game built with FEATURES=mods).

With --samp-dir DIR (the files of a SA:MP client install: samp.saa, sampgui.png, mouse.png, ...), those files are
overlaid on the game folder: listed in /game-index.json and served under /game/ like the game's own files (a file
of the game folder wins; DLLs are left out), so the page's file layer finds them where the native build looks for
them (the game folder).

Every file of /game-index.json carries its mtime: the page's persistent cache (RENDER-2 R2a, web/loader.js) keys its
blocks by path, size and mtime, so a changed file is fetched again.

Headers: COOP/COEP (cross-origin isolation: the audio ring's SharedArrayBuffer, 5 us timers). A reverse proxy in
front of this server (another device reaching the page) must pass the response headers through unchanged (Caddy's
reverse_proxy and nginx's proxy_pass do) and must not buffer or re-chunk ranged responses away: the loader sends
`Range` requests and expects `206` with `Content-Range`. Over plain http from another host the page is not a
secure context: no cross-origin isolation, no OPFS; the loader then keeps its cache in IndexedDB.

--throttle-rtt MS / --throttle-mbit N: simulate a slow link for /game/ (each request waits MS before its answer;
all answers share N Mbit/s), for measurements (docs/perf/measure.md).
--web DIR: serve another build of the page (e.g. a saved copy of web/ for before/after measurements).
"""

import argparse
import json
import os
import posixpath
import sys
import threading
import time
import urllib.parse
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
WEB = os.path.join(ROOT, "web")


def game_index(game, skip_dll=False):
    out = []
    for d, _, files in os.walk(game):
        for f in files:
            if skip_dll and f.lower().endswith(".dll"):
                continue
            p = os.path.join(d, f)
            rel = os.path.relpath(p, game).replace(os.sep, "/")
            st = os.stat(p)
            out.append({"path": rel, "size": st.st_size, "mtime": int(st.st_mtime)})
    out.sort(key=lambda e: e["path"])
    return out


def merged_index(game, samp):
    """The game folder's index plus the SA:MP files it does not have (case-insensitive paths)."""
    idx = game_index(game) if game else []
    have = {e["path"].lower() for e in idx}
    extra = [e for e in game_index(samp, skip_dll=True) if e["path"].lower() not in have]
    Handler.samp_files = {e["path"] for e in extra}
    out = idx + extra
    out.sort(key=lambda e: e["path"])
    return out


class Link:
    """The simulated link of --throttle-*: a delay per request and one shared bandwidth."""

    rtt = 0.0
    rate = 0.0  # bytes per second, 0 = unlimited
    lock = threading.Lock()
    free_at = 0.0

    @classmethod
    def send(cls, n):
        """Waits until `n` more bytes fit through the shared link."""
        if cls.rate <= 0:
            return
        with cls.lock:
            now = time.monotonic()
            start = max(now, cls.free_at)
            cls.free_at = start + n / cls.rate
            done = cls.free_at
        time.sleep(max(0.0, done - time.monotonic()))


class Handler(SimpleHTTPRequestHandler):
    web = WEB
    game = None
    index = b"[]"
    samp_auth = None
    samp_dir = None
    samp_files = set()
    mods = None

    def __init__(self, *a, **kw):
        super().__init__(*a, directory=Handler.web, **kw)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        # cross-origin isolation (needed only for a future threaded build; harmless now)
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()

    def guess_type(self, path):
        if path.endswith(".wasm"):
            return "application/wasm"
        if path.endswith(".js") or path.endswith(".mjs"):
            return "text/javascript"
        return super().guess_type(path)

    def log_message(self, fmt, *args):
        if os.environ.get("SA_WEB_VERBOSE"):
            super().log_message(fmt, *args)

    def do_GET(self):
        url = urllib.parse.urlparse(self.path)
        if url.path == "/game-index.json":
            body = Handler.index
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        if url.path == "/samp-auth.txt":
            if Handler.samp_auth is None:
                self.send_error(404, "no --samp-auth file")
                return
            with open(Handler.samp_auth, "rb") as f:
                body = f.read()
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        if url.path.startswith("/game/"):
            self.send_game(urllib.parse.unquote(url.path[len("/game/"):]))
            return
        if url.path.startswith("/mods/"):
            self.send_mod(urllib.parse.unquote(url.path[len("/mods/"):]))
            return
        super().do_GET()

    def send_mod(self, rel):
        rel = posixpath.normpath(rel)
        if Handler.mods is None or rel.startswith("..") or rel.startswith("/"):
            self.send_error(404, "no --mods folder")
            return
        path = os.path.join(Handler.mods, *rel.split("/"))
        if not os.path.isfile(path):
            self.send_error(404)
            return
        with open(path, "rb") as f:
            body = f.read()
        self.send_response(200)
        self.send_header("Content-Type", self.guess_type(path))
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def send_game(self, rel):
        if Handler.game is None:
            self.send_error(404, "no --game folder")
            return
        rel = posixpath.normpath(rel)
        if rel.startswith("..") or rel.startswith("/"):
            self.send_error(403)
            return
        base = Handler.samp_dir if rel in Handler.samp_files else Handler.game
        path = os.path.join(base, *rel.split("/"))
        if not os.path.isfile(path):
            self.send_error(404)
            return
        size = os.path.getsize(path)
        start, end = 0, size - 1
        rng = self.headers.get("Range")
        partial = False
        if rng and rng.startswith("bytes="):
            a, _, b = rng[6:].partition("-")
            start = int(a) if a else 0
            end = min(int(b), size - 1) if b else size - 1
            partial = True
        if start > end:
            self.send_response(416)
            self.send_header("Content-Range", f"bytes */{size}")
            self.end_headers()
            return
        n = end - start + 1
        if Link.rtt > 0:
            time.sleep(Link.rtt)
        self.send_response(206 if partial else 200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(n))
        if partial:
            self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
        self.end_headers()
        with open(path, "rb") as f:
            f.seek(start)
            while n > 0:
                chunk = f.read(min(n, 64 << 10 if Link.rate > 0 else 1 << 20))
                if not chunk:
                    break
                Link.send(len(chunk))
                self.wfile.write(chunk)
                n -= len(chunk)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, default=8642)
    ap.add_argument("--bind", default="127.0.0.1")
    ap.add_argument("--game", help="your GTA San Andreas folder (served to this machine only)")
    ap.add_argument("--samp-dir", help="a SA:MP client folder (samp.saa, sampgui.png, mouse.png) overlaid on the game folder")
    ap.add_argument("--samp-auth", help="the SA:MP auth table (third-party data, served to this machine only as /samp-auth.txt)")
    ap.add_argument("--mods", help="a folder of mod folders (MOD-2), served under /mods/ for the page's ?mods=NAME,...")
    ap.add_argument("--web", help="serve this directory instead of web/ (a saved build of the page)")
    ap.add_argument("--throttle-rtt", type=float, default=0, help="milliseconds each /game/ request waits (slow link)")
    ap.add_argument("--throttle-mbit", type=float, default=0, help="shared bandwidth of /game/ answers in Mbit/s")
    a = ap.parse_args()
    if a.web:
        Handler.web = os.path.abspath(a.web)
    Link.rtt = a.throttle_rtt / 1000
    Link.rate = a.throttle_mbit * 1e6 / 8
    if a.bind not in ("127.0.0.1", "::1", "localhost"):
        sys.exit("serve.py: binds to the loopback address only (the game files must not leave this machine)")
    if a.game:
        if not os.path.isfile(os.path.join(a.game, "gta_sa.exe")):
            sys.exit(f"serve.py: {a.game} has no gta_sa.exe")
        Handler.game = os.path.abspath(a.game)
        Handler.index = json.dumps(game_index(Handler.game)).encode()
    if a.samp_dir:
        if not a.game:
            sys.exit("serve.py: --samp-dir needs --game")
        if not os.path.isfile(os.path.join(a.samp_dir, "samp.saa")):
            sys.exit(f"serve.py: {a.samp_dir} has no samp.saa")
        Handler.samp_dir = os.path.abspath(a.samp_dir)
        Handler.index = json.dumps(merged_index(Handler.game, Handler.samp_dir)).encode()
    if a.samp_auth:
        if not os.path.isfile(a.samp_auth):
            sys.exit(f"serve.py: {a.samp_auth} does not exist")
        Handler.samp_auth = os.path.abspath(a.samp_auth)
    if a.mods:
        if not os.path.isdir(a.mods):
            sys.exit(f"serve.py: --mods {a.mods} is not a folder")
        Handler.mods = os.path.abspath(a.mods)
    if not os.path.isfile(os.path.join(Handler.web, "pkg", "sa_game.js")):
        print("serve.py: web/pkg is missing: run tools/web/build.sh first", file=sys.stderr)
    srv = ThreadingHTTPServer((a.bind, a.port), Handler)
    where = f" with the game folder {Handler.game}" if Handler.game else " (choose the game folder in the page)"
    if Link.rtt or Link.rate:
        where += f" (simulated link: {a.throttle_rtt:g} ms per request, {a.throttle_mbit:g} Mbit/s)"
    print(f"sa-game web: http://localhost:{a.port}/{where}", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
