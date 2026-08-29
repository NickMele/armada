#!/usr/bin/env python3
"""One connection, from a process whose lifetime the runner chooses.

    012-race-client.py <server port> <source port|0> <hold ms> <tag>

`source port` of 0 lets the kernel pick, which is what a Drone's `curl` does;
any other value is bound explicitly with SO_REUSEADDR, which is how a collision
is arranged rather than waited for. `hold ms` is how long the process lives
after the connection is up — the quantity the whole lifetime experiment sweeps,
because a peer pid is only recoverable while the peer is still a process.

The pid goes in the body so the receiving side has a ground truth to score
against. In production that is precisely the claim a peer pid exists to
disbelieve; here the client is the spike's own and does not lie.
"""
import json
import os
import socket
import sys
import time

server_port, source_port, hold_ms, tag = (
    int(sys.argv[1]), int(sys.argv[2]), float(sys.argv[3]), sys.argv[4])

s = socket.socket()
if source_port:
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(("127.0.0.1", source_port))
s.connect(("127.0.0.1", server_port))
s.sendall((json.dumps({"pid": os.getpid(), "local": s.getsockname()[1],
                       "tag": tag}) + "\n").encode())
if hold_ms:
    time.sleep(hold_ms / 1000.0)
# No close, no interpreter teardown: the fastest exit available, so the window
# between the server's accept and its lookup is as exposed as it can be made.
os._exit(0)
