//! What `get_limits` and `save_limits` must keep true: a value out of range
//! does not decode, an omitted field is absent, and the answer is flat.

use crate::{decode, encode, FleetLimits, LimitValues, SaveLimits};

#[test]
fn each_bound_is_accepted_and_one_past_it_is_refused() {
    for (body, takes) in [
        (r#"{"concurrency":1}"#, true),
        (r#"{"concurrency":8}"#, true),
        (r#"{"concurrency":0}"#, false),
        (r#"{"concurrency":9}"#, false),
        (r#"{"memory_spare_percent":0}"#, true),
        (r#"{"memory_spare_percent":50}"#, true),
        (r#"{"memory_spare_percent":51}"#, false),
        (r#"{"disk_floor_gib":0}"#, true),
        (r#"{"disk_floor_gib":100}"#, true),
        (r#"{"disk_floor_gib":101}"#, false),
        (r#"{"disk_floor_gib":-1}"#, false),
        (r#"{"concurrency":2.5}"#, false),
    ] {
        let read = decode::<SaveLimits>("limits to save", body.as_bytes());
        assert_eq!(read.is_ok(), takes, "{body}: {read:?}");
    }
}

/// **One bad field refuses the whole save**, so a request is never half taken.
#[test]
fn one_value_out_of_range_refuses_the_rest_of_the_save_with_it() {
    let body = br#"{"concurrency":3,"disk_floor_gib":500}"#;
    let refused = decode::<SaveLimits>("limits to save", body).expect_err("500 is past 100");
    assert!(refused.to_string().contains("500"), "{refused}");
}

#[test]
fn an_empty_save_omits_every_field_and_round_trips() {
    let empty = decode::<SaveLimits>("limits to save", b"{}").expect("nothing is valid");
    assert_eq!(empty, SaveLimits::default());
    assert_eq!(encode(&empty).expect("plain data"), "{}");
}

#[test]
fn the_limits_in_force_are_flat_with_what_shipped_beside_them() {
    let limits = FleetLimits {
        values: LimitValues {
            concurrency: 4,
            memory_spare_percent: 15,
            disk_floor_gib: 10,
            checks_at_once: 4,
        },
        shipped: LimitValues {
            concurrency: 2,
            memory_spare_percent: 15,
            disk_floor_gib: 10,
            checks_at_once: 4,
        },
    };
    let json = encode(&limits).expect("plain data");
    assert!(json.starts_with(r#"{"concurrency":4,"#), "{json}");
    assert!(json.contains(r#""shipped":{"concurrency":2,"#), "{json}");
    assert_eq!(
        decode::<FleetLimits>("limits", json.as_bytes()).expect("round-trips"),
        limits
    );
}
