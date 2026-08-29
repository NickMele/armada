#!/usr/bin/env python3
"""Does `--mcp-config` take the document itself rather than a path? The flag's
help says "JSON files or strings"; this is whether a Drone could be configured
without a file existing on disk at all — and what `ps` then shows."""
import json
import os
import subprocess
import threading

CLI = os.environ.get("CLI", "claude")
DOC = json.dumps({"mcpServers": {"armada": {
    "type": "http", "url": "http://127.0.0.1:8937/mcp"}}})

args = [CLI, "-p", "--input-format", "stream-json", "--output-format",
        "stream-json", "--verbose", "--model", "haiku",
        "--permission-mode", "dontAsk", "--strict-mcp-config",
        "--mcp-config", DOC]
p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                     stderr=subprocess.PIPE, text=True)
p.stdin.write(json.dumps({"type": "user", "message": {"role": "user", "content": [
    {"type": "text", "text": "stop"}]}}) + "\n")
p.stdin.flush()
init, done = [], threading.Event()


def reader():
    for line in p.stdout:
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("type") == "system" and msg.get("subtype") == "init":
            init.append(msg)
            done.set()
            return
    done.set()


threading.Thread(target=reader, daemon=True).start()
done.wait(0.5)
# What the machine can see of the invocation while it runs.
ps = subprocess.run(["ps", "-o", "args=", "-p", str(p.pid)],
                    capture_output=True, text=True).stdout.strip()
done.wait(90)
p.kill()
p.wait()
print("mcp_servers:", json.dumps(init[0].get("mcp_servers")) if init else "no init")
print("ps shows the document:", "mcpServers" in ps)
print("ps args:", ps[:400])
