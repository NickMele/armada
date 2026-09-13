//! An empty optional field leaves no key, because Bridge reads a key as present.
//!
//! Both of these were sent as `null`: a flag with no line drew `file:null`, and
//! an argument of unknown length offered the rest of something it could not size.

use crate::{decode, encode, CallArguments, CitedAt};

#[test]
fn a_citation_about_the_whole_file_carries_no_line_key() {
    let at = CitedAt {
        file: "src/lib.rs".to_string(),
        line: None,
    };
    let written = encode(&at).expect("a citation encodes");
    assert_eq!(written, r#"{"file":"src/lib.rs"}"#);
    let read: CitedAt = decode("a citation", written.as_bytes()).expect("it reads");
    assert_eq!(read, at);
}

#[test]
fn an_argument_of_unknown_length_carries_no_length_key() {
    let call = CallArguments {
        tool: "Bash".to_string(),
        call: "toolu_01".to_string(),
        arguments: "ls".to_string(),
        whole: false,
        length: None,
    };
    let written = encode(&call).expect("an argument encodes");
    assert!(
        !written.contains("length") && !written.contains("null"),
        "{written}"
    );
    let read: CallArguments = decode("an argument", written.as_bytes()).expect("it reads");
    assert_eq!(read, call);
}
