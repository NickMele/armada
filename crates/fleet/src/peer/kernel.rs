//! Asking the kernel which connections a process holds.
//!
//! # Transcribed from `sys/proc_info.h`, and checked by the compiler
//!
//! `libc` carries `proc_pidinfo`, `proc_pidfdinfo`, `proc_fdinfo` and the two
//! constants, and it does **not** carry `socket_fdinfo` — so the record's shape
//! is written out here. Every struct below is `#[repr(C)]` and is followed by a
//! `const` assertion on its size, so a field transcribed wrongly is a build
//! failure rather than an offset that reads four bytes of somebody else's port.
//!
//! `docs/spikes/012` recovered these offsets at runtime because it was Python.
//! This is not, and a layout the compiler agrees with is a better guarantee than
//! a calibration that has to be right every time it runs.
//!
//! # Only as far as the ports, and the buffer is still whole
//!
//! `soi_proto` is a union of seven protocol records and only two of them are
//! read here — `SOCKINFO_IN` and `SOCKINFO_TCP`, which begin with the same
//! `in_sockinfo` and therefore with the same two ports. The rest of the union is
//! not transcribed, because nothing reads it and a wrong size in a part nobody
//! reads is a wrong size all the same.
//!
//! What the kernel is given is a **byte buffer larger than any of them**, since
//! `proc_pidfdinfo` refuses a buffer smaller than the flavour's own size and
//! copies out that size regardless of how much more is there.
//!
//! # A failure is silence
//!
//! Every call here answers `false` on any error. `crate::peer` says why that is
//! the only direction this may fail in: a caller Fleet cannot place is refused,
//! and a caller it places wrongly is one Job's work credited to another.

use std::mem::size_of;

use libc::{c_int, c_void, proc_fdinfo, proc_pidfdinfo, proc_pidinfo};

use super::PeerOf;

/// `PROC_PIDLISTFDS`, from `sys/proc_info.h` by way of `libc`.
const LIST_FDS: c_int = libc::PROC_PIDLISTFDS;
/// `PROX_FDTYPE_SOCKET`.
const FD_SOCKET: u32 = libc::PROX_FDTYPE_SOCKET as u32;
/// `PROC_PIDFDSOCKETINFO`. Not in `libc`; the header's value.
const FD_SOCKET_INFO: c_int = 3;
/// `SOCKINFO_IN` and `SOCKINFO_TCP` — the two kinds whose protocol record
/// begins with an `in_sockinfo`, which is where the two ports are.
const SOCKINFO_IN: i32 = 1;
const SOCKINFO_TCP: i32 = 2;

/// Bigger than `sizeof(struct socket_fdinfo)` on every darwin this runs on. The
/// call refuses a buffer that is too small and ignores one that is too large,
/// so the only failure this size can have is the one direction that errors.
const BUFFER: usize = 2048;

/// How many open files one process may be asked about in one pass. A process
/// with more is walked in one allocation regardless — the list call answers how
/// many bytes it wanted, and this is only the first guess.
const FDS: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
// Every field is transcribed and only three are read. The rest are here to
// place the three, and removing one would move them.
#[allow(dead_code)]
struct ProcFileInfo {
    fi_openflags: u32,
    fi_status: u32,
    fi_offset: i64,
    fi_type: i32,
    fi_guardflags: u32,
}
const _: () = assert!(size_of::<ProcFileInfo>() == 24);

/// `struct vinfo_stat` — a copy of `stat64` with static sized fields. Nothing
/// here reads a field of it; it is transcribed because `socket_info` opens with
/// one and everything after it is at its far side.
#[repr(C)]
#[derive(Clone, Copy)]
// Every field is transcribed and only three are read. The rest are here to
// place the three, and removing one would move them.
#[allow(dead_code)]
struct VinfoStat {
    vst_dev: u32,
    vst_mode: u16,
    vst_nlink: u16,
    vst_ino: u64,
    vst_uid: u32,
    vst_gid: u32,
    vst_atime: i64,
    vst_atimensec: i64,
    vst_mtime: i64,
    vst_mtimensec: i64,
    vst_ctime: i64,
    vst_ctimensec: i64,
    vst_birthtime: i64,
    vst_birthtimensec: i64,
    vst_size: i64,
    vst_blocks: i64,
    vst_blksize: i32,
    vst_flags: u32,
    vst_gen: u32,
    vst_rdev: u32,
    vst_qspare: [i64; 2],
}
const _: () = assert!(size_of::<VinfoStat>() == 136);

#[repr(C)]
#[derive(Clone, Copy)]
// Every field is transcribed and only three are read. The rest are here to
// place the three, and removing one would move them.
#[allow(dead_code)]
struct SockbufInfo {
    sbi_cc: u32,
    sbi_hiwat: u32,
    sbi_mbcnt: u32,
    sbi_mbmax: u32,
    sbi_lowat: u32,
    sbi_flags: i16,
    sbi_timeo: i16,
}
const _: () = assert!(size_of::<SockbufInfo>() == 24);

/// `struct socket_fdinfo`, as far as the two ports and no further.
///
/// The union that follows `rfu_1` in the header is `soi_proto`; its first two
/// members both begin with `struct in_sockinfo`, whose first two fields are the
/// foreign port and the local port. **In that order** — the header puts
/// `insi_fport` first, and reading them the other way round is exactly the
/// mistake this whole module exists to not make.
#[repr(C)]
#[derive(Clone, Copy)]
// Every field is transcribed and only three are read. The rest are here to
// place the three, and removing one would move them.
#[allow(dead_code)]
struct SocketPorts {
    pfi: ProcFileInfo,
    soi_stat: VinfoStat,
    soi_so: u64,
    soi_pcb: u64,
    soi_type: i32,
    soi_protocol: i32,
    soi_family: i32,
    soi_options: i16,
    soi_linger: i16,
    soi_state: i16,
    soi_qlen: i16,
    soi_incqlen: i16,
    soi_qlimit: i16,
    soi_timeo: i16,
    soi_error: u16,
    soi_oobmark: u32,
    soi_rcv: SockbufInfo,
    soi_snd: SockbufInfo,
    soi_kind: i32,
    rfu_1: u32,
    /// `in_sockinfo::insi_fport` — the port at the far end, which for a Drone's
    /// connection to Fleet is Fleet's own listening port.
    insi_fport: i32,
    /// `in_sockinfo::insi_lport` — the port this process opened from, which is
    /// what the listener saw as its peer.
    insi_lport: i32,
}
const _: () = assert!(size_of::<SocketPorts>() == 272);
const _: () = assert!(size_of::<SocketPorts>() <= BUFFER);

/// The real answer: `proc_pidfdinfo` over the sockets one process holds.
#[derive(Debug, Default)]
pub struct Kernel;

impl PeerOf for Kernel {
    fn holds(&self, pid: u32, from: u16, to: u16) -> bool {
        sockets_of(pid)
            .into_iter()
            .filter_map(|fd| ports_of(pid, fd))
            .any(|(lport, fport)| lport == from && fport == to)
    }
}

/// Every socket file descriptor the process holds. Empty on any failure,
/// including a process that has gone.
fn sockets_of(pid: u32) -> Vec<c_int> {
    let mut entries: Vec<proc_fdinfo> = vec![
        proc_fdinfo {
            proc_fd: 0,
            proc_fdtype: 0,
        };
        FDS
    ];
    // Asked once with a generous buffer rather than twice with a sizing call:
    // the answer is how many bytes were written, so a process holding more than
    // `FDS` open files is truncated rather than mis-parsed, and a Drone's
    // session connection is not the two-hundred-and-fifty-seventh thing the CLI
    // opened.
    #[allow(unsafe_code)]
    let bytes = unsafe {
        proc_pidinfo(
            pid as c_int,
            LIST_FDS,
            0,
            entries.as_mut_ptr().cast::<c_void>(),
            (size_of::<proc_fdinfo>() * FDS) as c_int,
        )
    };
    if bytes <= 0 {
        return Vec::new();
    }
    let held = (bytes as usize) / size_of::<proc_fdinfo>();
    entries
        .into_iter()
        .take(held.min(FDS))
        .filter(|entry| entry.proc_fdtype == FD_SOCKET)
        .map(|entry| entry.proc_fd)
        .collect()
}

/// The local and foreign ports of one socket, where it is an IP or TCP socket.
///
/// `None` for a unix socket, a kernel control, a descriptor that closed between
/// the list and this call, or any error at all.
fn ports_of(pid: u32, fd: c_int) -> Option<(u16, u16)> {
    let mut buffer = [0u8; BUFFER];
    #[allow(unsafe_code)]
    let bytes = unsafe {
        proc_pidfdinfo(
            pid as c_int,
            fd,
            FD_SOCKET_INFO,
            buffer.as_mut_ptr().cast::<c_void>(),
            BUFFER as c_int,
        )
    };
    if (bytes as usize) < size_of::<SocketPorts>() {
        return None;
    }
    // `read_unaligned` because the buffer is a byte array: the layout is the
    // C one and the compiler has agreed its size above, but the array itself
    // carries no alignment promise.
    #[allow(unsafe_code)]
    let info = unsafe { buffer.as_ptr().cast::<SocketPorts>().read_unaligned() };
    if info.soi_kind != SOCKINFO_IN && info.soi_kind != SOCKINFO_TCP {
        return None;
    }
    // The kernel keeps these in network byte order in a host-sized field. Both
    // are read the same way, because reading one of them differently is how a
    // pair match becomes a coincidence.
    Some((
        u16::from_be(info.insi_lport as u16),
        u16::from_be(info.insi_fport as u16),
    ))
}
