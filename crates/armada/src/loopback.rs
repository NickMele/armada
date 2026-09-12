//! One request to Fleet, on loopback, written by hand.
//!
//! **Twenty lines of HTTP rather than the workspace's first client crate.** The
//! peer is Fleet's own listener answering two routes whose bodies both have a
//! length, which is what makes the framing a paragraph instead of a dependency.
//!
//! **The host is a constant and never a parameter**, as
//! `fleet::runtime::listener_address` is on the other side: there is no field
//! through which a routable address could arrive.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

/// How long to wait for Fleet to accept a connection.
///
/// Short, because the runtime file already said a live process holds this port:
/// a connect that does not complete quickly on loopback is a Fleet that is not
/// listening rather than a slow network.
const CONNECT_PATIENCE: Duration = Duration::from_secs(2);

/// Fleet, at a port read out of the runtime file.
pub struct Loopback {
    at: SocketAddr,
}

/// What Fleet answered: the status and the bytes, nothing interpreted. **This
/// module does not parse JSON** — it carries a body between two things that do.
#[derive(Debug)]
pub struct Answer {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Loopback {
    pub fn at(port: u16) -> Loopback {
        Loopback {
            at: SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
        }
    }

    pub fn port(&self) -> u16 {
        self.at.port()
    }

    pub fn get(&self, path: &str) -> Result<Answer, Unreachable> {
        self.sent("GET", path, None)
    }

    pub fn post(&self, path: &str, body: &[u8]) -> Result<Answer, Unreachable> {
        self.sent("POST", path, Some(body))
    }

    /// One request, one connection, closed by the server.
    ///
    /// `Connection: close` rather than a kept-alive pool: the response ends at
    /// end-of-stream, so there is no second framing rule to get wrong, and a
    /// connection per tool call on loopback costs microseconds against a call
    /// that is reading a database or running a Check.
    fn sent(&self, method: &str, path: &str, body: Option<&[u8]>) -> Result<Answer, Unreachable> {
        let mut socket =
            TcpStream::connect_timeout(&self.at, CONNECT_PATIENCE).map_err(|cause| {
                Unreachable::NotListening {
                    port: self.at.port(),
                    cause: cause.to_string(),
                }
            })?;
        // **No read timeout, deliberately.** A tool call here can be a Check
        // that runs for minutes, and a deadline would cut it off with no answer
        // — which reads as a broken server rather than as a long call.
        let mut head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\
             Accept: application/json\r\n",
            self.at
        );
        if let Some(body) = body {
            head.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            ));
        }
        head.push_str("\r\n");

        let mut raw = Vec::new();
        let sent = socket
            .write_all(head.as_bytes())
            .and_then(|()| socket.write_all(body.unwrap_or_default()))
            .and_then(|()| socket.flush())
            .and_then(|()| socket.read_to_end(&mut raw).map(|_| ()));
        sent.map_err(|cause| Unreachable::Cut {
            port: self.at.port(),
            cause: cause.to_string(),
        })?;
        answered(&raw, self.at.port())
    }
}

/// The status line and the body, out of what came back.
pub fn answered(raw: &[u8], port: u16) -> Result<Answer, Unreachable> {
    let Some(split) = raw.windows(4).position(|four| four == b"\r\n\r\n") else {
        return Err(Unreachable::Unreadable {
            port,
            why: String::from("the answer had no header"),
        });
    };
    let head = String::from_utf8_lossy(&raw[..split]);
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|status| status.split(' ').nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| Unreachable::Unreadable {
            port,
            why: String::from("the answer began with no status"),
        })?;

    let mut length: Option<usize> = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        if name == "content-length" {
            length = value.parse().ok();
        }
        // **Refused rather than half-read.** Both routes called here answer
        // with a body of known size, so a de-chunker nothing exercises would be
        // worse than a sentence saying this is not the Fleet it was written for.
        if name == "transfer-encoding" && value.to_ascii_lowercase().contains("chunked") {
            return Err(Unreachable::Unreadable {
                port,
                why: String::from("the answer was chunked, which this reader does not carry"),
            });
        }
    }

    let body = &raw[split + 4..];
    let body = match length {
        Some(length) if length <= body.len() => &body[..length],
        _ => body,
    };
    Ok(Answer {
        status,
        body: body.to_vec(),
    })
}

/// Why Fleet did not answer.
///
/// **Three, because they are three different things to tell somebody.** A port
/// nothing is listening on is a Fleet that stopped; a connection cut mid-answer
/// is one that died during the call; an answer that will not parse is a port
/// held by something that is not Fleet at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unreachable {
    NotListening { port: u16, cause: String },
    Cut { port: u16, cause: String },
    Unreadable { port: u16, why: String },
}

impl std::fmt::Display for Unreachable {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unreachable::NotListening { port, cause } => write!(
                out,
                "nothing answered on port {port}, where the runtime file says Fleet is \
                 listening: {cause}"
            ),
            Unreachable::Cut { port, cause } => write!(
                out,
                "the connection to Fleet on port {port} ended before the answer did: {cause}"
            ),
            Unreachable::Unreadable { port, why } => write!(
                out,
                "what answered on port {port} was not an answer Fleet would have sent: {why}"
            ),
        }
    }
}

impl std::error::Error for Unreachable {}
