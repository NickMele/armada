#!/usr/bin/env python3
"""Two MCP stdio tools: submit_evidence and escape_hatch.

Every request and every tool call is appended to $SPIKE_LOG as one JSON object
per line, so a run is judged on what the server saw rather than on what the
transcript appears to say. `submit_evidence` is spike 6's tool, description
unchanged, so the two spikes measure the same pipe.
"""
import json, os, sys, time

LOG = os.environ.get("SPIKE_LOG", "/tmp/spike7.log")

def log(kind, payload):
    with open(LOG, "a") as f:
        f.write(json.dumps({"t": time.time(), "kind": kind, "payload": payload}) + "\n")

SUBMIT = {
    "name": "submit_evidence",
    "description": (
        "Report the outcome of the step you were given. This is the only way to report: "
        "the result is not read from anything you write in prose. Returns a receipt, "
        "not a verdict — the receipt does not mean the step passed."
    ),
    "inputSchema": {
        "type": "object",
        "properties": {
            "summary": {"type": "string", "description": "What you did, in one or two sentences."},
            "check_command": {"type": "string", "description": "The command you ran to verify it."},
            "exit_code": {"type": "integer", "description": "That command's exit code."},
        },
        "required": ["summary", "check_command", "exit_code"],
        "additionalProperties": False,
    },
}

HATCH = {
    "name": "escape_hatch",
    "description": (
        "Hand this task to a person and end autonomous work on it. The three fields below "
        "are the only account of your side of it that reaches them."
    ),
    "inputSchema": {
        "type": "object",
        "properties": {
            "trying_to": {"type": "string", "description": "What this task was meant to produce."},
            "blocked_by": {"type": "string", "description": "The specific thing preventing it."},
            "tried": {"type": "string", "description": "What you attempted, and what each attempt produced."},
        },
        "required": ["trying_to", "blocked_by", "tried"],
        "additionalProperties": False,
    },
}

def send(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()

def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except Exception as e:
            log("parse_error", {"line": line[:400], "error": str(e)})
            continue
        method, rid = req.get("method"), req.get("id")
        log("request", {"method": method, "id": rid})
        if method == "initialize":
            ver = (req.get("params") or {}).get("protocolVersion") or "2025-06-18"
            send({"jsonrpc": "2.0", "id": rid, "result": {
                "protocolVersion": ver,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "armada-hatch-spike", "version": "0.1.0"},
            }})
        elif method in ("notifications/initialized", "notifications/cancelled"):
            pass
        elif method == "ping":
            send({"jsonrpc": "2.0", "id": rid, "result": {}})
        elif method == "tools/list":
            send({"jsonrpc": "2.0", "id": rid, "result": {"tools": [SUBMIT, HATCH]}})
        elif method == "tools/call":
            params = req.get("params") or {}
            name = params.get("name")
            log("tool_call", {"name": name, "arguments": params.get("arguments")})
            text = "recorded" if name == "submit_evidence" else "handed off"
            send({"jsonrpc": "2.0", "id": rid, "result": {
                "content": [{"type": "text", "text": text}],
                "isError": False,
            }})
        elif rid is not None:
            send({"jsonrpc": "2.0", "id": rid,
                  "error": {"code": -32601, "message": f"method not found: {method}"}})
    log("eof", {})

main()
