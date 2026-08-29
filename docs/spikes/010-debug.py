#!/usr/bin/env python3
"""Same invocation, with `--debug mcp`, to read what the CLI says about a
transport it would not connect. A status of `failed` in the init line is not an
answer; the debug line naming the error is."""
import json
import os
import subprocess
import sys
import threading

HERE = os.path.dirname(os.path.abspath(__file__))
CLI = os.environ.get("CLI", "claude")


def run(cfg, timeout=90):
    args = [CLI, "-p", "--input-format", "stream-json", "--output-format",
            "stream-json", "--verbose", "--debug", "mcp", "--model", "haiku",
            "--permission-mode", "dontAsk", "--strict-mcp-config",
            "--debug-file", cfg + ".debug.log",
            "--mcp-config", cfg]
    proc = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, text=True)
    proc.stdin.write(json.dumps({"type": "user", "message": {
        "role": "user", "content": [{"type": "text", "text": "stop"}]}}) + "\n")
    proc.stdin.flush()
    out, err = [], []
    done = threading.Event()

    def pump(stream, into, stop_on_result):
        for line in stream:
            into.append(line.rstrip())
            if stop_on_result and '"type":"result"' in line:
                done.set()
        done.set()

    threading.Thread(target=pump, args=(proc.stdout, out, True), daemon=True).start()
    threading.Thread(target=pump, args=(proc.stderr, err, False), daemon=True).start()
    done.wait(timeout)
    proc.kill()
    proc.wait()
    return out, err


for name in sys.argv[1:]:
    cfg = os.path.join(HERE, "cfg-%s.json" % name)
    out, err = run(cfg)
    print("=== %s  %s" % (name, open(cfg).read().strip()))
    for line in err:
        print("  stderr:", line[:500])
    try:
        for line in open(cfg + ".debug.log"):
            low = line.lower()
            if "mcp" in low or "unix" in low or "socket" in low or "fetch" in low:
                print("  debug:", line.rstrip()[:500])
    except OSError as why:
        print("  no debug file:", why)
    for line in out:
        if '"subtype":"init"' in line:
            init = json.loads(line)
            print("  mcp_servers:", json.dumps(init.get("mcp_servers")))
    print()
