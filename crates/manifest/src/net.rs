//! The real [`Fetch`] seam, and the bind probe that is deliberately not one.
//!
//! **The port probe is not a ready-check and must not be confused with one.**
//! `fetch` answers "is *this service* up?" against a URL or a port the config
//! named; the probe answers "is this port held by anything at all?", which is a
//! question about the machine.
//!
//! **It asks twice, on both families, in two ways**, and every one of those is
//! a measured blind spot rather than belt and braces:
//!
//! | Holder | Seen by |
//! |---|---|
//! | a listener on `127.0.0.1` | the bind |
//! | a listener on `::1` only — modern Node resolving `localhost` | the bind, on the second family |
//! | **a wildcard listener, which is every published container** | **the connect** |
//! | a socket bound but never `listen()`ed | the bind |
//! | `SO_REUSEPORT` on both sides | neither — a stated limit, not a bug |
//!
//! The third row is the one that cost something: `SO_REUSEADDR`, which Rust
//! sets on every `TcpListener::bind`, lets a specific-address bind succeed
//! while a wildcard holder exists, so the bind alone reported every container
//! Armada had ever published as free (`docs/traps.md`).

use armada_core::ctx::Fetch;
use armada_core::error::{ArmadaError, ErrClass};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

/// The production [`Fetch`].
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFetch;

impl Fetch for RealFetch {
    fn http_status(&self, url: &str, timeout: Duration) -> Result<u16, ArmadaError> {
        let (host, port, path) = split_url(url)?;
        let address = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(|e| environment(format!("cannot resolve {host}: {e}")))?
            .next()
            .ok_or_else(|| environment(format!("cannot resolve {host}")))?;

        let mut stream = TcpStream::connect_timeout(&address, timeout)
            .map_err(|e| environment(format!("{url}: {e}")))?;
        stream
            .set_read_timeout(Some(timeout))
            .and_then(|()| stream.set_write_timeout(Some(timeout)))
            .map_err(|e| environment(format!("{url}: {e}")))?;

        let request = format!(
            "GET {path} HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\nAccept: */*\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|e| environment(format!("{url}: {e}")))?;

        // The status line is the first 16 bytes at most; reading further would
        // pull a body Armada has no use for into memory.
        let mut head = [0u8; 64];
        let read = stream
            .read(&mut head)
            .map_err(|e| environment(format!("{url}: {e}")))?;
        parse_status_line(&head[..read])
            .ok_or_else(|| environment(format!("{url}: no status line")))
    }

    fn tcp_connect(&self, host: &str, port: u16, timeout: Duration) -> Result<bool, ArmadaError> {
        let addresses = (host, port)
            .to_socket_addrs()
            .map_err(|e| environment(format!("cannot resolve {host}: {e}")))?;
        for address in addresses {
            if TcpStream::connect_timeout(&address, timeout).is_ok() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Whether anything holds this port, on either family.
///
/// **Two probes, because one of them cannot see the holder that matters most.**
/// Binding asks the kernel the question a service is about to ask; connecting
/// asks whether anything is accepting. Either answering yes is "taken".
///
/// # Why the bind alone is not enough
///
/// Measured on darwin 2026-08-14 against Docker 29.6.2, and it corrects an
/// entry in `docs/traps.md` that said the opposite. Docker publishes a port by
/// binding the **wildcard** — `0.0.0.0:5460` and `[::]:5460` — and Rust's
/// `TcpListener::bind` sets `SO_REUSEADDR` on Unix, which under BSD semantics
/// permits binding a *specific* address while a wildcard holder exists:
///
/// ```text
/// holder: docker publishing 0.0.0.0:5460 and [::]:5460
///   bind 127.0.0.1:5460 without SO_REUSEADDR  -> EADDRINUSE   (taken)
///   bind 127.0.0.1:5460 with    SO_REUSEADDR  -> SUCCEEDS     (reads as FREE)
/// ```
///
/// So the bind probe reported **every container Armada has ever published** as
/// free. `armada manifest up` reported `RESERVED` for a healthy compose
/// service, `armada manifest status` rendered it `DOWN` while it was serving
/// traffic, and `init`'s `CONFLICT` detection could not see a container at all
/// — which is the one holder this project exists to manage.
///
/// The standard library gives no way to unset `SO_REUSEADDR`, and a fourth
/// `unsafe` call is a change to a design invariant (`ARCHITECTURE.md` §1). A
/// `connect()` needs neither and sees exactly the case the bind misses.
///
/// **`PLAN.md` §3.1's objection to `connect()` does not hold**, and it is worth
/// naming because it is why the connect was left out: it says a connect
/// *"reports a listening-but-idle socket as free"*. Measured, it does not — a
/// `connect()` to any listening socket completes whether or not the listener
/// ever reads. What a connect genuinely cannot see is a socket bound without
/// `listen()`, which the bind probe does see; the two are complementary, which
/// is why both are asked.
pub fn port_is_taken(port: u16) -> bool {
    let v4 = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let v6 = SocketAddr::from((Ipv6Addr::LOCALHOST, port));
    // A bind that fails for any reason at all is "taken" as far as Armada is
    // concerned: it could not have the port either way, and reporting a port
    // it cannot use as free is the failure that matters.
    if !bindable(v4) || !bindable(v6) {
        return true;
    }
    // The wildcard case. Only reached when the bind said free, so the ordinary
    // free port pays one `connect()` to a closed loopback port, which is an
    // immediate `ECONNREFUSED` rather than a wait.
    accepting(v4) || accepting(v6)
}

/// Whether anything completes a connection here.
const PROBE_CONNECT: Duration = Duration::from_millis(250);

/// Whether something is listening, as opposed to merely bound.
///
/// A short deadline rather than the default: a port a firewall blackholes would
/// otherwise stall a `status` that is meant to be cheap enough to poll, and a
/// loopback connection that has not completed in 250 ms is not a local service.
fn accepting(address: SocketAddr) -> bool {
    TcpStream::connect_timeout(&address, PROBE_CONNECT).is_ok()
}

fn bindable(address: SocketAddr) -> bool {
    match TcpListener::bind(address) {
        Ok(listener) => {
            drop(listener);
            true
        }
        Err(e) => {
            // A machine with IPv6 disabled answers `EAFNOSUPPORT` for every
            // `[::1]` bind. Treating that as "taken" would make every port on
            // the machine look busy.
            matches!(
                e.kind(),
                std::io::ErrorKind::AddrNotAvailable | std::io::ErrorKind::Unsupported
            )
        }
    }
}

fn parse_status_line(bytes: &[u8]) -> Option<u16> {
    let text = String::from_utf8_lossy(bytes);
    let line = text.lines().next()?;
    line.split_whitespace().nth(1)?.parse().ok()
}

fn split_url(url: &str) -> Result<(String, u16, String), ArmadaError> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| environment(format!("{url}: only http:// URLs are supported")))?;
    let (authority, path) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host.to_string(),
            port.parse()
                .map_err(|_| environment(format!("{url}: {port} is not a port")))?,
        ),
        None => (authority.to_string(), 80u16),
    };
    Ok((host, port, path.to_string()))
}

fn environment(message: String) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: "network".to_string(),
        message,
        next_action: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_port_probes_free() {
        // Ask the kernel for one, then let it go: whatever it hands out is
        // free by construction at the moment it is released.
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        assert!(!port_is_taken(port));
    }

    #[test]
    fn a_bound_port_probes_taken() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port_is_taken(port));
        drop(listener);
    }

    /// **The blind spot that made every published container read as free.**
    ///
    /// Docker publishes by binding the *wildcard*, and Rust's
    /// `TcpListener::bind` sets `SO_REUSEADDR`, which under BSD semantics
    /// permits binding a specific address while a wildcard holder exists — so
    /// the bind half of the probe answers FREE for a port that is serving
    /// traffic. This reproduces the holder without needing a daemon: what the
    /// kernel reacts to is the *shape* of the holder, not the fact that Docker
    /// made it.
    ///
    /// The second assertion is what makes the first mean something. Without it
    /// this test would pass against the old code on any platform where the
    /// kernel happened to refuse the bind for an unrelated reason, and the
    /// regression it guards would walk straight back in.
    #[test]
    fn a_wildcard_holder_is_seen_even_though_the_bind_probe_cannot_see_it() {
        let holder = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).expect("a wildcard listener");
        let port = holder.local_addr().unwrap().port();

        assert!(
            port_is_taken(port),
            "a wildcard holder on {port} read as free — every published \
             container is invisible to `status` and to `init`'s CONFLICT check"
        );
        assert!(
            bindable(SocketAddr::from((Ipv4Addr::LOCALHOST, port))),
            "SO_REUSEADDR no longer defeats a specific-address bind here, so \
             this platform does not have the behaviour the connect probe exists \
             for — re-measure before trusting either half"
        );
        drop(holder);
    }

    /// A socket that is listening and never reads is **taken**, which is the
    /// claim PLAN.md §3.1 got backwards when it ruled `connect()` out for
    /// "reporting a listening-but-idle socket as free".
    #[test]
    fn a_listening_but_idle_socket_is_not_reported_free() {
        let idle = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
        let port = idle.local_addr().unwrap().port();
        // Nothing ever calls `accept`, which is the "idle" in the claim.
        assert!(accepting(SocketAddr::from((Ipv4Addr::LOCALHOST, port))));
        drop(idle);
    }

    /// The measured blind spot the two-family probe exists to close: a
    /// listener on `::1` only is invisible to an IPv4 bind, and `localhost`
    /// resolving to `::1` is what modern Node does.
    #[test]
    fn an_ipv6_only_listener_is_still_seen() {
        let Ok(listener) = TcpListener::bind((Ipv6Addr::LOCALHOST, 0)) else {
            // A machine with IPv6 disabled cannot host the case at all.
            return;
        };
        let port = listener.local_addr().unwrap().port();
        assert!(
            TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok(),
            "the premise: an IPv4 bind succeeds against an IPv6-only listener"
        );
        assert!(
            port_is_taken(port),
            "and the two-family probe still sees it"
        );
        drop(listener);
    }

    #[test]
    fn a_status_line_parses_and_junk_does_not() {
        assert_eq!(parse_status_line(b"HTTP/1.1 204 No Content\r\n"), Some(204));
        assert_eq!(parse_status_line(b"garbage"), None);
    }

    #[test]
    fn urls_split_into_host_port_and_path() {
        assert_eq!(
            split_url("http://127.0.0.1:8080/healthz").unwrap(),
            ("127.0.0.1".to_string(), 8080, "/healthz".to_string())
        );
        assert_eq!(
            split_url("http://example.test").unwrap(),
            ("example.test".to_string(), 80, "/".to_string())
        );
        assert!(split_url("https://example.test").is_err());
    }

    #[test]
    fn tcp_connect_reports_a_listener_and_the_absence_of_one() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(RealFetch
            .tcp_connect("127.0.0.1", port, Duration::from_millis(500))
            .unwrap());
        drop(listener);
    }
}
