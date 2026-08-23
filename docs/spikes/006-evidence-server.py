#!/usr/bin/env python3
"""A trivial MCP stdio server exposing exactly one submission tool.

Every request and every tool call is appended to $EVIDENCE_LOG as one JSON
object per line, so a run can be judged on what the server saw rather than on
what the transcript appears to say.
"""
import json, os, sys, time

LOG = os.environ.get("EVIDENCE_LOG", "/tmp/evidence.log")

def log(kind, payload):
    with open(LOG, "a") as f:
        f.write(json.dumps({"t": time.time(), "kind": kind, "payload": payload}) + "\n")

TOOL = {
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
                "serverInfo": {"name": "armada-evidence-spike", "version": "0.1.0"},
            }})
        elif method in ("notifications/initialized", "notifications/cancelled"):
            pass
        elif method == "ping":
            send({"jsonrpc": "2.0", "id": rid, "result": {}})
        elif method == "tools/list":
            send({"jsonrpc": "2.0", "id": rid, "result": {"tools": [TOOL]}})
        elif method == "tools/call":
            params = req.get("params") or {}
            log("tool_call", {"name": params.get("name"), "arguments": params.get("arguments")})
            send({"jsonrpc": "2.0", "id": rid, "result": {
                "content": [{"type": "text", "text": "recorded"}],
                "isError": False,
            }})
        elif rid is not None:
            send({"jsonrpc": "2.0", "id": rid,
                  "error": {"code": -32601, "message": f"method not found: {method}"}})
    log("eof", {})

main()
