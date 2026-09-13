//! A preference a person saves survives a restart, and a name outside the
//! closed set is refused by name on the wire code `fleet.unknown_preference`.

use api::{Commands, Queries, Refusal};
use ipc::{Preferences, SavePreference};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

fn saving(name: &str, value: bool) -> SavePreference {
    SavePreference {
        name: name.to_string(),
        value,
    }
}

#[tokio::test]
async fn nothing_saved_reads_as_the_shipped_default() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(
        fleet.get_preferences().await.expect("reads"),
        Preferences::default()
    );
}

#[tokio::test]
async fn a_saved_preference_survives_a_restart() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let now = fleet
        .save_preferences(saving("where_things_are_open", true))
        .await
        .expect("saved");
    assert!(now.where_things_are_open);
    drop(fleet);

    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(
        fleet.get_preferences().await.expect("reads"),
        Preferences {
            where_things_are_open: true
        }
    );
}

#[tokio::test]
async fn an_unknown_name_is_refused_by_name_and_saves_nothing() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));

    let refused = fleet
        .save_preferences(saving("where_things_are_purple", true))
        .await
        .expect_err("not a preference this build reads");
    match refused {
        Refusal::Unacceptable(error) => {
            assert_eq!(error.code, "fleet.unknown_preference");
            assert!(error.message.contains("where_things_are_purple"));
        }
        other => panic!("expected Unacceptable, got {other:?}"),
    }
    assert_eq!(
        fleet.get_preferences().await.expect("reads"),
        Preferences::default(),
        "nothing was saved"
    );
}
