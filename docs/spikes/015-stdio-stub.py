#!/usr/bin/env python3
"""A stdio MCP server with one tool, `approve`, used as Claude Code's
--permission-prompt-tool. It logs every call, waits WAIT_SECONDS, then
answers with ANSWER ("allow" or "deny"). No dependencies."""
import json, os, sys, time

LOG = os.environ.get("STUB_LOG", "stub.log")
WAIT = float(os.environ.get("WAIT_SECONDS", "0"))
ANSWER = os.environ.get("ANSWER", "allow")


def log(event, **fields):
    with open(LOG, "a") as f:
        f.write(json.dumps({"t": round(time.time(), 3), "event": event, **fields}) + "\n")


def send(msg):
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


TOOL = {
    "name": "approve",
    "description": "Answers a permission question for the session.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "tool_name": {"type": "string"},
            "input": {"type": "object"},
            "tool_use_id": {"type": "string"},
        },
        "required": ["tool_name", "input"],
    },
}

log("start", wait=WAIT, answer=ANSWER)
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        msg = json.loads(line)
    except json.JSONDecodeError:
        log("bad_json", line=line[:200])
        continue
    method, mid = msg.get("method"), msg.get("id")
    if method == "initialize":
        version = msg.get("params", {}).get("protocolVersion", "2025-06-18")
        send({"jsonrpc": "2.0", "id": mid, "result": {
            "protocolVersion": version,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "stub", "version": "0.0.1"},
        }})
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": mid, "result": {"tools": [TOOL]}})
    elif method == "tools/call":
        args = msg.get("params", {}).get("arguments", {})
        log("asked", tool_name=args.get("tool_name"), input=args.get("input"))
        time.sleep(WAIT)
        if ANSWER == "allow":
            body = {"behavior": "allow", "updatedInput": args.get("input", {})}
        else:
            body = {"behavior": "deny", "message": "A person said no to this command."}
        log("answered", after=WAIT, body=body)
        send({"jsonrpc": "2.0", "id": mid, "result": {
            "content": [{"type": "text", "text": json.dumps(body)}],
        }})
    elif method == "notifications/cancelled":
        log("cancelled", params=msg.get("params"))
    elif mid is not None:
        send({"jsonrpc": "2.0", "id": mid, "result": {}})
    else:
        log("notification", method=method)
log("stdin_closed")
