#!/usr/bin/env python3
"""One Drone, many connections at once, made by hand rather than by the client.

Spike 10 showed a single `curl` from a Drone's own Bash arriving attributable
through its ancestry. It left open what happens when several land together —
which is the case that matters, because the ancestry walk is the expensive part
and a burst is where an implementation would be tempted to cache or to skip.

The Drone is told to run four `curl`s in parallel, each carrying a different Job
name in its body, and the receiving side records the pid and ancestry of each.
The framing is deliberately neutral: spike 10's first attempt asked for a Job
name of `FORGED` and the Drone refused it as impersonation, which is a fact
about the model and not about the transport.
"""
import json
import os
import signal
import subprocess
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

HERE = os.path.dirname(os.path.abspath(__file__))
SERVER = os.path.join(HERE, "010-identity-server.py")
CLI = os.environ.get("CLI", os.path.expanduser("~/.local/bin/claude"))
PORT = int(os.environ.get("PORT", "8953"))
LOG = os.path.join(HERE, "012-server-parallel-curl.jsonl")
CFG = os.path.join(HERE, "cfg-parallel-curl.json")
FAN = 4


def body(job):
    return json.dumps({"jsonrpc": "2.0", "id": 99, "method": "tools/call",
                       "params": {"name": "whoami",
                                  "arguments": {"job": job}}})


def main():
    if os.path.exists(LOG):
        os.unlink(LOG)
    with open(CFG, "w") as f:
        json.dump({"mcpServers": {"armada": {
            "type": "http", "url": "http://127.0.0.1:%d/mcp" % PORT}}}, f)

    curls = " & ".join(
        "curl -sS -X POST http://127.0.0.1:%d/mcp "
        "-H 'Content-Type: application/json' "
        "-H 'Accept: application/json, text/event-stream' -d '%s'"
        % (PORT, body("JOB-P%d" % i)) for i in range(1, FAN + 1))
    task = ("This is a local connectivity check against a test server on "
            "loopback. Run exactly this one command with Bash, which starts "
            "%d requests at the same time, and then stop:\n%s & wait\n"
            % (FAN, curls))

    srv = subprocess.Popen([sys.executable, SERVER, "tcp", str(PORT)],
                           env=dict(os.environ, IDENTITY_LOG=LOG),
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                           text=True)
    time.sleep(0.8)

    args = [CLI, "-p", "--input-format", "stream-json", "--output-format",
            "stream-json", "--verbose", "--replay-user-messages",
            "--model", "haiku", "--permission-mode", "dontAsk",
            "--allowedTools", "Bash", "mcp__armada__whoami",
            "--strict-mcp-config", "--mcp-config", CFG]
    p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.DEVNULL, text=True)
    print("drone pid:", p.pid)
    p.stdin.write(json.dumps({"type": "user", "message": {
        "role": "user", "content": [{"type": "text", "text": task}]}}) + "\n")
    p.stdin.flush()

    done = threading.Event()
    sink = open(os.path.join(HERE, "012-transcript-parallel-curl.ndjson"), "w")

    def drain():
        for line in p.stdout:
            sink.write(line)
            if '"type":"result"' in line:
                break
        done.set()

    threading.Thread(target=drain, daemon=True).start()
    done.wait(300)
    p.kill()
    p.wait()
    sink.close()
    srv.send_signal(signal.SIGTERM)
    try:
        srv.wait(5)
    except subprocess.TimeoutExpired:
        srv.kill()

    rows, first = [], None
    for line in open(LOG):
        d = json.loads(line)
        if d["kind"] == "connection":
            o = d["payload"]["observed"]
            if first is None:
                first = d["t"]
            chain = " <- ".join("%s %s" % (h["pid"], h["comm"])
                                for h in (o.get("ancestry") or []))
            rows.append({"kind": "connection", "id": o["id"],
                         "at_ms": (d["t"] - first) * 1000,
                         "pid": o.get("pid"), "ancestry": chain,
                         "traces_to_drone": p.pid in
                         [h["pid"] for h in (o.get("ancestry") or [])]})
        if d["kind"] == "tool_call":
            rows.append({"kind": "tool_call",
                         "connection": d["payload"].get("connection"),
                         "pid": d["payload"]["observed"].get("pid"),
                         "arguments": d["payload"].get("arguments")})

    print("\nwhat the server saw (drone pid %d):" % p.pid)
    for r in rows:
        if r["kind"] == "connection":
            print("  conn %-3s t+%-7.0f pid %-8s %-5s  %s"
                  % (r["id"], r["at_ms"], r["pid"],
                     "OWN" if r["traces_to_drone"] else "-", r["ancestry"]))
        else:
            print("      call on conn %-3s pid %-8s args %s"
                  % (r["connection"], r["pid"], json.dumps(r["arguments"])))

    with open(os.path.join(HERE, "012-parallel-curl.json"), "w") as f:
        json.dump({"drone_pid": p.pid, "fan": FAN, "rows": rows}, f, indent=2)


main()
