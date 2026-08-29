#!/usr/bin/env python3
"""Run one Drone-shaped invocation per candidate MCP transport and record, for
each, what the CLI reported about the server and what the server observed about
the caller.

Each case writes its own config file and its own server log. Nothing touches the
operator's own MCP configuration: the config path is under this directory and
`--strict-mcp-config` is on every invocation, as it is on a real Drone's.
"""
import json
import os
import shutil
import signal
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
SERVER = os.path.join(HERE, "010-identity-server.py")
CLI = os.environ.get("CLI", "claude")
PORT = int(os.environ.get("PORT", "8931"))
SOCK = os.path.join(HERE, "armada.sock")
PY = shutil.which("python3")

CASES = {
    # The transport a Drone is spawned against today.
    "http": {"kind": "tcp", "server": {"type": "http", "url": "http://127.0.0.1:%d/mcp" % PORT}},
    "streamable-http": {"kind": "tcp", "server": {"type": "streamable-http", "url": "http://127.0.0.1:%d/mcp" % PORT}},
    "sse": {"kind": "tcp", "server": {"type": "sse", "url": "http://127.0.0.1:%d/sse" % PORT}},
    "ws": {"kind": "tcp", "server": {"type": "ws", "url": "ws://127.0.0.1:%d/mcp" % PORT}},
    "stdio": {"kind": "none", "server": {"type": "stdio", "command": PY, "args": [SERVER, "stdio"]}},
    "stdio-implicit": {"kind": "none", "server": {"command": PY, "args": [SERVER, "stdio"]}},
    "sdk": {"kind": "none", "server": {"type": "sdk", "name": "armada"}},
    # Invented and near-miss spellings: is a unix socket reachable at all?
    "unix-type": {"kind": "unix", "server": {"type": "unix", "url": SOCK}},
    "unix-url": {"kind": "unix", "server": {"type": "http", "url": "unix://%s" % SOCK}},
    "http+unix-url": {"kind": "unix", "server": {"type": "http", "url": "http+unix://%s" % SOCK.replace("/", "%2F")}},
    "unix-path-url": {"kind": "unix", "server": {"type": "http", "url": "http://localhost/mcp", "socketPath": SOCK}},
}


def start_server(kind, log_path):
    if kind == "none":
        return None
    env = dict(os.environ, IDENTITY_LOG=log_path)
    args = [PY, SERVER, "tcp", str(PORT)] if kind == "tcp" else [PY, SERVER, "unix", SOCK]
    proc = subprocess.Popen(args, env=env, stdout=subprocess.DEVNULL,
                            stderr=subprocess.PIPE, text=True)
    time.sleep(0.7)
    return proc


def probe(config_path, log_path, timeout):
    """Start the CLI the way `adapters::harness` renders a Drone, send one turn,
    and stop as soon as the session announces itself."""
    args = [
        CLI, "-p",
        "--input-format", "stream-json",
        "--output-format", "stream-json",
        "--verbose",
        "--replay-user-messages",
        "--model", "haiku",
        "--permission-mode", "dontAsk",
        "--strict-mcp-config",
        "--mcp-config", config_path,
    ]
    env = dict(os.environ, IDENTITY_LOG=log_path)
    proc = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, text=True, env=env)
    proc.stdin.write(json.dumps({
        "type": "user",
        "message": {"role": "user", "content": [{"type": "text", "text":
            "Call the whoami tool once with job=\"J1\", then stop."}]},
    }) + "\n")
    proc.stdin.flush()
    init, lines, deadline = None, [], time.time() + timeout
    import threading
    done = threading.Event()

    def reader():
        for line in proc.stdout:
            lines.append(line.rstrip())
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                continue
            if msg.get("type") == "system" and msg.get("subtype") == "init":
                nonlocal_init.append(msg)
            if msg.get("type") == "result":
                break
        done.set()

    nonlocal_init = []
    threading.Thread(target=reader, daemon=True).start()
    done.wait(timeout)
    proc.kill()
    err = proc.stderr.read()
    proc.wait()
    init = nonlocal_init[0] if nonlocal_init else None
    return {"init": init, "lines": lines, "stderr": err, "pid": proc.pid}


def main(only=None):
    results = {}
    for name, case in CASES.items():
        if only and name not in only:
            continue
        cfg = os.path.join(HERE, "cfg-%s.json" % name)
        log_path = os.path.join(HERE, "010-server-%s.jsonl" % name)
        for stale in (cfg, log_path):
            if os.path.exists(stale):
                os.unlink(stale)
        with open(cfg, "w") as f:
            json.dump({"mcpServers": {"armada": case["server"]}}, f)
        srv = start_server(case["kind"], log_path)
        try:
            out = probe(cfg, log_path, timeout=int(os.environ.get("TIMEOUT", "150")))
        finally:
            if srv:
                srv.send_signal(signal.SIGTERM)
                try:
                    srv.wait(5)
                except subprocess.TimeoutExpired:
                    srv.kill()
        init = out["init"] or {}
        results[name] = {
            "config": case["server"],
            "mcp_servers": init.get("mcp_servers"),
            "mcp_tools": [t for t in init.get("tools", []) if t.startswith("mcp__")],
            "stderr": out["stderr"][-2000:],
            "server_log_lines": sum(1 for _ in open(log_path)) if os.path.exists(log_path) else 0,
        }
        print(name, json.dumps(results[name]["mcp_servers"]),
              "server log lines:", results[name]["server_log_lines"], flush=True)
    with open(os.path.join(HERE, "010-results.json"), "w") as f:
        json.dump(results, f, indent=2)


main(sys.argv[1:] or None)
