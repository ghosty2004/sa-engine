#!/usr/bin/env python3
"""A UDP echo server on 127.0.0.1 for the net-ping example mod (examples/mods/net-ping): every datagram goes back to
its sender unchanged.

    tools/mods/udp-echo.py [PORT]      (default 7790)

Prints one line per 100 datagrams. Natively the mod reaches it directly; in the browser through sa-udp-proxy
(`sa-udp-proxy 127.0.0.1:7790 --bind 127.0.0.1:7781`, the page's `?mods=net-ping&proxy=ws://127.0.0.1:7781`).
"""
import socket
import sys

port = int(sys.argv[1]) if len(sys.argv) > 1 else 7790
s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
s.bind(("127.0.0.1", port))
print(f"udp-echo: 127.0.0.1:{port}", flush=True)
n = 0
while True:
    data, peer = s.recvfrom(65535)
    s.sendto(data, peer)
    n += 1
    if n % 100 == 1:
        print(f"udp-echo: {n} datagrams, last from {peer[0]}:{peer[1]}", flush=True)
