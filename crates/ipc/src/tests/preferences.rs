//! What `get_preferences` and `save_preferences` must keep true: the answer
//! is flat, and a save is a name and a value with nothing else read from it.

use crate::{decode, encode, Preferences, SavePreference};

#[test]
fn nothing_saved_reads_as_the_shipped_default() {
    assert_eq!(
        Preferences::default(),
        Preferences {
            where_things_are_open: false
        }
    );
}

#[test]
fn preferences_round_trip_flat() {
    let preferences = Preferences {
        where_things_are_open: true,
    };
    let json = encode(&preferences).expect("plain data");
    assert_eq!(json, r#"{"where_things_are_open":true}"#);
    assert_eq!(
        decode::<Preferences>("preferences", json.as_bytes()).expect("round-trips"),
        preferences
    );
}

#[test]
fn a_save_names_one_preference_and_a_value() {
    let body = br#"{"name":"where_things_are_open","value":true}"#;
    let save = decode::<SavePreference>("a preference to save", body).expect("plain data");
    assert_eq!(
        save,
        SavePreference {
            name: "where_things_are_open".to_string(),
            value: true,
        }
    );
}

/// **The name is a plain string, not a closed wire type.** An unrecognised
/// name still decodes — refusing it by name is `store`'s and `fleet`'s, not a
/// deserializer's, so the message can say exactly what was sent.
#[test]
fn an_unrecognised_name_still_decodes() {
    let body = br#"{"name":"where_things_are_purple","value":true}"#;
    decode::<SavePreference>("a preference to save", body).expect("the wire type takes any name");
}
