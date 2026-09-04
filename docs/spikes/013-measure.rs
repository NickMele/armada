//! Times `WorkProduct::changed_files` — the one read #431 would make universal —
//! and the absolute-tier scan over its answer.
//!
//! Usage: measure-changed-files <worktree-path> <branch> <iterations> <label>
//! Prints one CSV row per call: label,iteration,micros,files,scan_micros.

use std::time::Instant;

use adapter_traits::{Changed, WorkProduct, Worktree};
use adapters::GitVcs;
use core_model::RepoPath;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args[1].clone();
    let branch = args[2].clone();
    let iterations: usize = args[3].parse().expect("an iteration count");
    let label = args[4].clone();

    let vcs = GitVcs::new();
    let worktree = Worktree::at(path, branch);

    let mut micros: Vec<u128> = Vec::with_capacity(iterations);
    let mut scan_micros: Vec<u128> = Vec::with_capacity(iterations);
    let mut files = 0usize;
    for n in 0..iterations {
        let started = Instant::now();
        let changed = vcs.changed_files(&worktree).expect("a reading");
        let took = started.elapsed().as_micros();
        files = changed.len();
        let paths: Vec<RepoPath> = Changed::paths(&changed)
            .into_iter()
            .map(|each| RepoPath::new(&each))
            .collect();
        let scan_started = Instant::now();
        let found = verification::forbidden_among(paths.iter());
        let scan = scan_started.elapsed().as_micros();
        std::hint::black_box(&found);
        println!("{label},{n},{took},{files},{scan}");
        micros.push(took);
        scan_micros.push(scan);
    }
    let first = micros[0];
    micros.sort_unstable();
    scan_micros.sort_unstable();
    let pick = |v: &Vec<u128>, q: f64| v[((v.len() as f64 - 1.0) * q) as usize];
    eprintln!(
        "SUMMARY {label} files={files} n={} first={first}us min={}us median={}us p90={}us max={}us scan_median={}us",
        micros.len(),
        pick(&micros, 0.0),
        pick(&micros, 0.5),
        pick(&micros, 0.9),
        pick(&micros, 1.0),
        pick(&scan_micros, 0.5),
    );
}
