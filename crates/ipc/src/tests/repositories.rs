use crate::{decode, encode, CloneRepository};

#[test]
fn a_clone_is_a_url_and_a_parent_and_a_newer_field_does_not_break_it() {
    let asked = CloneRepository {
        url: "/srv/repos/shop.git".to_string(),
        parent: "/projects".to_string(),
    };
    let spelled = encode(&asked).expect("plain data");
    assert_eq!(
        spelled,
        r#"{"url":"/srv/repos/shop.git","parent":"/projects"}"#
    );
    let newer = r#"{"url":"/srv/repos/shop.git","parent":"/projects","depth":1}"#;
    let read: CloneRepository = decode("a clone", newer.as_bytes()).expect("reads");
    assert_eq!(read, asked);
    let no_parent = decode::<CloneRepository>("a clone", br#"{"url":"/srv/repos/shop.git"}"#);
    assert!(no_parent.is_err(), "a parent is required");
}
