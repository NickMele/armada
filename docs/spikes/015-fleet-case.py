#!/usr/bin/env python3
"""One refusal, spawned with exactly the flags Fleet's harness renders today:
dontAsk, stream-json both ways, --replay-user-messages, --verbose, a strict
MCP config, --allowedTools Read. Records whether the stream carries a
system/permission_denied line, which is the only thing Fleet turns into a
refusal.

PROMPT_TOOL=1       the asking mode, with a permission tool answering
HTTP_WAIT=SECONDS   serve that tool over HTTP (http_stub.py), waiting this long
ANSWER=allow|deny   what the tool answers (default deny)

usage: fleet_case.py NAME
"""
import json, os, subprocess, sys, time

name = sys.argv[1]
here = os.path.dirname(os.path.abspath(__file__))
out = os.path.join(here, "cases", name)
work = os.path.join(out, "cwd")
os.makedirs(work, exist_ok=True)

asking = os.environ.get("PROMPT_TOOL") == "1"
http_wait = os.environ.get("HTTP_WAIT")
answer = os.environ.get("ANSWER", "deny")
servers = {}
stub = None
dead_url = os.environ.get("DEAD_URL")
if asking and dead_url:
    servers["stub"] = {"type": "http", "url": dead_url}
elif asking and http_wait:
    port = int(os.environ.get("PORT", "47811"))
    stub = subprocess.Popen(["python3", os.path.join(here, "http_stub.py"), str(port), http_wait,
                             answer, os.path.join(out, "stub.log")])
    time.sleep(1)
    servers["stub"] = {"type": "http", "url": f"http://127.0.0.1:{port}/mcp"}
elif asking:
    servers["stub"] = {"command": "python3", "args": [os.path.join(here, "stub_mcp.py")],
                       "env": {"STUB_LOG": os.path.join(out, "stub.log"),
                               "WAIT_SECONDS": "0", "ANSWER": answer}}
mcp_path = os.path.join(out, "mcp.json")
with open(mcp_path, "w") as f:
    json.dump({"mcpServers": servers}, f)

args = ["claude", "-p",
        "--input-format", "stream-json", "--output-format", "stream-json",
        "--verbose", "--replay-user-messages",
        "--model", "haiku",
        "--permission-mode", "default" if asking else "dontAsk",
        "--strict-mcp-config", "--mcp-config", mcp_path,
        "--allowedTools", "Read"]
if asking:
    args += ["--permission-prompt-tool", "mcp__stub__approve"]

prompt = ("Use the Bash tool to run exactly this command: touch spike-marker.txt\n"
          "Then reply with one line saying whether it ran.")
turn = {"type": "user", "message": {"role": "user", "content": prompt}}
limit = float(http_wait or 0) + 240

started = time.time()
lines = []
env = {**os.environ, "GIT_DIR": os.path.join(work, "no-such-git-dir")}
try:
    proc = subprocess.Popen(args, cwd=work, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=subprocess.DEVNULL, text=True)
    assert proc.stdin and proc.stdout
    proc.stdin.write(json.dumps(turn) + "\n")
    proc.stdin.flush()
    result = None
    with open(os.path.join(out, "stream.jsonl"), "w") as record:
        for line in proc.stdout:
            record.write(json.dumps({"t": round(time.time() - started, 1), "line": line.rstrip()}) + "\n")
            record.flush()
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            lines.append(event)
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
finally:
    if stub:
        stub.kill()

version = next((e.get("claude_code_version") for e in lines
                if e.get("type") == "system" and e.get("subtype") == "init"), None)
denied_lines = [e for e in lines if e.get("type") == "system" and e.get("subtype") == "permission_denied"]
error_results = [b for e in lines if e.get("type") == "user"
                 for b in (e.get("message", {}).get("content") or [])
                 if isinstance(b, dict) and b.get("type") == "tool_result" and b.get("is_error")]
print(json.dumps({
    "case": name,
    "elapsed": round(time.time() - started, 1),
    "claude_code_version": version,
    "system_permission_denied_lines": len(denied_lines),
    "error_tool_results": [str(b.get("content"))[:160] for b in error_results],
    "result_permission_denials": None if result is None else len(result.get("permission_denials") or []),
    "final_text": None if result is None else str(result.get("result"))[:200],
    "marker_written": os.path.exists(os.path.join(work, "spike-marker.txt")),
}, indent=1))
