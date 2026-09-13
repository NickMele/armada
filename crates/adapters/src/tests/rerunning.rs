//! Which runs a re-run starts: each failed check's run, read off its link, once. #905.

use crate::rerunning::run_ids;

#[test]
fn each_failed_checks_run_is_named_once() {
    let links = "https://github.com/o/r/actions/runs/123/job/9\n\
                 https://github.com/o/r/actions/runs/123/job/10\n\
                 https://github.com/o/r/actions/runs/456/job/1\n";
    let ids: Vec<String> = run_ids(links).into_iter().collect();
    assert_eq!(ids, vec!["123".to_string(), "456".to_string()]);
}

#[test]
fn a_check_whose_link_names_no_run_starts_nothing() {
    assert!(run_ids("https://ci.example/build/7\n\n").is_empty());
}
