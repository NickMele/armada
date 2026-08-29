#!/usr/bin/env python3
"""Five Drones, one listener, one config file. Is every call attributed?

Spike 10 did this with two and left five as an open question. The server is
[`010-identity-server.py`](010-identity-server.py) unchanged, so the only thing
that varies from that run is the number of Drones and the fact that all five are
started at once — which is what puts five connections into one accept loop and
makes the cost of the lookup visible as serialisation rather than as latency.

Three things are recorded that spike 10 did not need:

  * the wall time between one connection being accepted and the next being
    logged, because `010-identity-server.py` does its `lsof` inside the accept
    loop and a Drone behind four others waits for all four
  * how many TCP connections one session opens over its life
  * what the cheaper lookup of [`012-peerpid.py`](012-peerpid.py) says about the
    same real connections, scored against the pids this script spawned
"""
import importlib
import json
import os
import signal
import subprocess
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
peerpid = importlib.import_module("012-peerpid")

HERE = os.path.dirname(os.path.abspath(__file__))
SERVER = os.path.join(HERE, "010-identity-server.py")
# The native install. `claude` on PATH here is an unrelated shim, and spike 10
# measured the binary rather than somebody's wrapper.
CLI = os.environ.get("CLI", os.path.expanduser("~/.local/bin/claude"))
PORT = int(os.environ.get("PORT", "8951"))
FLEET = int(os.environ.get("FLEET", "5"))
# With LOOKUP=0 the listener is the control below rather than
# `010-identity-server.py`: it accepts, timestamps, and asks nothing. The
# difference between the two runs is the lookup, and everything else — five CLIs
# starting at once, and however long each takes to get to its first HTTP
# request — is common to both.
LOOKUP = os.environ.get("LOOKUP", "1") != "0"
SUFFIX = "" if LOOKUP else "-control"
LOG = os.path.join(HERE, "012-server-five-drones%s.jsonl" % SUFFIX)
CFG = os.path.join(HERE, "cfg-five-drones.json")

CONTROL = r"""
import json, os, socket, sys, threading, time, itertools
log = open(os.environ["IDENTITY_LOG"], "a", buffering=1)
s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("127.0.0.1", int(sys.argv[1]))); s.listen(16)
n = itertools.count(1)
def serve(conn, cid):
    f = conn.makefile("rwb")
    while True:
        line = f.readline()
        if not line: break
        headers = {}
        while True:
            raw = f.readline()
            if not raw or raw in (b"\r\n", b"\n"): break
            k, _, v = raw.decode("latin1").partition(":")
            headers[k.strip().lower()] = v.strip()
        body = f.read(int(headers.get("content-length") or 0))
        out = []
        for chunk in body.decode().splitlines():
            if not chunk.strip(): continue
            req = json.loads(chunk)
            m, rid = req.get("method"), req.get("id")
            if m == "initialize":
                out.append({"jsonrpc":"2.0","id":rid,"result":{
                    "protocolVersion": (req.get("params") or {}).get(
                        "protocolVersion") or "2025-06-18",
                    "capabilities":{"tools":{}},
                    "serverInfo":{"name":"armada-012-control","version":"0.1"}}})
            elif m == "tools/list":
                out.append({"jsonrpc":"2.0","id":rid,"result":{"tools":[{
                    "name":"whoami","description":"Report which Job you believe"
                    " you are working on. Call it once.","inputSchema":{
                    "type":"object","properties":{"job":{"type":"string"}},
                    "required":["job"],"additionalProperties":False}}]}})
            elif m == "tools/call":
                log.write(json.dumps({"t": time.time(), "kind": "tool_call",
                    "payload": {"connection": cid,
                    "arguments": (req.get("params") or {}).get("arguments"),
                    "observed": {}}}) + "\n")
                out.append({"jsonrpc":"2.0","id":rid,"result":{
                    "content":[{"type":"text","text":"seen"}],"isError":False}})
            elif rid is not None:
                out.append({"jsonrpc":"2.0","id":rid,"result":{}})
        if out:
            p = json.dumps(out[0] if len(out) == 1 else out).encode()
            f.write(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n"
                    b"Content-Length: %d\r\n\r\n%s" % (len(p), p))
        else:
            f.write(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n")
        f.flush()
while True:
    conn, addr = s.accept()
    cid = next(n)
    log.write(json.dumps({"t": time.time(), "kind": "connection", "payload": {
        "observed": {"id": cid, "peer": list(addr), "pid": None}}}) + "\n")
    threading.Thread(target=serve, args=(conn, cid), daemon=True).start()
"""


def spawn_drone(job, tools, task):
    args = [CLI, "-p", "--input-format", "stream-json", "--output-format",
            "stream-json", "--verbose", "--replay-user-messages",
            "--model", "haiku", "--permission-mode", "dontAsk",
            "--allowedTools", *tools,
            "--strict-mcp-config", "--mcp-config", CFG]
    p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.DEVNULL, text=True)
    p.stdin.write(json.dumps({"type": "user", "message": {
        "role": "user", "content": [{"type": "text", "text": task}]}}) + "\n")
    p.stdin.flush()
    return p


def drain(p, done, sink):
    for line in p.stdout:
        sink.write(line)
        if '"type":"result"' in line:
            break
    done.set()


def main():
    if os.path.exists(LOG):
        os.unlink(LOG)
    with open(CFG, "w") as f:
        json.dump({"mcpServers": {"armada": {
            "type": "http", "url": "http://127.0.0.1:%d/mcp" % PORT}}}, f)

    cmd = ([sys.executable, SERVER, "tcp", str(PORT)] if LOOKUP
           else [sys.executable, "-c", CONTROL, str(PORT)])
    srv = subprocess.Popen(cmd, env=dict(os.environ, IDENTITY_LOG=LOG),
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                           text=True)
    time.sleep(0.8)

    jobs = ["JOB-%s" % c for c in "ABCDE"[:FLEET]]
    task = ("Call the whoami tool exactly once with job=\"%s\". "
            "Say nothing else.")
    drones, sinks, dones = {}, {}, []
    started = time.time()
    for job in jobs:
        p = spawn_drone(job, ["mcp__armada__whoami"], task % job)
        drones[p.pid] = job
        sink = open(os.path.join(
            HERE, "012-transcript-%s%s.ndjson" % (job, SUFFIX)), "w")
        sinks[p.pid] = (p, sink)
        done = threading.Event()
        dones.append(done)
        threading.Thread(target=drain, args=(p, done, sink), daemon=True).start()
    print("spawned in %.0f ms: %s"
          % ((time.time() - started) * 1000,
             json.dumps({v: k for k, v in drones.items()})))

    # The cheap lookup, against the same live connections. The server has
    # already logged each one (its own `lsof` took ~65ms), and an `http` session
    # holds its connection open, so the socket is still there to be asked about.
    cheap = []
    watching = threading.Event()

    def watch():
        seen, offset = set(), 0
        while not watching.is_set():
            if os.path.exists(LOG):
                with open(LOG) as fh:
                    fh.seek(offset)
                    for line in fh:
                        offset += len(line)
                        d = json.loads(line)
                        if d["kind"] != "connection":
                            continue
                        o = d["payload"]["observed"]
                        if o["id"] in seen:
                            continue
                        seen.add(o["id"])
                        peer = (o.get("peer") or [None, None])[1]
                        t = time.perf_counter()
                        got = peerpid.libproc_owner_4tuple(
                            peer, list(drones), PORT)
                        cheap.append({"connection": o["id"], "peer": peer,
                                      "lsof_pid": o.get("pid"),
                                      "libproc_pid": got,
                                      "us": (time.perf_counter() - t) * 1e6})
            time.sleep(0.02)

    if LOOKUP:
        peerpid.calibrate()
        threading.Thread(target=watch, daemon=True).start()

    for d in dones:
        d.wait(240)
    watching.set()
    for p, sink in sinks.values():
        p.kill()
        p.wait()
        sink.close()
    srv.send_signal(signal.SIGTERM)
    try:
        srv.wait(5)
    except subprocess.TimeoutExpired:
        srv.kill()

    report(drones, cheap)


def report(drones, cheap):
    conns, calls, first = {}, [], None
    for line in open(LOG):
        d = json.loads(line)
        if d["kind"] == "connection":
            o = d["payload"]["observed"]
            if first is None:
                first = d["t"]
            conns[o["id"]] = {"at": d["t"] - first, "peer": o.get("peer"),
                              "pid": o.get("pid"),
                              "root": (o.get("ancestry") or [{}])[-1].get("pid")}
        if d["kind"] == "tool_call":
            calls.append({"connection": d["payload"].get("connection"),
                          "args": d["payload"].get("arguments"),
                          "pid": d["payload"]["observed"].get("pid")})

    print("\nconnections, in the order the server logged them:")
    print("  %-4s %-9s %-9s %-8s %-9s %s"
          % ("conn", "t+ms", "peer port", "pid", "job", "verdict"))
    rows = []
    for cid, c in sorted(conns.items()):
        job = drones.get(c["pid"])
        verdict = ("no lookup (control)" if not LOOKUP else
                   "attributed" if job else
                   "unattributable" if c["pid"] is None else
                   "not a spawned Drone")
        rows.append(dict(c, id=cid, job=job, verdict=verdict))
        print("  %-4s %-9.0f %-9s %-8s %-9s %s"
              % (cid, c["at"] * 1000, (c["peer"] or [None, None])[1],
                 c["pid"], job or "-", verdict))

    per_drone = {}
    for r in rows:
        per_drone.setdefault(r["pid"], []).append(r["id"])
    print("\nconnections per Drone: %s"
          % json.dumps({str(drones.get(k, k)): len(v)
                        for k, v in per_drone.items()}))

    print("\ntool calls, payload against transport:")
    call_rows = []
    for c in calls:
        said = (c["args"] or {}).get("job")
        was = drones.get(c["pid"])
        agree = None if not LOOKUP else said == was
        call_rows.append({"claimed": said, "transport_says": was,
                          "pid": c["pid"], "agree": agree})
        print("  conn %-3s pid %-8s payload %-8s transport %-8s %s"
              % (c["connection"], c["pid"], said, was,
                 "no lookup (control)" if agree is None else
                 "agree" if agree else "DISAGREE"))

    if cheap:
        print("\nthe same live connections, asked the cheaper way:")
        for c in cheap:
            same = c["lsof_pid"] == c["libproc_pid"]
            print("  conn %-3s peer %-6s lsof %-8s libproc %-8s %-9s %.0f us"
                  % (c["connection"], c["peer"], c["lsof_pid"],
                     c["libproc_pid"], "agree" if same else "DISAGREE",
                     c["us"]))

    gaps = [rows[i + 1]["at"] - rows[i]["at"] for i in range(len(rows) - 1)]
    print("\ngaps between logged connections (ms): %s"
          % ", ".join("%.0f" % (g * 1000) for g in gaps))

    with open(os.path.join(
            HERE, "012-five-drones%s.json" % SUFFIX), "w") as f:
        json.dump({"fleet": len(drones), "drones": {str(k): v for k, v in
                                                    drones.items()},
                   "connections": rows, "calls": call_rows,
                   "cheap_lookup": cheap,
                   "gaps_ms": [g * 1000 for g in gaps]}, f, indent=2)


main()
