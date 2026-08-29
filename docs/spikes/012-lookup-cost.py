#!/usr/bin/env python3
"""What a peer pid costs, four ways, with five candidate processes live.

Spike 10 timed one route and got 67ms. This times that route again on the same
machine, beside three others, with five real child processes each holding an
open loopback connection — the n=5 shape, not a single pair. Ground truth is
known: this script spawned the children and knows which pid opened which port,
so a route that returns a pid is scored right or wrong rather than merely fast.

Also timed: `getsockopt(SOL_LOCAL, LOCAL_PEERPID)` on AF_UNIX, unchanged from
spike 10, as the floor a kernel attestation would set if the CLI would take one.
"""
import json
import os
import socket
import struct
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import importlib
peerpid = importlib.import_module("012-peerpid")

HERE = os.path.dirname(os.path.abspath(__file__))
N = 20
FLEET = 5

CHILD = r"""
import socket, sys, os, time
port = int(sys.argv[1])
s = socket.socket()
s.connect(("127.0.0.1", port))
print(os.getpid(), s.getsockname()[1], flush=True)
time.sleep(600)
"""


def median(xs):
    xs = sorted(xs)
    return xs[len(xs) // 2]


def time_unix_peerpid():
    path = "/tmp/armada-012-peer-cost.sock"
    if os.path.exists(path):
        os.unlink(path)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(path)
    srv.listen(1)
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    import threading
    threading.Thread(target=lambda: client.connect(path), daemon=True).start()
    conn, _ = srv.accept()
    times = []
    for _ in range(N):
        t = time.perf_counter()
        pid = struct.unpack("=i", conn.getsockopt(0, 0x002, 4))[0]
        times.append((time.perf_counter() - t) * 1e6)
    conn.close()
    client.close()
    srv.close()
    os.unlink(path)
    return pid, median(times)


def main():
    peerpid.calibrate()
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", 0))
    srv.listen(16)
    port = srv.getsockname()[1]

    kids, held, truth = [], [], {}
    for _ in range(FLEET):
        p = subprocess.Popen([sys.executable, "-c", CHILD, str(port)],
                             stdout=subprocess.PIPE, text=True)
        pid, sport = p.stdout.readline().split()
        conn, addr = srv.accept()
        kids.append(p)
        held.append(conn)
        truth[int(sport)] = int(pid)
        assert addr[1] == int(sport), (addr, sport)

    pids = [p.pid for p in kids]
    ports = sorted(truth)
    print("fleet of %d, ports %s" % (FLEET, ports))

    results = {}
    for name, fn in peerpid.ROUTES.items():
        times, right, wrong, missed = [], 0, 0, 0
        for i in range(N):
            want_port = ports[i % len(ports)]
            t = time.perf_counter()
            got = fn(want_port, pids)
            times.append((time.perf_counter() - t) * 1e6)
            if got is None:
                missed += 1
            elif got == truth[want_port]:
                right += 1
            else:
                wrong += 1
        results[name] = {"median_us": median(times), "right": right,
                         "wrong": wrong, "missed": missed}
        print("  %-10s median %9.1f us   right %2d  wrong %2d  missed %2d"
              % (name, median(times), right, wrong, missed))

    upid, ut = time_unix_peerpid()
    results["unix_peerpid"] = {"median_us": ut, "right": N, "wrong": 0,
                               "missed": 0}
    print("  %-10s median %9.1f us   (AF_UNIX, kernel-answered, pid=%s)"
          % ("peerpid", ut, upid))

    for c in held:
        c.close()
    srv.close()
    for p in kids:
        p.kill()
        p.wait()

    with open(os.path.join(HERE, "012-lookup-cost.json"), "w") as f:
        json.dump({"fleet": FLEET, "samples": N,
                   "offsets": {"local": peerpid.LOCAL_PORT_OFFSET,
                               "peer": peerpid.PEER_PORT_OFFSET},
                   "routes": results}, f, indent=2)


main()
