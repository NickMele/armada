//! The half of HTTP this binary writes by hand, read back.
//!
//! **The framing is the risk, not the request.** A body taken one byte short is
//! a JSON-RPC message an agent's client cannot parse, and it would look like
//! Fleet had answered nonsense rather than like this reader had miscounted.

use crate::loopback::{answered, Unreachable};

fn raw(head: &str, body: &str) -> Vec<u8> {
    format!("{head}\r\n\r\n{body}").into_bytes()
}

#[test]
fn the_status_and_the_body_come_back_whole() {
    let answer = answered(
        &raw(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13",
            r#"{"jsonrpc":2}"#,
        ),
        4180,
    )
    .expect("an answer");

    assert_eq!(answer.status, 200);
    assert_eq!(answer.body, br#"{"jsonrpc":2}"#.to_vec());
}

/// A notification is acknowledged with no body at all, and that is an answer
/// rather than a failure.
#[test]
fn an_empty_body_is_an_answer() {
    let answer = answered(&raw("HTTP/1.1 202 Accepted\r\nContent-Length: 0", ""), 4180)
        .expect("an answer");

    assert_eq!(answer.status, 202);
    assert!(answer.body.is_empty());
}

/// Nothing on either route answers this way. Said out loud rather than
/// half-read, because a de-chunker nothing exercises is worse than a sentence.
#[test]
fn a_chunked_answer_is_refused_rather_than_misread() {
    let refused = answered(
        &raw("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked", "5\r\nhello\r\n0\r\n"),
        4180,
    )
    .expect_err("not carried");

    assert!(matches!(refused, Unreachable::Unreadable { .. }));
    assert!(refused.to_string().contains("chunked"), "{refused}");
}

#[test]
fn something_that_is_not_fleet_is_named_as_such() {
    let refused = answered(b"hello?\r\n\r\n", 4180).expect_err("not an answer");

    assert!(
        refused.to_string().contains("was not an answer Fleet would have sent"),
        "{refused}"
    );
}

/// **The one case a canned buffer cannot prove**: that the request this writes
/// by hand is one a server reads, over a real socket, with the answer framed by
/// the connection closing.
#[test]
fn a_real_socket_carries_a_request_and_an_answer() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("an address").port();
    let served = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("one connection");
        let mut asked = Vec::new();
        let mut chunk = [0u8; 1024];
        while !asked.windows(4).any(|four| four == b"\r\n\r\n") || !asked.ends_with(b"}") {
            match socket.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(read) => asked.extend_from_slice(&chunk[..read]),
            }
        }
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 9\r\n\r\n\
                  {\"ok\":1}\n",
            )
            .expect("an answer");
        String::from_utf8_lossy(&asked).to_string()
    });

    let answer = crate::loopback::Loopback::at(port)
        .post("/agent/mcp", br#"{"method":"ping"}"#)
        .expect("an answer");
    let asked = served.join().expect("the server thread");

    assert_eq!(answer.status, 200);
    assert_eq!(answer.body, b"{\"ok\":1}\n".to_vec());
    assert!(asked.starts_with("POST /agent/mcp HTTP/1.1\r\n"), "{asked}");
    assert!(asked.contains("Content-Length: 17"), "{asked}");
    assert!(asked.contains("Connection: close"), "{asked}");
    assert!(asked.ends_with(r#"{"method":"ping"}"#), "{asked}");
}
