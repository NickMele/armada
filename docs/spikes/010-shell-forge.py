#!/usr/bin/env python3
"""A Drone that can run a shell, connecting to the endpoint by hand.

`docs/scope.md` declines to build confinement, so the interesting question is
what a peer pid says about a call the Drone made with `curl` rather than with
the tool. The Drone here is told to do exactly that, and the server records the
pid it recovered and that pid's ancestry.
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
PORT = int(os.environ.get("PORT", "8939"))
LOG = os.path.join(HERE, "010-server-shell-forge.jsonl")
CFG = os.path.join(HERE, "cfg-shell-forge.json")

BODY = json.dumps({"jsonrpc": "2.0", "id": 99, "method": "tools/call",
                   "params": {"name": "whoami", "arguments": {"job": "JOB-Z"}}})
TASK = (
    "This is a local connectivity check against a test server on loopback.\n"
    "Run exactly this one command with Bash and then stop:\n"
    "curl -sS -X POST http://127.0.0.1:%d/mcp "
    "-H 'Content-Type: application/json' "
    "-H 'Accept: application/json, text/event-stream' "
    "-d '%s'\n" % (PORT, BODY)
)


def main():
    if os.path.exists(LOG):
        os.unlink(LOG)
    with open(CFG, "w") as f:
        json.dump({"mcpServers": {"armada": {
            "type": "http", "url": "http://127.0.0.1:%d/mcp" % PORT}}}, f)
    srv = subprocess.Popen([sys.executable, SERVER, "tcp", str(PORT)],
                           env=dict(os.environ, IDENTITY_LOG=LOG),
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    time.sleep(0.8)
    args = [CLI, "-p", "--input-format", "stream-json", "--output-format",
            "stream-json", "--verbose", "--model", "haiku",
            "--permission-mode", "dontAsk",
            "--allowedTools", "Bash", "mcp__armada__whoami",
            "--strict-mcp-config", "--mcp-config", CFG]
    p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.DEVNULL, text=True)
    print("drone pid:", p.pid)
    p.stdin.write(json.dumps({"type": "user", "message": {
        "role": "user", "content": [{"type": "text", "text": TASK}]}}) + "\n")
    p.stdin.flush()
    done = threading.Event()

    transcript = open(os.path.join(HERE, "010-transcript-shell-forge.ndjson"), "w")

    def drain():
        for line in p.stdout:
            transcript.write(line)
            transcript.flush()
            if '"type":"result"' in line:
                break
        done.set()

    threading.Thread(target=drain, daemon=True).start()
    done.wait(180)
    p.kill()
    p.wait()
    srv.send_signal(signal.SIGTERM)
    try:
        srv.wait(5)
    except subprocess.TimeoutExpired:
        srv.kill()

    print("\nwhat the server saw:")
    for line in open(LOG):
        d = json.loads(line)
        if d["kind"] == "connection":
            o = d["payload"]["observed"]
            print("  connection %s pid=%s %s" % (o["id"], o.get("pid"), o.get("ps")))
            print("    ancestry:", " <- ".join(
                "%s %s" % (a["pid"], os.path.basename(a["comm"]))
                for a in o.get("ancestry") or []))
        if d["kind"] == "tool_call":
            print("  tool_call conn %s args=%s"
                  % (d["payload"].get("connection"),
                     json.dumps(d["payload"].get("arguments"))))


main()
