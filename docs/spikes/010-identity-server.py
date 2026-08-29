#!/usr/bin/env python3
"""One MCP server, three transports, and a log of everything it can learn about
its caller without asking the caller.

    010-identity-server.py stdio
    010-identity-server.py tcp  <port>
    010-identity-server.py unix <socket path>

Every connection and every JSON-RPC message is appended to $IDENTITY_LOG, one
JSON object per line. What is recorded is only what the receiving side can
observe: on a unix socket the peer credentials, on TCP the peer port and what
`lsof` says holds it, on stdio the parent process. The `ps` line for whatever
pid comes back is recorded beside it, so "the CLI, or a child of it" is a
reading rather than an assumption.
"""
import json
import os
import socket
import struct
import subprocess
import sys
import time

LOG = os.environ.get("IDENTITY_LOG", "/tmp/identity.log")

# <sys/un.h> on darwin. SOL_LOCAL is 0; the option numbers are stable ABI.
SOL_LOCAL = 0
LOCAL_PEERCRED = 0x001
LOCAL_PEERPID = 0x002
LOCAL_PEEREPID = 0x003


def log(kind, payload):
    with open(LOG, "a") as f:
        f.write(json.dumps({"t": time.time(), "kind": kind, "payload": payload}) + "\n")


def ps_line(pid):
    if not pid:
        return None
    try:
        out = subprocess.run(
            ["ps", "-o", "pid=,ppid=,comm=", "-p", str(pid)],
            capture_output=True, text=True, timeout=5,
        ).stdout.strip()
        return out or None
    except Exception as why:
        return "ps failed: %s" % why


def peer_of_unix(conn):
    """What a unix socket tells the server about who connected."""
    seen = {}
    try:
        raw = conn.getsockopt(SOL_LOCAL, LOCAL_PEERCRED, 1024)
        # struct xucred { u_int cr_version; uid_t cr_uid; short cr_ngroups; gid_t cr_groups[16]; }
        version, uid, ngroups = struct.unpack_from("=IIh", raw)
        seen["peercred"] = {"version": version, "uid": uid, "ngroups": ngroups}
    except OSError as why:
        seen["peercred_error"] = str(why)
    for name, opt in (("peerpid", LOCAL_PEERPID), ("peerepid", LOCAL_PEEREPID)):
        try:
            seen[name] = struct.unpack("=i", conn.getsockopt(SOL_LOCAL, opt, 4))[0]
        except OSError as why:
            seen[name + "_error"] = str(why)
    pid = seen.get("peerpid")
    seen["ps"] = ps_line(pid)
    seen["ancestry"] = ancestry(pid)
    return seen


def peer_of_tcp(conn):
    """What a TCP socket tells the server: an address, a port, and nothing else.
    The pid is recovered out of band, through lsof, which a server would have to
    do on every call and which races a short-lived connection."""
    addr = conn.getpeername()
    seen = {"peer": addr}
    try:
        out = subprocess.run(
            ["lsof", "-nP", "-i", "TCP:%d" % addr[1]],
            capture_output=True, text=True, timeout=10,
        ).stdout.strip().splitlines()
        seen["lsof"] = out
        pid = None
        for line in out[1:]:
            parts = line.split()
            if len(parts) > 1 and ("%d->" % addr[1]) in line:
                pid = int(parts[1])
                break
        seen["pid"] = pid
        seen["ps"] = ps_line(pid)
        seen["ancestry"] = ancestry(pid)
    except Exception as why:
        seen["lsof_error"] = str(why)
    return seen


def ancestry(pid, limit=6):
    """pid, then its parents, so a hit on a child of the CLI is legible as one."""
    chain = []
    while pid and pid > 1 and len(chain) < limit:
        try:
            out = subprocess.run(
                ["ps", "-o", "ppid=,comm=", "-p", str(pid)],
                capture_output=True, text=True, timeout=5,
            ).stdout.strip()
            if not out:
                break
            ppid, comm = out.split(None, 1)
            chain.append({"pid": pid, "comm": comm})
            pid = int(ppid)
        except Exception:
            break
    return chain


TOOL = {
    "name": "whoami",
    "description": "Report which Job you believe you are working on. Call it once.",
    "inputSchema": {
        "type": "object",
        "properties": {"job": {"type": "string"}},
        "required": ["job"],
        "additionalProperties": False,
    },
}


def handle(req, transport, observed):
    method, rid = req.get("method"), req.get("id")
    # The peer is recorded once, on the connection line; a method line names the
    # connection it arrived on so the artifact stays readable.
    log("request", {"transport": transport, "method": method, "id": rid,
                    "connection": observed.get("id")})
    if method == "initialize":
        ver = (req.get("params") or {}).get("protocolVersion") or "2025-06-18"
        return {"jsonrpc": "2.0", "id": rid, "result": {
            "protocolVersion": ver,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "armada-identity-spike", "version": "0.1.0"},
        }}
    if method in ("notifications/initialized", "notifications/cancelled"):
        return None
    if method == "ping":
        return {"jsonrpc": "2.0", "id": rid, "result": {}}
    if method == "tools/list":
        return {"jsonrpc": "2.0", "id": rid, "result": {"tools": [TOOL]}}
    if method == "tools/call":
        params = req.get("params") or {}
        log("tool_call", {"transport": transport, "name": params.get("name"),
                          "arguments": params.get("arguments"),
                          "connection": observed.get("id"), "observed": observed})
        return {"jsonrpc": "2.0", "id": rid, "result": {
            "content": [{"type": "text", "text": "seen"}], "isError": False}}
    if rid is not None:
        return {"jsonrpc": "2.0", "id": rid,
                "error": {"code": -32601, "message": "method not found: %s" % method}}
    return None


def serve_stdio():
    observed = {"id": 1, "ppid": os.getppid(), "ps": ps_line(os.getppid()),
                "ancestry": ancestry(os.getppid())}
    log("connection", {"transport": "stdio", "observed": observed,
                       "argv": sys.argv, "cwd": os.getcwd(),
                       "env_keys": sorted(os.environ)})
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except Exception as why:
            log("parse_error", {"line": line[:400], "error": str(why)})
            continue
        reply = handle(req, "stdio", observed)
        if reply is not None:
            sys.stdout.write(json.dumps(reply) + "\n")
            sys.stdout.flush()
    log("eof", {"transport": "stdio"})


def read_http_request(f):
    start = f.readline()
    if not start:
        return None
    request_line = start.decode("latin1").strip()
    headers = {}
    while True:
        raw = f.readline()
        if not raw or raw in (b"\r\n", b"\n"):
            break
        k, _, v = raw.decode("latin1").partition(":")
        headers[k.strip().lower()] = v.strip()
    body = b""
    n = int(headers.get("content-length") or 0)
    if n:
        body = f.read(n)
    return request_line, headers, body


def serve_http(sock, transport, peer_reader):
    """One thread per connection: two Drones connect at once, and a server that
    served them in turn would measure its own serialisation rather than theirs."""
    import itertools
    import threading
    counter = itertools.count(1)
    sock.listen(16)
    while True:
        conn, _ = sock.accept()
        observed = peer_reader(conn)
        observed["id"] = next(counter)
        log("connection", {"transport": transport, "observed": observed})
        threading.Thread(target=serve_connection,
                         args=(conn, transport, observed), daemon=True).start()


# Sessions of the legacy HTTP+SSE transport: the reply to a POST goes out on a
# GET stream opened earlier, so the stream has to be findable from another
# connection. Keyed by the session id the server itself minted.
SSE_STREAMS = {}
SSE_LOCK = __import__("threading").Lock()
WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"


def websocket_handshake(f, headers):
    import base64
    import hashlib
    key = headers.get("sec-websocket-key", "")
    accept = base64.b64encode(
        hashlib.sha1((key + WS_GUID).encode()).digest()).decode()
    f.write(("HTTP/1.1 101 Switching Protocols\r\n"
             "Upgrade: websocket\r\nConnection: Upgrade\r\n"
             "Sec-WebSocket-Accept: %s\r\n"
             "Sec-WebSocket-Protocol: mcp\r\n\r\n" % accept).encode())
    f.flush()


def ws_read(f):
    head = f.read(2)
    if len(head) < 2:
        return None
    opcode = head[0] & 0x0F
    masked = head[1] & 0x80
    length = head[1] & 0x7F
    if length == 126:
        length = struct.unpack(">H", f.read(2))[0]
    elif length == 127:
        length = struct.unpack(">Q", f.read(8))[0]
    mask = f.read(4) if masked else b""
    data = f.read(length)
    if masked:
        data = bytes(b ^ mask[i % 4] for i, b in enumerate(data))
    if opcode == 0x8:
        return None
    return data.decode("utf-8", "replace")


def ws_write(f, text):
    payload = text.encode("utf-8")
    header = bytes([0x81])
    n = len(payload)
    if n < 126:
        header += bytes([n])
    elif n < 65536:
        header += bytes([126]) + struct.pack(">H", n)
    else:
        header += bytes([127]) + struct.pack(">Q", n)
    f.write(header + payload)
    f.flush()


def serve_websocket(f, transport, observed):
    log("upgraded", {"transport": transport, "to": "websocket",
                     "connection": observed.get("id")})
    while True:
        text = ws_read(f)
        if text is None:
            break
        for chunk in text.splitlines():
            chunk = chunk.strip()
            if not chunk:
                continue
            req = json.loads(chunk)
            reply = handle(req, transport, observed)
            if reply is not None:
                ws_write(f, json.dumps(reply))


def serve_sse_stream(f, transport, observed):
    session = "s%d" % observed.get("id", 0)
    f.write(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n"
            b"Cache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n")
    f.write(("event: endpoint\ndata: /messages?sessionId=%s\n\n" % session).encode())
    f.flush()
    with SSE_LOCK:
        SSE_STREAMS[session] = f
    log("upgraded", {"transport": transport, "to": "sse", "session": session,
                     "connection": observed.get("id")})
    # Hold the stream open; the POSTs arrive on other connections.
    while True:
        time.sleep(1)
        try:
            f.write(b": ping\n\n")
            f.flush()
        except OSError:
            break


def serve_connection(conn, transport, observed):
    f = conn.makefile("rwb")
    try:
        while True:
            got = read_http_request(f)
            if got is None:
                break
            request_line, headers, body = got
            log("http", {"transport": transport, "request_line": request_line,
                         "headers": headers, "connection": observed.get("id")})
            if headers.get("upgrade", "").lower() == "websocket":
                websocket_handshake(f, headers)
                serve_websocket(f, transport, observed)
                break
            if (request_line.startswith("GET /sse")
                    and "event-stream" in headers.get("accept", "")):
                serve_sse_stream(f, transport, observed)
                break
            sse_session = None
            if request_line.startswith("POST /messages"):
                sse_session = request_line.split("sessionId=")[-1].split()[0]
            replies = []
            if body:
                for chunk in body.decode("utf-8").splitlines():
                    chunk = chunk.strip()
                    if not chunk:
                        continue
                    try:
                        req = json.loads(chunk)
                    except Exception as why:
                        log("parse_error", {"line": chunk[:400], "error": str(why)})
                        continue
                    if isinstance(req, list):
                        for one in req:
                            r = handle(one, transport, observed)
                            if r is not None:
                                replies.append(r)
                    else:
                        r = handle(req, transport, observed)
                        if r is not None:
                            replies.append(r)
            if sse_session is not None:
                # Legacy HTTP+SSE: the POST is acknowledged and the reply is
                # written to the stream the client opened with its GET.
                f.write(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n"
                        b"Connection: keep-alive\r\n\r\n")
                f.flush()
                with SSE_LOCK:
                    stream = SSE_STREAMS.get(sse_session)
                for reply in replies:
                    if stream is not None:
                        stream.write(("event: message\ndata: %s\n\n"
                                      % json.dumps(reply)).encode())
                        stream.flush()
                continue
            if replies:
                payload = json.dumps(replies[0] if len(replies) == 1 else replies)
                out = payload.encode("utf-8")
                f.write(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n"
                        b"Content-Length: %d\r\nConnection: keep-alive\r\n\r\n%s"
                        % (len(out), out))
            else:
                f.write(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n"
                        b"Connection: keep-alive\r\n\r\n")
            f.flush()
    except Exception as why:
        log("connection_error", {"transport": transport, "error": str(why)})
    finally:
        try:
            conn.close()
        except Exception:
            pass


def main():
    mode = sys.argv[1]
    if mode == "stdio":
        serve_stdio()
    elif mode == "tcp":
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        s.bind(("127.0.0.1", int(sys.argv[2])))
        serve_http(s, "tcp", peer_of_tcp)
    elif mode == "unix":
        path = sys.argv[2]
        if os.path.exists(path):
            os.unlink(path)
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.bind(path)
        os.chmod(path, 0o600)
        serve_http(s, "unix", peer_of_unix)
    else:
        raise SystemExit("unknown mode %r" % mode)


main()
