#!/usr/bin/env python3
"""Four ways to turn a loopback peer port into a pid, on darwin.

Spike 10 used one — `lsof -nP -i TCP:<port>` — and measured 67ms. That number is
the whole reason this spike exists, so it is worth knowing whether it is the
price of the question or the price of `lsof`. Three cheaper readings are offered
here and all four are timed by `012-lookup-cost.py`:

  lsof_port     `lsof -nP -i TCP:<port>`; scans every process on the machine
  lsof_pids     `lsof -nP -p <known pids> -a -i TCP`; scans the pids Fleet holds
  netstat       one `netstat -anv -p tcp`, which prints a `process:pid` column
  libproc       `proc_pidfdinfo(pid, fd, PROC_PIDFDSOCKETINFO)` over the same
                known pids, through ctypes — no subprocess at all

**The last one inverts the question.** `lsof -i TCP:<port>` asks "who holds this
port", which nothing but a full scan can answer. Fleet already knows the pid of
every Drone it spawned, so it can instead ask "does any of these five hold it",
which is bounded by the fleet size rather than by the process table.

The offset of the local port inside `struct socket_fdinfo` is not hardcoded. It
is calibrated once against a socket whose port this process already knows, and
confirmed against a second one, so a header change is a failed calibration
rather than a wrong pid.
"""
import ctypes
import os
import socket
import struct
import subprocess

PROC_PIDLISTFDS = 1
PROC_PIDFDSOCKETINFO = 3
PROX_FDTYPE_SOCKET = 2
PROC_PIDLISTFD_SIZE = 8          # struct proc_fdinfo { int32 fd; uint32 type; }
SOCKETINFO_SIZE = 2048           # comfortably over sizeof(struct socket_fdinfo)

_libproc = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
_libproc.proc_pidinfo.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_uint64,
                                  ctypes.c_void_p, ctypes.c_int]
_libproc.proc_pidinfo.restype = ctypes.c_int
_libproc.proc_pidfdinfo.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_int,
                                    ctypes.c_void_p, ctypes.c_int]
_libproc.proc_pidfdinfo.restype = ctypes.c_int

# Set by calibrate(): where the local port sits in the socket_fdinfo blob, and
# where the peer port sits. Both are big-endian uint16 in the in_sockinfo.
LOCAL_PORT_OFFSET = None
PEER_PORT_OFFSET = None


def _fds(pid):
    """The socket fds of `pid`, or an empty list if the pid is gone or opaque."""
    size = _libproc.proc_pidinfo(pid, PROC_PIDLISTFDS, 0, None, 0)
    if size <= 0:
        return []
    buf = ctypes.create_string_buffer(size)
    got = _libproc.proc_pidinfo(pid, PROC_PIDLISTFDS, 0, buf, size)
    if got <= 0:
        return []
    out = []
    for i in range(got // PROC_PIDLISTFD_SIZE):
        fd, kind = struct.unpack_from("=ii", buf, i * PROC_PIDLISTFD_SIZE)
        if kind == PROX_FDTYPE_SOCKET:
            out.append(fd)
    return out


def _socket_blob(pid, fd):
    buf = ctypes.create_string_buffer(SOCKETINFO_SIZE)
    got = _libproc.proc_pidfdinfo(pid, fd, PROC_PIDFDSOCKETINFO, buf,
                                  SOCKETINFO_SIZE)
    if got <= 0:
        return None
    return buf.raw[:got]


def calibrate():
    """Find the local- and peer-port offsets against a socket we already know.

    A connected loopback pair in this very process gives two ports whose values
    are known before the blob is read, so the offset is the one that agrees with
    both. A second, independent pair confirms it — one accidental match is
    plausible in 2KB, two are not.
    """
    global LOCAL_PORT_OFFSET, PEER_PORT_OFFSET

    def candidates():
        srv = socket.socket()
        srv.bind(("127.0.0.1", 0))
        srv.listen(1)
        client = socket.socket()
        client.connect(srv.getsockname())
        conn, _ = srv.accept()
        want_local = client.getsockname()[1]
        want_peer = client.getpeername()[1]
        blob = _socket_blob(os.getpid(), client.fileno())
        local, peer = set(), set()
        if blob:
            for off in range(0, len(blob) - 1):
                value = struct.unpack_from(">H", blob, off)[0]
                if value == want_local:
                    local.add(off)
                if value == want_peer:
                    peer.add(off)
        for s in (client, conn, srv):
            s.close()
        return local, peer

    l1, p1 = candidates()
    l2, p2 = candidates()
    local, peer = sorted(l1 & l2), sorted(p1 & p2)
    if not local or not peer:
        raise SystemExit("could not calibrate socket_fdinfo port offsets")
    LOCAL_PORT_OFFSET, PEER_PORT_OFFSET = local[0], peer[0]
    return LOCAL_PORT_OFFSET, PEER_PORT_OFFSET


def libproc_owner(port, pids):
    """Which of `pids` holds a TCP socket with local port `port`.

    The bounded question: Fleet passes the pids it spawned. Returns None when
    none of them does, which is the honest answer for a connection whose process
    has already exited.
    """
    if LOCAL_PORT_OFFSET is None:
        calibrate()
    for pid in pids:
        for fd in _fds(pid):
            blob = _socket_blob(pid, fd)
            if blob is None or len(blob) < LOCAL_PORT_OFFSET + 2:
                continue
            if struct.unpack_from(">H", blob, LOCAL_PORT_OFFSET)[0] == port:
                return pid
    return None


def libproc_owner_4tuple(port, pids, remote=None):
    """The same scan, matching the connection rather than the local port.

    A local port number is not unique on a host: two processes can each hold
    port 24101 as long as they are talking to different places, and a lookup
    keyed on the number alone answers with whichever it met first. The pair
    (local port, remote port) is unique per address family, and both are in the
    same blob, so the stricter match costs nothing.
    """
    if LOCAL_PORT_OFFSET is None:
        calibrate()
    for pid in pids:
        for fd in _fds(pid):
            blob = _socket_blob(pid, fd)
            if blob is None or len(blob) < LOCAL_PORT_OFFSET + 2:
                continue
            if struct.unpack_from(">H", blob, LOCAL_PORT_OFFSET)[0] != port:
                continue
            if remote is not None and \
                    struct.unpack_from(">H", blob, PEER_PORT_OFFSET)[0] != remote:
                continue
            return pid
    return None


def lsof_port_owner(port, _pids=None):
    """Spike 10's route: ask the machine who holds the port."""
    out = subprocess.run(["lsof", "-nP", "-i", "TCP:%d" % port],
                         capture_output=True, text=True).stdout
    for line in out.splitlines()[1:]:
        if ("%d->" % port) in line:
            return int(line.split()[1])
    return None


def lsof_pids_owner(port, pids):
    """The same tool, bounded to the pids Fleet already holds."""
    if not pids:
        return None
    out = subprocess.run(
        ["lsof", "-nP", "-p", ",".join(str(p) for p in pids), "-a", "-i", "TCP"],
        capture_output=True, text=True).stdout
    for line in out.splitlines()[1:]:
        if ("%d->" % port) in line:
            return int(line.split()[1])
    return None


def netstat_owner(port, _pids=None):
    """One netstat, which on darwin prints a `process:pid` column under -v."""
    out = subprocess.run(["netstat", "-anv", "-p", "tcp"],
                         capture_output=True, text=True).stdout
    for line in out.splitlines():
        parts = line.split()
        if len(parts) < 12:
            continue
        if not parts[3].endswith(".%d" % port):
            continue
        for field in parts:
            if ":" in field and field.rsplit(":", 1)[-1].isdigit():
                return int(field.rsplit(":", 1)[-1])
    return None


ROUTES = {
    "lsof_port": lsof_port_owner,
    "lsof_pids": lsof_pids_owner,
    "netstat": netstat_owner,
    "libproc": libproc_owner,
}

if __name__ == "__main__":
    print("calibrated offsets: local=%d peer=%d" % calibrate())
