#!/usr/bin/env python3
"""Local development server for the web build (WEB-1), bound to 127.0.0.1 only.

Serves web/ (index.html, the worker, web/pkg from tools/web/build.sh) and, with --game DIR, the user's own GTA San
Andreas folder under /game/ (byte ranges, for the worker's synchronous reads) plus /game-index.json (path and size of
every file). The game files never leave this machine: the server refuses any non-loopback bind address.
"""

import argparse
import json
import os
import posixpath
import sys
import urllib.parse
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
WEB = os.path.join(ROOT, "web")


def game_index(game):
    out = []
    for d, _, files in os.walk(game):
        for f in files:
            p = os.path.join(d, f)
            rel = os.path.relpath(p, game).replace(os.sep, "/")
            out.append({"path": rel, "size": os.path.getsize(p)})
    out.sort(key=lambda e: e["path"])
    return out


class Handler(SimpleHTTPRequestHandler):
    game = None
    index = b"[]"
    samp_auth = None

    def __init__(self, *a, **kw):
        super().__init__(*a, directory=WEB, **kw)

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
        super().do_GET()

    def send_game(self, rel):
        if Handler.game is None:
            self.send_error(404, "no --game folder")
            return
        rel = posixpath.normpath(rel)
        if rel.startswith("..") or rel.startswith("/"):
            self.send_error(403)
            return
        path = os.path.join(Handler.game, *rel.split("/"))
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
        self.send_response(206 if partial else 200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(n))
        if partial:
            self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
        self.end_headers()
        with open(path, "rb") as f:
            f.seek(start)
            while n > 0:
                chunk = f.read(min(n, 1 << 20))
                if not chunk:
                    break
                self.wfile.write(chunk)
                n -= len(chunk)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, default=8642)
    ap.add_argument("--bind", default="127.0.0.1")
    ap.add_argument("--game", help="your GTA San Andreas folder (served to this machine only)")
    ap.add_argument("--samp-auth", help="the SA:MP auth table (third-party data, served to this machine only as /samp-auth.txt)")
    a = ap.parse_args()
    if a.bind not in ("127.0.0.1", "::1", "localhost"):
        sys.exit("serve.py: binds to the loopback address only (the game files must not leave this machine)")
    if a.game:
        if not os.path.isfile(os.path.join(a.game, "gta_sa.exe")):
            sys.exit(f"serve.py: {a.game} has no gta_sa.exe")
        Handler.game = os.path.abspath(a.game)
        Handler.index = json.dumps(game_index(Handler.game)).encode()
    if a.samp_auth:
        if not os.path.isfile(a.samp_auth):
            sys.exit(f"serve.py: {a.samp_auth} does not exist")
        Handler.samp_auth = os.path.abspath(a.samp_auth)
    if not os.path.isfile(os.path.join(WEB, "pkg", "sa_game.js")):
        print("serve.py: web/pkg is missing: run tools/web/build.sh first", file=sys.stderr)
    srv = ThreadingHTTPServer((a.bind, a.port), Handler)
    where = f" with the game folder {Handler.game}" if Handler.game else " (choose the game folder in the page)"
    print(f"sa-game web: http://localhost:{a.port}/{where}", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
