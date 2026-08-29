#!/usr/bin/env python3
"""Can a peer-pid lookup be made to name the wrong process?

Spike 10 recovered a pid on TCP loopback with `lsof`, 67ms after accept. That is
not a kernel attestation at accept; it is a question asked afterwards about
state that has since moved on. Three ways that could go wrong are tried here,
each with a ground truth the runner spawned and therefore knows.

  A  reuse       a second process takes the same local port and connects to the
                 same listener while the server still holds the first connection
  B  ambiguity   two live processes hold the same local port number against
                 different destinations, and the lookup is keyed on the number
  C  lifetime    the peer exits before the lookup finishes

Each is scored right / none / WRONG against the pid the client wrote in its
body. `none` and `wrong` are different failures and are never added together:
one is a Drone that cannot be identified, the other is a Drone identified as
another Drone.
"""
import collections
import importlib
import json
import os
import socket
import subprocess
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
peerpid = importlib.import_module("012-peerpid")

HERE = os.path.dirname(os.path.abspath(__file__))
CLIENT = os.path.join(HERE, "012-race-client.py")
# Outside the ephemeral range (49152-65535 on this machine), so the only thing
# reusing these is this script.
POOL = [24101, 24102, 24103, 24104]
HOLDS_MS = [int(x) for x in
            os.environ.get("HOLDS", "0,1,5,20,50,100,200,500").split(",")]
LIFETIME_N = int(os.environ.get("LIFETIME_N", "24"))


class Listener:
    """A server that records, per connection, what each route says about its
    peer — and holds the connection open so nothing is scored against a socket
    the kernel has already reclaimed."""

    def __init__(self, routes):
        self.routes = routes
        self.sock = socket.socket()
        self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.sock.bind(("127.0.0.1", 0))
        self.sock.listen(64)
        self.port = self.sock.getsockname()[1]
        self.known = []
        self.seen = []
        self.held = []
        self.lock = threading.Lock()
        self.stop = threading.Event()
        threading.Thread(target=self._accept, daemon=True).start()

    def spawn(self, source_port, hold_ms, tag, wait=True):
        p = subprocess.Popen(
            [sys.executable, CLIENT, str(self.port), str(source_port),
             str(hold_ms), tag],
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        with self.lock:
            self.known.append(p.pid)
        if wait:
            _, err = p.communicate()
            return p.pid, p.returncode, err.decode().strip()
        return p.pid, None, None

    def _accept(self):
        while not self.stop.is_set():
            try:
                conn, addr = self.sock.accept()
            except OSError:
                return
            self.held.append(conn)
            threading.Thread(target=self._handle, args=(conn, addr),
                             daemon=True).start()

    def _handle(self, conn, addr):
        record = {"peer_port": addr[1], "accepted": time.perf_counter()}
        body = b""
        conn.settimeout(10)
        try:
            while b"\n" not in body:
                chunk = conn.recv(4096)
                if not chunk:
                    break
                body += chunk
        except OSError:
            pass
        try:
            record["claim"] = json.loads(body.decode().splitlines()[0])
        except Exception:
            record["claim"] = None
        with self.lock:
            self.seen.append(record)

    def look_up(self, record, delay_s=0.0):
        """Every route's answer for one recorded connection, taken now."""
        if delay_s:
            time.sleep(delay_s)
        with self.lock:
            candidates = list(self.known)
        out = {}
        for name in self.routes:
            t = time.perf_counter()
            if name == "libproc_4tuple":
                got = peerpid.libproc_owner_4tuple(record["peer_port"],
                                                   candidates, self.port)
            else:
                got = peerpid.ROUTES[name](record["peer_port"], candidates)
            out[name] = {"pid": got, "us": (time.perf_counter() - t) * 1e6}
        return out

    def close(self):
        self.stop.set()
        for c in self.held:
            try:
                c.close()
            except OSError:
                pass
        self.sock.close()


def score(got, truth):
    if got is None:
        return "none"
    return "right" if got == truth else "wrong"


# --------------------------------------------------------------- A: reuse

def experiment_reuse():
    """Two processes, one local port, one listener, the first still connected.

    If this succeeds the server is holding two open connections whose peer port
    is the same number, and no lookup keyed on that number can tell them apart.
    """
    srv = Listener(["libproc"])
    first_pid, _, _ = srv.spawn(POOL[0], 5000, "holder", wait=False)
    time.sleep(0.5)
    second_pid, rc, err = srv.spawn(POOL[0], 0, "reuser", wait=True)
    time.sleep(0.5)
    got = {"first_pid": first_pid, "second_pid": second_pid,
           "second_returncode": rc, "second_stderr": err,
           "connections_accepted": len(srv.seen)}
    srv.close()
    subprocess.run(["kill", "-9", str(first_pid)], capture_output=True)
    print("A  reuse of one local port against one listener")
    print("     first pid %s (holding), second pid %s -> rc %s"
          % (first_pid, second_pid, rc))
    print("     %s" % (err or "(no error)"))
    print("     connections the server accepted: %d" % got["connections_accepted"])
    return got


# ----------------------------------------------------------- B: ambiguity

def _ambiguity_once(impostor_first):
    """One live pair sharing a local port number, in a chosen scan order."""
    decoy = socket.socket()
    decoy.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    decoy.bind(("127.0.0.1", 0))
    decoy.listen(8)
    decoy_port = decoy.getsockname()[1]
    stop = threading.Event()

    def take():
        held = []
        while not stop.is_set():
            try:
                held.append(decoy.accept()[0])
            except OSError:
                return

    threading.Thread(target=take, daemon=True).start()
    routes = ["libproc", "libproc_4tuple", "lsof_pids", "lsof_port"]
    srv = Listener(routes)

    def start_impostor():
        p = subprocess.Popen(
            [sys.executable, CLIENT, str(decoy_port), str(POOL[0]), "9000",
             "impostor"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        with srv.lock:
            srv.known.append(p.pid)
        return p.pid

    if impostor_first:
        impostor_pid = start_impostor()
        time.sleep(0.6)
        real_pid, _, _ = srv.spawn(POOL[0], 9000, "real", wait=False)
    else:
        real_pid, _, _ = srv.spawn(POOL[0], 9000, "real", wait=False)
        time.sleep(0.6)
        impostor_pid = start_impostor()
    time.sleep(0.8)

    rows = []
    order = "impostor first" if impostor_first else "impostor second"
    print("     %s: real %s -> Fleet:%d, impostor %s -> decoy:%d, both from"
          " local port %d"
          % (order, real_pid, srv.port, impostor_pid, decoy_port, POOL[0]))
    for record in list(srv.seen):
        if not record.get("claim") or record["claim"]["tag"] != "real":
            continue
        for name, ans in srv.look_up(record).items():
            verdict = score(ans["pid"], real_pid)
            rows.append({"order": order, "route": name,
                         "recovered": ans["pid"], "truth": real_pid,
                         "impostor": impostor_pid, "verdict": verdict,
                         "us": ans["us"]})
            print("       %-16s recovered %-8s %s"
                  % (name, ans["pid"], verdict.upper()))
    stop.set()
    srv.close()
    decoy.close()
    for pid in (real_pid, impostor_pid):
        subprocess.run(["kill", "-9", str(pid)], capture_output=True)
    return rows


def experiment_ambiguity():
    """Two live processes, the same local port number, different destinations.

    Drone A's session connection to Fleet, and Drone B's connection to something
    else entirely, both from local port 24101. Only one of them is the peer on
    Fleet's connection; a lookup keyed on the port number alone has to guess,
    and which way it guesses depends on the order it happens to scan in. Both
    orders are run, because a route that is right in one of them and wrong in
    the other is not a route that is sometimes right.
    """
    print("\nB  one local port number, two live holders, different destinations")
    return _ambiguity_once(False) + _ambiguity_once(True)


# ------------------------------------------------------------ C: lifetime

def experiment_lifetime():
    """How long the peer has to live for the lookup to still find it.

    The client exits `hold` ms after connecting; the server does its lookup as
    soon as the connection is accepted. A route that costs more than `hold` is
    asking about a process that no longer exists.
    """
    routes = ["libproc", "lsof_port"]
    print("\nC  the peer exits `hold` ms after connecting; the lookup starts at"
          " accept")
    out = []
    for hold in HOLDS_MS:
        tally = {r: collections.Counter() for r in routes}
        costs = {r: [] for r in routes}
        for i in range(LIFETIME_N):
            srv = Listener(routes)
            pid, _, _ = srv.spawn(0, hold, "life", wait=False)
            deadline = time.time() + 5
            while not srv.seen and time.time() < deadline:
                time.sleep(0.0005)
            if not srv.seen:
                srv.close()
                continue
            record = srv.seen[0]
            answers = srv.look_up(record)
            for name, ans in answers.items():
                tally[name][score(ans["pid"], pid)] += 1
                costs[name].append(ans["us"])
            srv.close()
            subprocess.run(["kill", "-9", str(pid)], capture_output=True)
        row = {"hold_ms": hold, "n": LIFETIME_N, "routes": {}}
        for name in routes:
            med = sorted(costs[name])[len(costs[name]) // 2] if costs[name] else 0
            row["routes"][name] = {"right": tally[name]["right"],
                                   "none": tally[name]["none"],
                                   "wrong": tally[name]["wrong"],
                                   "median_us": med}
            print("     hold %4dms  %-10s right %2d  none %2d  WRONG %2d "
                  " (lookup %.0f us)"
                  % (hold, name, tally[name]["right"], tally[name]["none"],
                     tally[name]["wrong"], med))
        out.append(row)
    return out


def main():
    peerpid.calibrate()
    result = {"offsets": {"local": peerpid.LOCAL_PORT_OFFSET,
                          "peer": peerpid.PEER_PORT_OFFSET},
              "reuse": experiment_reuse(),
              "ambiguity": experiment_ambiguity(),
              "lifetime": experiment_lifetime()}
    with open(os.path.join(HERE, "012-race.json"), "w") as f:
        json.dump(result, f, indent=2)


main()
