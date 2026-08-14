//! The real [`Fetch`] seam, and the bind probe that is deliberately not one.
//!
//! **The bind probe is not a ready-check and must not be confused with one.**
//! `fetch` answers "is the service up?"; the probe answers "is this port
//! taken?", and it does so with `bind()` rather than `connect()` — a
//! `connect()` answers the opposite question and reports a listening-but-idle
//! socket as free.
//!
//! **Measured, and it is the reason the probe binds twice:** an IPv6-only
//! listener is invisible to an IPv4 bind probe, and modern Node resolving
//! `localhost` to `::1` makes that the ordinary dev server rather than an
//! exotic case. So char binds **both** `127.0.0.1` and `[::1]` and treats
//! either `EADDRINUSE` as taken. `SO_REUSEPORT` on both sides remains
//! undetectable; that is a stated limit, not a bug (`docs/traps.md`).

use armada_core::ctx::Fetch;
use armada_core::error::{CharError, ErrClass};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

/// The production [`Fetch`].
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFetch;

impl Fetch for RealFetch {
    fn http_status(&self, url: &str, timeout: Duration) -> Result<u16, CharError> {
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
        // pull a body char has no use for into memory.
        let mut head = [0u8; 64];
        let read = stream
            .read(&mut head)
            .map_err(|e| environment(format!("{url}: {e}")))?;
        parse_status_line(&head[..read])
            .ok_or_else(|| environment(format!("{url}: no status line")))
    }

    fn tcp_connect(&self, host: &str, port: u16, timeout: Duration) -> Result<bool, CharError> {
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
/// Binding is the probe: char asks the kernel the same question the service
/// will ask, and gets the same answer. It costs one `bind()` per family and
/// releases both immediately.
pub fn port_is_taken(port: u16) -> bool {
    let v4 = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let v6 = SocketAddr::from((Ipv6Addr::LOCALHOST, port));
    // A bind that fails for any reason at all is "taken" as far as char is
    // concerned: it could not have the port either way, and reporting a port
    // it cannot use as free is the failure that matters.
    !bindable(v4) || !bindable(v6)
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

fn split_url(url: &str) -> Result<(String, u16, String), CharError> {
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

fn environment(message: String) -> CharError {
    CharError {
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
