//! `editing::create` against a real directory, which is its whole subject.

use crate::editing::{create, NotCreated};
use crate::tests::tmp::TempDir;

const TEXT: &str = "version: 1\nid: storefront\n";

#[test]
fn create_puts_the_bytes_down_and_leaves_nothing_beside_them() {
    let dir = TempDir::new();
    let file = dir.path().join("armada.yml");

    create(&file, TEXT).expect("the file lands");

    assert_eq!(std::fs::read_to_string(&file).expect("reads back"), TEXT);
    config::Manifest::load(&file).unwrap_or_else(|why| panic!("the file loads: {why}"));
    assert!(!dir.path().join(".armada.yml.saving").exists());
}

/// What is there stays there, and comes back to show.
#[test]
fn a_file_already_there_is_never_written_over() {
    let dir = TempDir::new();
    let file = dir.path().join("armada.yml");
    let theirs = "version: 1\nid: mine\n";
    std::fs::write(&file, theirs).expect("a file");

    match create(&file, TEXT) {
        Err(NotCreated::Appeared(Some(on_disk))) => assert_eq!(on_disk, theirs),
        other => panic!("refused as appeared, not {other:?}"),
    }
    assert_eq!(std::fs::read_to_string(&file).expect("reads"), theirs);
    assert!(!dir.path().join(".armada.yml.saving").exists());
}

#[test]
fn a_create_into_a_directory_that_is_gone_says_it_would_not_write() {
    let dir = TempDir::new();
    let file = dir.path().join("gone").join("armada.yml");
    assert!(matches!(
        create(&file, TEXT),
        Err(NotCreated::Unwritable(_))
    ));
}
