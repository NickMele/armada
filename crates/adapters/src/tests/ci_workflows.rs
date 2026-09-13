//! Workflow files held in memory: what is read is the subject, and nothing is
//! written or run to read it.

use std::collections::BTreeMap;

use adapter_traits::{CiConfiguration, CiReading, FileEntry, FileRead, RepositoryFiles};

use crate::ActionsWorkflows;

struct Held(BTreeMap<String, String>);

impl RepositoryFiles for Held {
    fn read(&self, path: &str) -> FileRead {
        match self.0.get(path) {
            Some(text) => FileRead::Bytes(text.as_bytes().to_vec()),
            None => FileRead::Absent,
        }
    }

    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String> {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{dir}/")
        };
        let mut found: BTreeMap<String, bool> = BTreeMap::new();
        for path in self.0.keys() {
            if let Some(rest) = path.strip_prefix(&prefix) {
                match rest.split_once('/') {
                    Some((child, _)) => found.insert(child.to_string(), true),
                    None => found.insert(rest.to_string(), false),
                };
            }
        }
        if found.is_empty() && !dir.is_empty() {
            return Err(format!("{dir} is not a directory"));
        }
        Ok(found
            .into_iter()
            .map(|(name, is_dir)| FileEntry { name, is_dir })
            .collect())
    }
}

fn read(files: &[(&str, &str)]) -> CiReading {
    let held = Held(
        files
            .iter()
            .map(|(path, text)| (path.to_string(), text.to_string()))
            .collect(),
    );
    ActionsWorkflows.read_jobs(&held)
}

type Row<'a> = (&'a str, &'a str, &'a str, &'a str, Option<&'a str>);

fn rows(reading: &CiReading) -> Vec<Row<'_>> {
    reading
        .commands
        .iter()
        .map(|c| {
            (
                c.file.as_str(),
                c.job.as_str(),
                c.key.as_str(),
                c.run.as_str(),
                c.cell.as_deref(),
            )
        })
        .collect()
}

#[test]
fn each_jobs_run_steps_are_read_with_the_file_and_job_each_came_from() {
    let reading = read(&[
        (
            ".github/workflows/ci.yml",
            "name: ci\non: [push]\njobs:\n  lint:\n    runs-on: ubuntu-latest\n    steps:\n      \
             - uses: actions/checkout@v4\n      - run: pnpm lint\n  test:\n    steps:\n      \
             - name: both\n        run: |\n          pnpm install\n          pnpm test\n",
        ),
        (
            ".github/workflows/release.yaml",
            "jobs:\n  ship:\n    steps:\n      - run: make dist\n",
        ),
        (".github/workflows/README.md", "not a workflow"),
        (
            ".github/workflows/nested/deep.yml",
            "jobs:\n  x:\n    steps:\n      - run: never\n",
        ),
        (".github/dependabot.yml", "version: 2\n"),
    ]);
    assert_eq!(
        rows(&reading),
        [
            (
                ".github/workflows/ci.yml",
                "lint",
                "jobs.lint.steps[1].run",
                "pnpm lint",
                None
            ),
            (
                ".github/workflows/ci.yml",
                "test",
                "jobs.test.steps[0].run",
                "pnpm install\npnpm test",
                None
            ),
            (
                ".github/workflows/release.yaml",
                "ship",
                "jobs.ship.steps[0].run",
                "make dist",
                None
            ),
        ]
    );
    assert!(
        reading.not_followed.is_empty(),
        "{:?}",
        reading.not_followed
    );
    assert_eq!(
        reading.claimed,
        [".github/workflows/ci.yml", ".github/workflows/release.yaml"],
        "what Actions does not read is left for Scan to say"
    );
}

#[test]
fn a_matrix_is_one_finding_per_step_naming_the_cell_it_came_from() {
    let reading = read(&[(
        ".github/workflows/ci.yml",
        "jobs:\n  test:\n    strategy:\n      matrix:\n        os: [ubuntu-latest, macos-latest]\n        \
         node: [20, 22]\n        exclude:\n          - os: ubuntu-latest\n            node: 20\n    \
         steps:\n      - run: pnpm install\n      - run: pnpm test\n  \
         dynamic:\n    strategy:\n      matrix: ${{ fromJSON(needs.plan.outputs.cells) }}\n    \
         steps:\n      - run: cargo test\n  \
         listed:\n    strategy:\n      matrix:\n        include:\n          - target: wasm\n    \
         steps:\n      - run: cargo build\n",
    )]);
    let cells: Vec<(&str, Option<&str>)> = rows(&reading)
        .into_iter()
        .map(|(_, job, _, _, cell)| (job, cell))
        .collect();
    assert_eq!(
        cells,
        [
            ("test", Some("os=ubuntu-latest, node=22")),
            ("test", Some("os=ubuntu-latest, node=22")),
            ("dynamic", Some("${{ fromJSON(needs.plan.outputs.cells) }}")),
            ("listed", Some("target=wasm")),
        ],
        "four cells, two steps: two findings, the first cell not excluded"
    );
}

#[test]
fn a_provider_this_read_does_not_know_reads_not_followed_never_clean() {
    let reading = read(&[
        (".gitlab-ci.yml", "test:\n  script: [make test]\n"),
        (".circleci/config.yml", "version: 2.1\n"),
        (".woodpecker/build.yaml", "steps: []\n"),
        ("Jenkinsfile", "pipeline {}\n"),
    ]);
    assert!(reading.commands.is_empty());
    let files: Vec<&str> = reading
        .not_followed
        .iter()
        .map(|one| one.file.as_str())
        .collect();
    assert_eq!(
        files,
        [
            ".circleci/config.yml",
            ".gitlab-ci.yml",
            "Jenkinsfile",
            ".woodpecker/build.yaml"
        ]
    );
    assert!(reading
        .not_followed
        .iter()
        .all(|one| one.why.contains("does not follow")));
    assert_eq!(
        reading.claimed, files,
        "claimed, so nothing else reports them twice"
    );
}

#[test]
fn a_workflow_or_job_that_will_not_read_is_not_followed_and_the_rest_still_are() {
    let reading = read(&[
        (".github/workflows/broken.yml", "jobs: [\n"),
        (
            ".github/workflows/ci.yml",
            "jobs:\n  shared:\n    uses: ./.github/workflows/reusable.yml\n  odd:\n    steps:\n      \
             - run: [not, text]\n      - run: still read\n",
        ),
    ]);
    let files: Vec<&str> = reading
        .not_followed
        .iter()
        .map(|one| one.file.as_str())
        .collect();
    assert_eq!(
        files,
        [
            ".github/workflows/broken.yml",
            ".github/workflows/ci.yml: jobs.shared",
            ".github/workflows/ci.yml: jobs.odd.steps[0]"
        ]
    );
    assert_eq!(rows(&reading)[0].3, "still read");
}

#[test]
fn a_repository_with_no_ci_configuration_reads_as_nothing_claimed() {
    assert_eq!(read(&[("package.json", "{}")]), CiReading::default());
}
