#!/usr/bin/env python3
"""How much a peer pid costs on each of the two socket families.

On a unix socket the kernel answers, so the cost is a getsockopt. On TCP
loopback there is no such call and the pid has to be recovered out of band; this
times the `lsof` lookup the identity server uses. Both are timed against a
connection this script makes to itself, which is the same syscall path a Drone's
connection takes.
"""
import socket
import struct
import subprocess
import threading
import time

SOL_LOCAL, LOCAL_PEERPID = 0, 0x002
N = 20


def time_unix():
    path = "/tmp/armada-peer-cost.sock"
    import os
    if os.path.exists(path):
        os.unlink(path)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(path)
    srv.listen(1)
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    threading.Thread(target=lambda: client.connect(path), daemon=True).start()
    conn, _ = srv.accept()
    times = []
    for _ in range(N):
        t = time.perf_counter()
        pid = struct.unpack("=i", conn.getsockopt(SOL_LOCAL, LOCAL_PEERPID, 4))[0]
        times.append((time.perf_counter() - t) * 1e6)
    os.unlink(path)
    return pid, sorted(times)[len(times) // 2]


def time_tcp():
    srv = socket.socket()
    srv.bind(("127.0.0.1", 0))
    srv.listen(1)
    port = srv.getsockname()[1]
    client = socket.socket()
    threading.Thread(target=lambda: client.connect(("127.0.0.1", port)), daemon=True).start()
    conn, addr = srv.accept()
    times, pid = [], None
    for _ in range(N):
        t = time.perf_counter()
        out = subprocess.run(["lsof", "-nP", "-i", "TCP:%d" % addr[1]],
                             capture_output=True, text=True).stdout
        for line in out.splitlines()[1:]:
            if ("%d->" % addr[1]) in line:
                pid = int(line.split()[1])
                break
        times.append((time.perf_counter() - t) * 1e6)
    return pid, sorted(times)[len(times) // 2]


upid, ut = time_unix()
tpid, tt = time_tcp()
print("unix LOCAL_PEERPID  pid=%s  median %.0f us" % (upid, ut))
print("tcp  lsof lookup    pid=%s  median %.0f us (%.1f ms)" % (tpid, tt, tt / 1000))
print("ratio %.0fx" % (tt / ut))
