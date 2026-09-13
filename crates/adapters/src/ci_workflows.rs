//! GitHub Actions: each workflow's jobs, and the `run` steps they take.
//!
//! **One finding per step, whatever the matrix.** On this machine a matrix
//! collapses to one run, so a step names the first cell it would run as rather
//! than being multiplied by every cell.
//!
//! **A provider this read does not follow is said, never skipped** — the files
//! of the ones it recognises are claimed and reported as not followed.

use adapter_traits::{
    CiCommand, CiConfiguration, CiNotFollowed, CiReading, FileEntry, FileRead, RepositoryFiles,
};
use serde::Deserialize;
use serde_yaml_ng::{Mapping, Value};

/// Reads `.github/workflows`.
pub struct ActionsWorkflows;

impl ActionsWorkflows {
    /// Only this directory and not below it, which is all Actions reads.
    pub const DIRECTORY: &'static str = ".github/workflows";
}

/// CI files of providers this read recognises and does not follow.
const NOT_FOLLOWED: &[(&str, &str)] = &[
    (".appveyor.yml", "AppVeyor"),
    (".buildkite/pipeline.yaml", "Buildkite"),
    (".buildkite/pipeline.yml", "Buildkite"),
    (".circleci/config.yml", "CircleCI"),
    (".drone.yml", "Drone CI"),
    (".gitlab-ci.yml", "GitLab CI"),
    (".travis.yml", "Travis CI"),
    (".woodpecker.yml", "Woodpecker CI"),
    ("Jenkinsfile", "Jenkins"),
    ("appveyor.yml", "AppVeyor"),
    ("azure-pipelines.yml", "Azure Pipelines"),
    ("bitbucket-pipelines.yml", "Bitbucket Pipelines"),
    ("cloudbuild.yaml", "Cloud Build"),
    ("cloudbuild.yml", "Cloud Build"),
];

/// Directories of YAML pipelines this read does not follow, Actions-shaped or not.
const NOT_FOLLOWED_DIRS: &[(&str, &str)] = &[
    (".forgejo/workflows", "Forgejo Actions"),
    (".gitea/workflows", "Gitea Actions"),
    (".woodpecker", "Woodpecker CI"),
];

/// Past this many combinations the first cell not excluded is not looked for.
const CELLS_LOOKED_AT: usize = 1024;

impl CiConfiguration for ActionsWorkflows {
    fn read_jobs(&self, files: &dyn RepositoryFiles) -> CiReading {
        let mut reading = CiReading::default();
        for file in yaml_in(files, Self::DIRECTORY, &mut reading) {
            reading.claimed.push(file.clone());
            match files.read(&file) {
                FileRead::Bytes(bytes) => workflow(&file, &bytes, &mut reading),
                FileRead::Unreadable(why) => {
                    not_followed(&mut reading, file, format!("would not read: {why}"))
                }
                FileRead::Absent => {}
            }
        }
        for (file, provider) in NOT_FOLLOWED {
            if !matches!(files.read(file), FileRead::Absent) {
                claim_unfollowed(&mut reading, file.to_string(), provider);
            }
        }
        for (dir, provider) in NOT_FOLLOWED_DIRS {
            for file in yaml_in(files, dir, &mut reading) {
                claim_unfollowed(&mut reading, file, provider);
            }
        }
        reading
    }
}

fn claim_unfollowed(reading: &mut CiReading, file: String, provider: &str) {
    reading.claimed.push(file.clone());
    let why = format!("CI configuration for {provider}, which this read does not follow");
    not_followed(reading, file, why);
}

fn not_followed(reading: &mut CiReading, file: String, why: impl Into<String>) {
    reading.not_followed.push(CiNotFollowed {
        file,
        why: why.into(),
    });
}

/// The YAML files directly in `dir`, by name. A directory that is there and
/// would not list is not followed rather than empty.
fn yaml_in(files: &dyn RepositoryFiles, dir: &str, reading: &mut CiReading) -> Vec<String> {
    let entries = match entries_of(files, dir) {
        None => return Vec::new(),
        Some(Ok(entries)) => entries,
        Some(Err(why)) => {
            not_followed(reading, dir.to_string(), format!("would not list: {why}"));
            return Vec::new();
        }
    };
    let mut names: Vec<String> = entries
        .into_iter()
        .filter(|entry| !entry.is_dir && is_yaml(&entry.name))
        .map(|entry| format!("{dir}/{}", entry.name))
        .collect();
    names.sort();
    names
}

/// `None` where `dir` is not there, found through its parents so a directory
/// that is absent is told apart from one that would not list.
fn entries_of(files: &dyn RepositoryFiles, dir: &str) -> Option<Result<Vec<FileEntry>, String>> {
    let (parent, name) = dir.rsplit_once('/').unwrap_or(("", dir));
    let siblings = match parent.is_empty() {
        // A root that will not list is Scan's to report, once.
        true => files.entries("").ok()?,
        false => match entries_of(files, parent)? {
            Ok(siblings) => siblings,
            Err(why) => return Some(Err(why)),
        },
    };
    let there = siblings
        .iter()
        .any(|entry| entry.is_dir && entry.name == name);
    there.then(|| files.entries(dir))
}

fn is_yaml(name: &str) -> bool {
    name.ends_with(".yml") || name.ends_with(".yaml")
}

#[derive(Deserialize)]
struct Workflow {
    jobs: Mapping,
}

#[derive(Deserialize)]
struct Job {
    #[serde(default)]
    uses: Option<String>,
    #[serde(default)]
    strategy: Option<Strategy>,
    #[serde(default)]
    steps: Vec<Value>,
}

#[derive(Deserialize)]
struct Strategy {
    #[serde(default)]
    matrix: Option<Value>,
}

#[derive(Deserialize)]
struct Step {
    #[serde(default)]
    run: Option<String>,
}

fn workflow(file: &str, bytes: &[u8], reading: &mut CiReading) {
    let workflow = match serde_yaml_ng::from_slice::<Workflow>(bytes) {
        Ok(workflow) => workflow,
        Err(why) => {
            let why = format!("does not read as a workflow: {why}");
            return not_followed(reading, file.to_string(), why);
        }
    };
    for (id, job) in &workflow.jobs {
        let Some(id) = id.as_str() else {
            not_followed(reading, file.to_string(), "a job whose id is not text");
            continue;
        };
        let cited = format!("{file}: jobs.{id}");
        let job = match serde_yaml_ng::from_value::<Job>(job.clone()) {
            Ok(job) => job,
            Err(why) => {
                not_followed(
                    reading,
                    cited,
                    format!("a job this read does not follow: {why}"),
                );
                continue;
            }
        };
        if job.uses.is_some() {
            not_followed(
                reading,
                cited,
                "a reusable workflow, which this read does not follow into",
            );
            continue;
        }
        let cell = job
            .strategy
            .and_then(|strategy| strategy.matrix)
            .and_then(|matrix| first_cell(&matrix));
        for (n, step) in job.steps.into_iter().enumerate() {
            let key = format!("jobs.{id}.steps[{n}]");
            match serde_yaml_ng::from_value::<Step>(step) {
                Ok(Step { run: Some(run) }) => reading.commands.push(CiCommand {
                    file: file.to_string(),
                    job: id.to_string(),
                    key: format!("{key}.run"),
                    // A block scalar's last newline is YAML's, not the command's.
                    run: run.trim_end_matches('\n').to_string(),
                    cell: cell.clone(),
                }),
                Ok(Step { run: None }) => {}
                Err(why) => {
                    let why = format!("a step this read does not follow: {why}");
                    not_followed(reading, format!("{file}: {key}"), why);
                }
            }
        }
    }
}

/// The first cell a matrix would run, as `axis=value` pairs. An expression is
/// named as written, since what it expands to is not in the file.
fn first_cell(matrix: &Value) -> Option<String> {
    let axes = match matrix {
        Value::String(expression) => return Some(expression.clone()),
        Value::Mapping(axes) => axes,
        _ => return None,
    };
    let mut named: Vec<(String, Vec<Value>)> = Vec::new();
    for (axis, values) in axes {
        let Some(axis) = axis
            .as_str()
            .filter(|a| !matches!(*a, "include" | "exclude"))
        else {
            continue;
        };
        let values = match values {
            Value::Sequence(values) => values.clone(),
            other => vec![other.clone()],
        };
        named.push((axis.to_string(), values));
    }
    let excluded = listed(axes.get("exclude"));
    let included = listed(axes.get("include"));

    let crossed = !named.is_empty() && named.iter().all(|(_, values)| !values.is_empty());
    if crossed {
        let mut at = vec![0usize; named.len()];
        for _ in 0..CELLS_LOOKED_AT {
            let cell: Vec<(&str, &Value)> = named
                .iter()
                .zip(&at)
                .map(|((axis, values), i)| (axis.as_str(), &values[*i]))
                .collect();
            if !excluded.iter().any(|exclude| matches_cell(exclude, &cell)) {
                return Some(rendered(cell));
            }
            if !advance(&mut at, &named) {
                break;
            }
        }
    }
    let first = included.first()?;
    let pairs = first
        .iter()
        .filter_map(|(axis, value)| Some((axis.as_str()?, value)));
    Some(rendered(pairs.collect()))
}

fn listed(value: Option<&Value>) -> Vec<Mapping> {
    let Some(Value::Sequence(entries)) = value else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|e| e.as_mapping().cloned())
        .collect()
}

/// Odometer order, the last axis turning fastest. `false` once every cell is seen.
fn advance(at: &mut [usize], named: &[(String, Vec<Value>)]) -> bool {
    for (i, (_, values)) in named.iter().enumerate().rev() {
        at[i] += 1;
        if at[i] < values.len() {
            return true;
        }
        at[i] = 0;
    }
    false
}

fn matches_cell(exclude: &Mapping, cell: &[(&str, &Value)]) -> bool {
    exclude.iter().all(|(axis, value)| {
        cell.iter()
            .any(|(name, held)| axis.as_str() == Some(*name) && *held == value)
    })
}

fn rendered(cell: Vec<(&str, &Value)>) -> String {
    let pairs: Vec<String> = cell
        .into_iter()
        .map(|(axis, value)| format!("{axis}={}", scalar(value)))
        .collect();
    pairs.join(", ")
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => "null".to_string(),
        other => serde_yaml_ng::to_string(other)
            .unwrap_or_default()
            .trim()
            .replace('\n', ", "),
    }
}
