#!/usr/bin/env python3
"""One spike case: spawn `claude -p` the way Armada spawns a Drone (stream-json
in and out, strict MCP config, a narrow --allowedTools), plus a
--permission-prompt-tool pointing at stub_mcp.py. Ask it to run a Bash command
that is not on the allowlist, and record what reaches the stub and what the
session does.

usage: run_case.py NAME WAIT_SECONDS ANSWER MODE   (MODE: a --permission-mode value, or unset)
"""
import json, os, subprocess, sys, time

name, wait, answer, mode = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
here = os.path.dirname(os.path.abspath(__file__))
out = os.path.join(here, "cases", name)
os.makedirs(out, exist_ok=True)
work = os.path.join(out, "cwd")
os.makedirs(work, exist_ok=True)

mcp = {"mcpServers": {"stub": {
    "command": "python3",
    "args": [os.path.join(here, "stub_mcp.py")],
    "env": {"STUB_LOG": os.path.join(out, "stub.log"), "WAIT_SECONDS": wait, "ANSWER": answer},
}}}
with open(os.path.join(out, "mcp.json"), "w") as f:
    json.dump(mcp, f)

args = ["claude", "-p",
        "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
        "--model", "haiku",
        "--no-session-persistence",
        "--strict-mcp-config", "--mcp-config", os.path.join(out, "mcp.json"),
        "--allowedTools", "Read",
        "--permission-prompt-tool", "mcp__stub__approve"]
if os.environ.get("SPIKE_USER_SETTINGS") != "1":
    args += ["--setting-sources", "local"]
if mode != "unset":
    args += ["--permission-mode", mode]

command = os.environ.get("SPIKE_COMMAND", "touch spike-marker.txt && echo spike-ok")
prompt = (f"Use the Bash tool to run exactly this command: {command}\n"
          "Then reply with one line saying what it printed, or that you could not run it.")
turn = {"type": "user", "message": {"role": "user", "content": prompt}}

started = time.time()
limit = float(wait) + 240
with open(os.path.join(out, "session.jsonl"), "w") as record:
    env = {**os.environ, "GIT_DIR": os.path.join(work, "no-such-git-dir")}
    proc = subprocess.Popen(args, cwd=work, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
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
summary = {
    "case": name, "wait": float(wait), "answer": answer, "mode": mode, "elapsed": elapsed,
    "marker_written": os.path.exists(os.path.join(work, "spike-marker.txt")),
    "result": None if result is None else {k: result.get(k) for k in
              ("subtype", "is_error", "result", "permission_denials", "num_turns")},
}
with open(os.path.join(out, "summary.json"), "w") as f:
    json.dump(summary, f, indent=1)
print(json.dumps(summary))
