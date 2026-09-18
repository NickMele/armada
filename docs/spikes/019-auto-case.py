#!/usr/bin/env python3
"""One spike-19 case: spawn `claude -p` the way Fleet spawns a Helm session —
stream-json in and out, the door passed on --mcp-config, no
--strict-mcp-config, no --allowedTools — under a named --permission-mode, with
a --permission-prompt-tool pointing at 015's stdio stub.

usage: 019-auto-case.py NAME MODE ANSWER WAIT_SECONDS [--no-tool] [--strict]

The prompt comes from SPIKE_PROMPT. Everything is written under SPIKE_OUT.
"""
import json, os, subprocess, sys, time

name, mode, answer, wait = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
flags = sys.argv[5:]
here = os.path.dirname(os.path.abspath(__file__))
out = os.path.join(os.environ.get("SPIKE_OUT", os.path.join(here, "cases")), name)
os.makedirs(out, exist_ok=True)
work = os.environ["SPIKE_CWD"]

mcp = {"mcpServers": {"stub": {
    "command": "python3",
    "args": [os.path.join(here, "015-stdio-stub.py")],
    "env": {"STUB_LOG": os.path.join(out, "tool.log"), "WAIT_SECONDS": wait, "ANSWER": answer},
}}}
with open(os.path.join(out, "mcp.json"), "w") as f:
    json.dump(mcp, f)

args = ["claude", "-p",
        "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
        "--model", os.environ.get("SPIKE_MODEL", "haiku"),
        "--no-session-persistence",
        "--mcp-config", os.path.join(out, "mcp.json"),
        "--permission-mode", mode]
if "--strict" in flags:
    args += ["--strict-mcp-config"]
if "--no-tool" not in flags:
    args += ["--permission-prompt-tool", "mcp__stub__approve"]

prompt = os.environ["SPIKE_PROMPT"]
turn = {"type": "user", "message": {"role": "user", "content": prompt}}

started = time.time()
limit = float(wait) + float(os.environ.get("SPIKE_LIMIT", "300"))
with open(os.path.join(out, "session.jsonl"), "w") as record:
    proc = subprocess.Popen(args, cwd=work, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=open(os.path.join(out, "stderr.txt"), "w"), text=True)
    proc.stdin.write(json.dumps(turn) + "\n")
    proc.stdin.flush()
    result = None
    for line in proc.stdout:
        record.write(json.dumps({"t": round(time.time() - started, 1), "line": line.rstrip()}) + "\n")
        record.flush()
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("type") == "result":
            result = event
            break
        if time.time() - started > limit:
            break
    proc.stdin.close()
    try:
        proc.wait(timeout=20)
    except subprocess.TimeoutExpired:
        proc.kill()

elapsed = round(time.time() - started, 1)
asked = []
log = os.path.join(out, "tool.log")
if os.path.exists(log):
    for line in open(log):
        entry = json.loads(line)
        if entry.get("event") == "asked":
            asked.append({"tool_name": entry.get("tool_name"), "input": entry.get("input")})
summary = {
    "case": name, "mode": mode, "answer": answer, "wait": float(wait), "flags": flags,
    "elapsed": elapsed, "tool_asked": asked,
    "marker": os.path.exists(os.path.join(work, "spike-marker.txt")),
    "result": None if result is None else {k: result.get(k) for k in
              ("subtype", "is_error", "result", "permission_denials", "num_turns")},
}
with open(os.path.join(out, "summary.json"), "w") as f:
    json.dump(summary, f, indent=1)
print(json.dumps(summary, indent=1)[:4000])
