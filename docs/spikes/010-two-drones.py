#!/usr/bin/env python3
"""Two Drones, one listener, one config: can the receiving side tell them apart?

This is the shape `#50` puts Fleet in. Both CLIs are spawned from the same MCP
config file, both connect to the same address, and both are told to call the
same tool. What is recorded is the pid the server recovered per connection and
the pid the runner actually spawned, so "the server knew which Drone" is a
comparison rather than a claim.
"""
import json
import os
import signal
import subprocess
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
SERVER = os.path.join(HERE, "010-identity-server.py")
CLI = os.environ.get("CLI", "claude")
PORT = int(os.environ.get("PORT", "8933"))
LOG = os.path.join(HERE, "010-server-two-drones.jsonl")


def spawn_drone(cfg, job):
    args = [CLI, "-p", "--input-format", "stream-json", "--output-format",
            "stream-json", "--verbose", "--replay-user-messages",
            "--model", "haiku", "--permission-mode", "dontAsk",
            "--allowedTools", "mcp__armada__whoami",
            "--strict-mcp-config", "--mcp-config", cfg]
    p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.DEVNULL, text=True)
    p.stdin.write(json.dumps({"type": "user", "message": {"role": "user", "content": [
        {"type": "text", "text":
         "Call the whoami tool exactly once with job=\"%s\". Say nothing else." % job}]}}) + "\n")
    p.stdin.flush()
    return p


def drain(p, done):
    for line in p.stdout:
        if '"type":"result"' in line:
            break
    done.set()


def main():
    for stale in (LOG,):
        if os.path.exists(stale):
            os.unlink(stale)
    cfg = os.path.join(HERE, "cfg-two-drones.json")
    with open(cfg, "w") as f:
        json.dump({"mcpServers": {"armada": {
            "type": "http", "url": "http://127.0.0.1:%d/mcp" % PORT}}}, f)

    srv = subprocess.Popen(
        [sys.executable, SERVER, "tcp", str(PORT)],
        env=dict(os.environ, IDENTITY_LOG=LOG),
        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    time.sleep(0.8)

    a = spawn_drone(cfg, "JOB-A")
    b = spawn_drone(cfg, "JOB-B")
    print("spawned pids:", {"JOB-A": a.pid, "JOB-B": b.pid})
    da, db = threading.Event(), threading.Event()
    threading.Thread(target=drain, args=(a, da), daemon=True).start()
    threading.Thread(target=drain, args=(b, db), daemon=True).start()
    da.wait(180)
    db.wait(180)
    for p in (a, b):
        p.kill()
        p.wait()
    srv.send_signal(signal.SIGTERM)
    try:
        srv.wait(5)
    except subprocess.TimeoutExpired:
        srv.kill()

    print("\nwhat the server saw:")
    conns = {}
    for line in open(LOG):
        d = json.loads(line)
        if d["kind"] == "connection":
            o = d["payload"]["observed"]
            conns[o["id"]] = o
            print("  connection %s  peer=%s  pid=%s  %s"
                  % (o["id"], o.get("peer"), o.get("pid"), o.get("ps")))
        if d["kind"] == "tool_call":
            o = d["payload"]["observed"]
            print("  tool_call on connection %s  pid=%s  args=%s"
                  % (d["payload"].get("connection"), o.get("pid"),
                     json.dumps(d["payload"].get("arguments"))))


main()
