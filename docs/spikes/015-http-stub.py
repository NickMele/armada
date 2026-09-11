#!/usr/bin/env python3
"""A permission tool served over HTTP the way Armada's MCP server serves its
tools: one POST per JSON-RPC message, answered with application/json, no
stream. `tools/call` waits WAIT_SECONDS, then answers ANSWER (allow or deny).

usage: http_stub.py PORT WAIT_SECONDS ANSWER LOG
"""
import json, sys, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

port, wait, answer, log = int(sys.argv[1]), float(sys.argv[2]), sys.argv[3], sys.argv[4]
started = time.time()


def note(what):
    with open(log, "a") as f:
        f.write(json.dumps({"t": round(time.time() - started, 1), **what}) + "\n")


TOOL = {"name": "approve", "description": "Permission prompt tool.",
        "inputSchema": {"type": "object", "properties": {
            "tool_name": {"type": "string"}, "input": {"type": "object"},
            "tool_use_id": {"type": "string"}}}}


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        self.send_response(405)
        self.end_headers()

    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        try:
            msg = json.loads(body)
        except json.JSONDecodeError:
            self.send_response(400)
            self.end_headers()
            return
        method = msg.get("method", "")
        if method.startswith("notifications/") or "id" not in msg:
            self.send_response(202)
            self.end_headers()
            return
        if method == "initialize":
            result = {"protocolVersion": msg.get("params", {}).get("protocolVersion", "2025-06-18"),
                      "capabilities": {"tools": {}}, "serverInfo": {"name": "stub", "version": "0"}}
        elif method == "tools/list":
            result = {"tools": [TOOL]}
        elif method == "tools/call":
            args = msg.get("params", {}).get("arguments", {})
            note({"asked": args})
            time.sleep(wait)
            if answer == "error":
                note({"answered": "tool error"})
                result = {"content": [{"type": "text", "text": "Fleet could not answer."}], "isError": True}
            else:
                if answer == "allow":
                    decision = {"behavior": "allow", "updatedInput": args.get("input", {})}
                else:
                    decision = {"behavior": "deny", "message": "A person said no to this command."}
                note({"answered": decision["behavior"]})
                result = {"content": [{"type": "text", "text": json.dumps(decision)}], "isError": False}
        else:
            result = {}
        out = json.dumps({"jsonrpc": "2.0", "id": msg["id"], "result": result}).encode()
        try:
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(out)))
            self.end_headers()
            self.wfile.write(out)
            note({"replied": method})
        except (BrokenPipeError, ConnectionResetError) as gone:
            note({"client_gone_before_reply": method, "error": str(gone)})


ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
