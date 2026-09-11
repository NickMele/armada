//! A run's channel: what a viewer is sent, and what it is told it missed.
//!
//! The socket's framing is `observing`'s and is proved there. What is new here
//! is the offset rule that keeps a line out of both the history and the live
//! feed, and the bound's two promises: drop-oldest, and say so.

use crate::{RunChunk, RunFeed, RunSeen, RUN_BACKLOG};

fn chunk(lines: &[(u64, &str)]) -> RunChunk {
    RunChunk {
        lines: lines
            .iter()
            .map(|(end, line)| (*end, line.to_string()))
            .collect(),
    }
}

/// The opening read stopped at byte 6, after `first\n`. A pass that began
/// before it and ended after it sends only what the read had not.
#[test]
fn a_line_the_history_already_sent_is_not_sent_again() {
    let straddling = chunk(&[(6, "first"), (13, "second")]);
    assert_eq!(straddling.after(6), vec!["second".to_string()]);
    assert!(chunk(&[(6, "first")]).after(6).is_empty());
}

#[tokio::test]
async fn a_viewer_that_falls_behind_is_told_how_many_it_lost_and_the_end_closes_it() {
    let feed = RunFeed::new();
    let mut watch = feed.watch();
    let over = 3u64;
    for at in 0..(RUN_BACKLOG as u64 + over) {
        feed.offer(chunk(&[(at + 1, &at.to_string())]));
    }
    assert_eq!(watch.next().await, Some(RunSeen::Missed(over)));
    assert_eq!(
        watch.next().await,
        Some(RunSeen::Chunk(chunk(&[(over + 1, &over.to_string())]))),
        "the oldest went and the rest follow in order"
    );
    drop(feed);
    let mut left = 0;
    while let Some(seen) = watch.next().await {
        assert!(matches!(seen, RunSeen::Chunk(_)));
        left += 1;
    }
    assert_eq!(
        left,
        RUN_BACKLOG - 1,
        "what was held is read before the end"
    );
}

#[test]
fn an_empty_pass_is_not_offered() {
    let feed = RunFeed::new();
    let mut watch = feed.watch();
    feed.offer(chunk(&[]));
    drop(feed);
    assert!(tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime")
        .block_on(watch.next())
        .is_none());
}
