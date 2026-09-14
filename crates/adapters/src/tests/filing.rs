//! Reading where the forge filed an issue. #906.

use crate::filing::issue_url;

#[test]
fn the_address_is_the_last_line_the_forge_prints() {
    let printed =
        "\nCreating issue in NickMele/armada\n\nhttps://github.com/NickMele/armada/issues/990\n";
    assert_eq!(
        issue_url(printed).as_deref(),
        Some("https://github.com/NickMele/armada/issues/990")
    );
}

#[test]
fn output_that_names_no_issue_names_none() {
    assert_eq!(issue_url("Creating issue in NickMele/armada\n"), None);
    assert_eq!(
        issue_url("https://github.com/NickMele/armada/pull/12\n"),
        None
    );
}
