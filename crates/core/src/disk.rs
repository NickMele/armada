//! What Docker is holding on this machine, and whose it is.
//!
//! Two questions with two different answers, and keeping them apart is the
//! whole of this module:
//!
//! | Question | Source | Remedy |
//! |---|---|---|
//! | how much is Docker holding, and how much of that is reclaimable | `docker system df` | the user's own `docker volume prune` |
//! | how much of it is **Armada's** | the ownership labels, cross-referenced against `docker system df -v` | `armada manifest clean --all` |
//!
//! **The remedies differ, so the numbers may never be added together.** A row
//! reporting one figure for "reclaimable" invites the reader to run Armada's
//! verb at a machine whose bloat Armada did not cause, or the machine's verb at
//! resources a live workspace is still using. The measured case this was written
//! against had 171 local volumes holding 12.0 GB, 100% reclaimable, and **not
//! one of them carried an Armada label**.
//!
//! # `docker system df` is not a stable API
//!
//! It is a human report with a `--format` template bolted on, and `docs/traps.md`
//! records what it actually emits: one JSON object per line for the summary and
//! a single object of arrays for `-v`, every value a string including the counts,
//! sizes as humanised text with no raw-byte field anywhere, and the volume type
//! spelled `Local Volumes` rather than `volumes`.
//!
//! So three rules hold throughout, and each exists because the alternative is
//! silent:
//!
//! 1. **Parse the template, never the human columns.** A renamed field makes the
//!    template exit non-zero; a renamed column shifts a `split_whitespace` by one
//!    and reports a count as a size.
//! 2. **An unreadable size is [`None`], never `0`.** `0 B` is a confident claim
//!    that there is nothing to reclaim, which is the one wrong answer that stops
//!    the reader looking.
//! 3. **A type Armada does not recognise is carried by name**, not dropped. A
//!    future `docker system df` row is a thing holding disk, and a parser that
//!    silently skips it under-reports for ever.

use serde::Serialize;

/// The four things `docker system df` accounts for.
///
/// **Spelled as the reader knows them, parsed as docker prints them.** The wire
/// spelling is `Local Volumes` — two words — and a parser keying on `volumes`
/// misses the row that holds all the disk, which is exactly the row this whole
/// module exists for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskKind {
    /// Built and pulled images.
    Images,
    /// Containers, running and stopped.
    Containers,
    /// Named volumes. **The leak** — a volume outlives `down`, outlives the
    /// container, and is what 171 of on one machine came to 12.0 GB.
    Volumes,
    /// BuildKit's cache.
    BuildCache,
}

impl DiskKind {
    /// From docker's own spelling, which is not Armada's.
    pub fn parse(text: &str) -> Option<DiskKind> {
        match text.trim() {
            "Images" => Some(DiskKind::Images),
            "Containers" => Some(DiskKind::Containers),
            // Both spellings: the summary says `Local Volumes` and the `-v`
            // payload keys the array `Volumes`.
            "Local Volumes" | "Volumes" => Some(DiskKind::Volumes),
            "Build Cache" | "BuildCache" => Some(DiskKind::BuildCache),
            _ => None,
        }
    }

    /// The word Armada puts on a screen.
    pub const fn word(self) -> &'static str {
        match self {
            DiskKind::Images => "image",
            DiskKind::Containers => "container",
            DiskKind::Volumes => "volume",
            DiskKind::BuildCache => "build cache",
        }
    }
}

/// One row of `docker system df`.
///
/// `size` and `reclaimable` are optional for the reason rule 2 above gives: a
/// size Armada could not read is unknown, and unknown is not zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskRow {
    /// What kind of thing this row accounts for.
    pub kind: DiskKind,
    /// How many there are.
    pub total: u64,
    /// How many are in use.
    pub active: u64,
    /// Bytes held, or [`None`] when docker printed a size Armada cannot read.
    pub size: Option<u64>,
    /// Bytes docker says it could reclaim, or [`None`] for the same reason.
    pub reclaimable: Option<u64>,
}

/// Everything `docker system df` reported, including what it reported that
/// Armada did not understand.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DiskUsage {
    /// One row per recognised type.
    pub rows: Vec<DiskRow>,
    /// Type names docker printed that this version of Armada has no case for.
    ///
    /// **Named rather than dropped.** A row Armada skips is disk it under-reports
    /// for ever, and the reader is the only one who can tell whether it matters.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unrecognised: Vec<String>,
}

impl DiskUsage {
    /// The row for one kind, when docker reported one.
    pub fn row(&self, kind: DiskKind) -> Option<&DiskRow> {
        self.rows.iter().find(|row| row.kind == kind)
    }

    /// Total reclaimable across every row Armada understood.
    ///
    /// [`None`] when **any** contributing row was unreadable, rather than a sum
    /// of the rows that happened to parse: a total quietly missing the volumes
    /// row is the same wrong answer as `0 B` and is harder to notice.
    pub fn reclaimable(&self) -> Option<u64> {
        self.rows
            .iter()
            .try_fold(0u64, |sum, row| Some(sum + row.reclaimable?))
    }
}

/// Parse the summary form: `docker system df --format '{{json .}}'`.
///
/// **One JSON object per line, not an array** — measured, and the reason this
/// reads line by line rather than handing the whole payload to `serde_json`.
///
/// A line that will not parse at all is skipped rather than failing the call:
/// docker has been known to print a warning above its output, and losing the
/// whole report to one stray line would turn a degraded answer into no answer.
pub fn parse_summary(stdout: &str) -> DiskUsage {
    let mut usage = DiskUsage::default();
    for line in stdout.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(kind_text) = value.get("Type").and_then(|t| t.as_str()) else {
            continue;
        };
        let Some(kind) = DiskKind::parse(kind_text) else {
            usage.unrecognised.push(kind_text.to_string());
            continue;
        };
        // Every value is a string, counts included — so the counts are parsed
        // from text too, and an unreadable count is 0 rather than optional
        // because a count is only ever used to say "171 volumes" beside a size
        // that carries its own unknown.
        let number = |key: &str| {
            value
                .get(key)
                .and_then(|v| v.as_str())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(0)
        };
        let size = value
            .get("Size")
            .and_then(|v| v.as_str())
            .and_then(parse_size);
        // `"12.01GB (100%)"` — a size and a percentage in one field. The
        // percentage is docker's own rounding of a ratio Armada can compute, so
        // only the size is kept.
        let reclaimable = value
            .get("Reclaimable")
            .and_then(|v| v.as_str())
            .and_then(|text| parse_size(text.split(" (").next().unwrap_or(text)));
        usage.rows.push(DiskRow {
            kind,
            total: number("TotalCount"),
            active: number("Active"),
            size,
            reclaimable,
        });
    }
    usage
}

/// One volume as `docker system df -v` reported it.
///
/// **`Labels` is deliberately not read from here.** It arrives as a single
/// comma-joined `k=v` string, and `docs/traps.md` already records that a label
/// value may legally contain the delimiter — so ownership is read through
/// `docker volume ls --filter label=…`, which the daemon answers, and this
/// payload contributes the **size** only, matched by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeEntry {
    /// The name docker knows it by, and the key ownership is matched on.
    pub name: String,
    /// Bytes, or [`None`] when the size could not be read.
    pub size: Option<u64>,
}

/// Parse the volume sizes out of `docker system df -v --format '{{json .}}'`.
///
/// A **single** object of arrays, which is a different shape from the summary
/// form above — the same flag on the same command changes the payload's
/// structure, which is worth knowing before writing one parser for both.
pub fn parse_verbose_volumes(stdout: &str) -> Vec<VolumeEntry> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout.trim()) else {
        return Vec::new();
    };
    let Some(volumes) = value.get("Volumes").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    volumes
        .iter()
        .filter_map(|entry| {
            Some(VolumeEntry {
                name: entry.get("Name")?.as_str()?.to_string(),
                size: entry
                    .get("Size")
                    .and_then(|v| v.as_str())
                    .and_then(parse_size),
            })
        })
        .collect()
}

/// Whose a resource is, which is the only question that decides what may happen
/// to it without being asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    /// It carries Armada's owner label. Armada made it and may offer to remove
    /// it.
    Armada,
    /// It carries no Armada label. **Armada did not create it and it is not
    /// Armada's to delete** — one of them may be a database somebody cares
    /// about, and a deleted volume does not come back.
    Unlabelled,
}

/// Armada's share of the disk, separated from everything else's.
///
/// **Two totals that are never summed**, for the reason at the top of this file:
/// they have different remedies, and a reader given one number acts on the wrong
/// one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Share {
    /// How many volumes carry an Armada owner label.
    pub armada_count: usize,
    /// What they hold, in bytes. [`None`] when any of their sizes was
    /// unreadable — the same rule [`DiskUsage::reclaimable`] follows.
    pub armada_bytes: Option<u64>,
    /// How many volumes do not.
    pub other_count: usize,
    /// What **those** hold. Reported so the reader can see the proportion, and
    /// never as something Armada offers to reclaim.
    pub other_bytes: Option<u64>,
}

/// Split what docker is holding into Armada's and everyone else's.
///
/// `armada_names` comes from `docker volume ls --filter label=…`, which is the
/// daemon's own answer and the only trustworthy one; `volumes` comes from
/// `docker system df -v`, which is where the sizes live. Matching them by name
/// is what avoids parsing the comma-joined label string.
pub fn split(volumes: &[VolumeEntry], armada_names: &[String]) -> Share {
    let mut share = Share {
        armada_bytes: Some(0),
        other_bytes: Some(0),
        ..Share::default()
    };
    for volume in volumes {
        let ours = armada_names.iter().any(|name| name == &volume.name);
        let (count, bytes) = match ours {
            true => (&mut share.armada_count, &mut share.armada_bytes),
            false => (&mut share.other_count, &mut share.other_bytes),
        };
        *count += 1;
        // One unreadable size makes the total unknown and keeps it unknown. A
        // total that silently omits a volume reads as a complete answer.
        *bytes = match (*bytes, volume.size) {
            (Some(sum), Some(size)) => Some(sum + size),
            _ => None,
        };
    }
    share
}

/// Bytes, from docker's humanised spelling.
///
/// **Base 1000, and `kB` has a lowercase `k`** — measured across 171 volumes,
/// the spellings docker actually emitted were `B`, `kB` and `MB`. Reading `kB`
/// as 1024 is a 2.4% error that compounds to a wrong gigabyte figure on the row
/// that matters.
///
/// Returns [`None`] rather than `0` for anything it cannot read, which is rule 2
/// at the top of this file.
pub fn parse_size(text: &str) -> Option<u64> {
    let text = text.trim();
    let split = text.find(|c: char| c.is_ascii_alphabetic())?;
    let (number, unit) = text.split_at(split);
    let number: f64 = number.trim().parse().ok()?;
    if !number.is_finite() || number < 0.0 {
        return None;
    }
    // Case-insensitive on the multiplier because there is no ambiguity among
    // these six, and docker's own casing has changed before.
    let multiplier: f64 = match unit.trim().to_ascii_lowercase().as_str() {
        "b" => 1.0,
        "kb" => 1e3,
        "mb" => 1e6,
        "gb" => 1e9,
        "tb" => 1e12,
        "pb" => 1e15,
        _ => return None,
    };
    Some((number * multiplier) as u64)
}

/// Bytes, in Armada's spelling.
///
/// **One ladder, in one place.** Printing sizes in `doctor` and in `prune` from
/// two helpers is two ladders that disagree the first time one of them gains a
/// unit, and a reader comparing the two rows would have no way to tell which
/// was wrong.
pub fn format_size(bytes: u64) -> String {
    const LADDER: [(u64, &str); 5] = [
        (1_000_000_000_000_000, "PB"),
        (1_000_000_000_000, "TB"),
        (1_000_000_000, "GB"),
        (1_000_000, "MB"),
        (1_000, "kB"),
    ];
    for (step, unit) in LADDER {
        if bytes >= step {
            // One decimal: the difference between 12.0 GB and 12 GB is whether
            // the reader can tell a rounded figure from an exact one.
            return format!("{:.1} {unit}", bytes as f64 / step as f64);
        }
    }
    format!("{bytes} B")
}

/// A size that may not be known, in Armada's spelling.
///
/// **The unknown case has words rather than a dash**, because a dash in a size
/// column reads as zero to everyone who has ever seen a table.
pub fn format_maybe(bytes: Option<u64>) -> String {
    match bytes {
        Some(bytes) => format_size(bytes),
        None => "size unknown".to_string(),
    }
}

/// One thing `armada manifest prune` could remove.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PruneCandidate {
    /// What kind of thing.
    pub kind: DiskKind,
    /// The name or id docker knows it by.
    pub reference: String,
    /// What removing it would free, when that is known.
    pub bytes: Option<u64>,
    /// Whose it is — **the field that decides everything below**.
    pub owner: Ownership,
    /// Whether a workspace that still exists is using it.
    ///
    /// **Armada owning a thing is not the same as Armada being finished with
    /// it.** A worktree somebody is working in right now has containers and
    /// volumes that are labelled, reclaimable in principle and in use in fact —
    /// and this project's founding premise is five of those at once. So a
    /// resource in use is offered, listed and **not ticked**: a person may still
    /// choose it, and enter on an unread screen may not.
    pub in_use: bool,
}

/// What a prune would open with ticked.
///
/// **Armada's own, and only when nothing is using them.** Everything else opens
/// unticked.
///
/// This is a function rather than a line in a render so that the rule is
/// testable in one place and cannot be re-decided by a second caller. Two things
/// keep a box empty, and they are different failures:
///
/// - **It is not Armada's.** The measured machine had 171 unlabelled volumes,
///   Armada created none of them, and one of them may be a database. A default
///   that ticked them would make the safe keypress — enter — the destructive one.
/// - **It is Armada's and in use.** A worktree somebody is working in has
///   labelled resources that are reclaimable in principle; taking them by
///   default would stop a colleague's stack mid-run, which is the exact failure
///   `clean --all`'s live-lease skip already exists to prevent.
pub fn default_ticks(candidates: &[PruneCandidate]) -> Vec<bool> {
    candidates
        .iter()
        .map(|candidate| candidate.owner == Ownership::Armada && !candidate.in_use)
        .collect()
}

/// How this run was confirmed, which is what an unlabelled resource's fate turns
/// on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmed {
    /// A person ticked this row and pressed enter, at a terminal, on this run.
    ByAPersonThisRun,
    /// A flag said yes — `--yes` in a script, or `--json` from an agent.
    ///
    /// **Sufficient for Armada's own and never for anything else.** A flag can
    /// live in a shell script or an agent's argv for ever, so treating one as
    /// consent to delete unlabelled data means the user's database is one
    /// committed line away from gone. If that makes the verb less useful in
    /// automation, that is the correct trade.
    ByAFlag,
    /// Nothing confirmed it. A preview, and nothing is removed.
    NotAtAll,
}

/// May this candidate actually be removed on this run?
///
/// **The single gate, and the only place the rule is written.** Two callers each
/// deciding it is how a `--json` path comes to prune a volume nobody agreed to.
pub fn permitted(candidate: &PruneCandidate, confirmed: Confirmed) -> bool {
    match (candidate.owner, confirmed) {
        (_, Confirmed::NotAtAll) => false,
        // Armada made it and knows what it is.
        (Ownership::Armada, _) => true,
        // A person, at a terminal, on this run. Nothing else reaches here —
        // not a flag, not `--json`, not a non-TTY.
        (Ownership::Unlabelled, Confirmed::ByAPersonThisRun) => true,
        (Ownership::Unlabelled, Confirmed::ByAFlag) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------ size parsing

    /// The spellings measured on a real machine, and the base that makes `kB`
    /// 1000 rather than 1024.
    #[test]
    fn the_units_docker_actually_emits_all_parse() {
        assert_eq!(parse_size("0B"), Some(0));
        assert_eq!(parse_size("138.3kB"), Some(138_300));
        assert_eq!(parse_size("72MB"), Some(72_000_000));
        assert_eq!(parse_size("4.171MB"), Some(4_171_000));
        assert_eq!(parse_size("12.01GB"), Some(12_010_000_000));
    }

    /// `kB` is 1000 bytes. Reading it as 1024 is a silent 2.4% error.
    #[test]
    fn kilobytes_are_base_ten_and_not_base_two() {
        assert_eq!(parse_size("1kB"), Some(1_000));
        assert_ne!(parse_size("1kB"), Some(1_024));
    }

    /// **Rule 2, inverted.** A size that will not parse must not become `0`,
    /// because `0 B` is a confident claim that there is nothing to reclaim.
    #[test]
    fn an_unreadable_size_is_unknown_and_never_zero() {
        for text in ["", "lots", "12.01ZB", "-4MB", "MB", "12.0.1GB"] {
            assert_eq!(parse_size(text), None, "{text:?} should not parse");
        }
    }

    #[test]
    fn a_size_survives_surrounding_whitespace() {
        assert_eq!(parse_size("  12.01GB  "), Some(12_010_000_000));
    }

    // ---------------------------------------------------------- size rendering

    #[test]
    fn a_size_is_printed_on_one_ladder() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(999), "999 B");
        assert_eq!(format_size(138_300), "138.3 kB");
        assert_eq!(format_size(12_010_000_000), "12.0 GB");
    }

    /// A dash in a size column reads as zero. The unknown case says so.
    #[test]
    fn an_unknown_size_says_so_rather_than_drawing_a_dash() {
        assert_eq!(format_maybe(None), "size unknown");
        assert_eq!(format_maybe(Some(0)), "0 B");
    }

    // --------------------------------------------------------- summary parsing

    /// The exact lines measured on darwin against Docker 29.6.2.
    fn measured_summary() -> &'static str {
        concat!(
            r#"{"Active":"0","Reclaimable":"4.171MB (100%)","Size":"4.171MB","TotalCount":"1","Type":"Images"}"#,
            "\n",
            r#"{"Active":"0","Reclaimable":"0B","Size":"0B","TotalCount":"0","Type":"Containers"}"#,
            "\n",
            r#"{"Active":"0","Reclaimable":"12.01GB (100%)","Size":"12.01GB","TotalCount":"171","Type":"Local Volumes"}"#,
            "\n",
            r#"{"Active":"0","Reclaimable":"0B","Size":"0B","TotalCount":"0","Type":"Build Cache"}"#,
            "\n",
        )
    }

    /// **The row that holds all the disk is spelled `Local Volumes`.** A parser
    /// keying on `volumes` finds nothing and reports a tidy machine.
    #[test]
    fn the_measured_output_parses_to_the_measured_numbers() {
        let usage = parse_summary(measured_summary());
        assert_eq!(usage.rows.len(), 4);
        let volumes = usage.row(DiskKind::Volumes).expect("`Local Volumes`");
        assert_eq!(volumes.total, 171);
        assert_eq!(volumes.active, 0);
        assert_eq!(volumes.size, Some(12_010_000_000));
        assert_eq!(volumes.reclaimable, Some(12_010_000_000));
        assert!(usage.unrecognised.is_empty());
    }

    /// The percentage rides in the same field as the size and is docker's own
    /// rounding of a ratio Armada can compute.
    #[test]
    fn the_percentage_is_stripped_off_the_reclaimable_field() {
        let usage = parse_summary(measured_summary());
        assert_eq!(
            usage.row(DiskKind::Images).unwrap().reclaimable,
            Some(4_171_000)
        );
    }

    #[test]
    fn the_machines_reclaimable_total_is_the_sum_of_its_rows() {
        let usage = parse_summary(measured_summary());
        assert_eq!(usage.reclaimable(), Some(12_010_000_000 + 4_171_000));
    }

    /// **Rule 2 again, at the total.** A sum that silently omits the volumes row
    /// reads as a complete answer and is not one.
    #[test]
    fn one_unreadable_row_makes_the_total_unknown_rather_than_smaller() {
        let usage = parse_summary(
            r#"{"Active":"0","Reclaimable":"who knows","Size":"12.01GB","TotalCount":"171","Type":"Local Volumes"}
{"Active":"0","Reclaimable":"4.171MB (100%)","Size":"4.171MB","TotalCount":"1","Type":"Images"}"#,
        );
        assert_eq!(usage.row(DiskKind::Volumes).unwrap().reclaimable, None);
        assert_eq!(usage.reclaimable(), None);
    }

    /// **Rule 3.** A type this version has no case for is disk, and dropping it
    /// under-reports for ever.
    #[test]
    fn a_type_armada_does_not_know_is_named_rather_than_dropped() {
        let usage = parse_summary(
            r#"{"Active":"0","Reclaimable":"1GB","Size":"1GB","TotalCount":"3","Type":"Snapshots"}"#,
        );
        assert!(usage.rows.is_empty());
        assert_eq!(usage.unrecognised, vec!["Snapshots".to_string()]);
    }

    /// One stray line — a warning printed above the report — must not cost the
    /// whole answer.
    #[test]
    fn a_line_that_is_not_json_is_skipped_and_the_rest_still_parses() {
        let usage = parse_summary(
            "WARNING: something\n{\"Active\":\"0\",\"Reclaimable\":\"0B\",\"Size\":\"0B\",\
             \"TotalCount\":\"0\",\"Type\":\"Containers\"}",
        );
        assert_eq!(usage.rows.len(), 1);
    }

    // --------------------------------------------------------- verbose parsing

    /// The `-v` payload is one object of arrays — a **different shape** from the
    /// line-per-row summary, on the same command with one more flag.
    #[test]
    fn the_verbose_payload_yields_names_and_sizes() {
        let found = parse_verbose_volumes(
            r#"{"Images":[],"Containers":[],"BuildCache":[],"Volumes":[
                 {"Name":"armada-a3f91c02_pgdata","Size":"79.02MB","Labels":"armada.workspace=a3f91c02"},
                 {"Name":"someone-elses","Size":"138.3kB","Labels":""}]}"#,
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "armada-a3f91c02_pgdata");
        assert_eq!(found[0].size, Some(79_020_000));
        assert_eq!(found[1].size, Some(138_300));
    }

    /// A payload that is not the expected shape yields nothing rather than
    /// panicking or inventing a zero.
    #[test]
    fn a_verbose_payload_that_changed_shape_yields_nothing() {
        assert!(parse_verbose_volumes("not json").is_empty());
        assert!(parse_verbose_volumes(r#"{"Images":[]}"#).is_empty());
    }

    // ------------------------------------------------------------------- split

    fn entry(name: &str, size: Option<u64>) -> VolumeEntry {
        VolumeEntry {
            name: name.to_string(),
            size,
        }
    }

    /// The measured shape of the user's machine: Armada's share is separate, and
    /// on that machine it was empty while everything else held 12 GB.
    #[test]
    fn the_two_shares_are_counted_apart_and_never_summed() {
        let volumes = [
            entry("armada-a3f91c02_pgdata", Some(79_020_000)),
            entry("someone-elses", Some(12_010_000_000)),
        ];
        let share = split(&volumes, &["armada-a3f91c02_pgdata".to_string()]);
        assert_eq!(share.armada_count, 1);
        assert_eq!(share.armada_bytes, Some(79_020_000));
        assert_eq!(share.other_count, 1);
        assert_eq!(share.other_bytes, Some(12_010_000_000));
    }

    /// Zero labelled volumes beside a machine full of them is the measured case,
    /// and it must read as *Armada is holding nothing* rather than as a failure.
    #[test]
    fn a_machine_whose_bloat_is_none_of_armadas_says_so() {
        let volumes = [entry("someone-elses", Some(12_010_000_000))];
        let share = split(&volumes, &[]);
        assert_eq!(share.armada_count, 0);
        assert_eq!(share.armada_bytes, Some(0));
        assert_eq!(share.other_count, 1);
    }

    #[test]
    fn one_unreadable_volume_makes_only_its_own_side_unknown() {
        let volumes = [entry("ours", None), entry("theirs", Some(5_000))];
        let share = split(&volumes, &["ours".to_string()]);
        assert_eq!(share.armada_bytes, None);
        assert_eq!(share.other_bytes, Some(5_000));
    }

    // ------------------------------------------------------------ prune safety

    fn candidate(reference: &str, owner: Ownership) -> PruneCandidate {
        PruneCandidate {
            kind: DiskKind::Volumes,
            reference: reference.to_string(),
            bytes: Some(1_000),
            owner,
            in_use: false,
        }
    }

    /// **The rule the verb exists to keep.** Armada's own open ticked; nothing
    /// else does, so the safe keypress stays the safe one.
    #[test]
    fn armadas_own_open_ticked_and_nothing_else_does() {
        let candidates = [
            candidate("armada-vol", Ownership::Armada),
            candidate("his-database", Ownership::Unlabelled),
        ];
        assert_eq!(default_ticks(&candidates), vec![true, false]);
    }

    /// **Owning a thing is not being finished with it.** Five worktrees at once
    /// is this project's premise, and a default that took a live one's volumes
    /// would stop a colleague's stack mid-run.
    #[test]
    fn armadas_own_but_in_use_opens_unticked() {
        let live = PruneCandidate {
            in_use: true,
            ..candidate("armada-live", Ownership::Armada)
        };
        assert_eq!(default_ticks(&[live]), vec![false]);
    }

    /// Inverted: a list of nothing but the user's own volumes opens with every
    /// box empty, so enter on an unread screen removes nothing.
    #[test]
    fn a_list_of_only_unlabelled_resources_opens_with_nothing_ticked() {
        let candidates: Vec<PruneCandidate> = (0..171)
            .map(|n| candidate(&format!("vol-{n}"), Ownership::Unlabelled))
            .collect();
        assert!(default_ticks(&candidates).iter().all(|ticked| !ticked));
    }

    /// **An agent must not be able to prune the user's data.** A flag — which is
    /// what `--json` and `--yes` amount to — is consent for Armada's own and for
    /// nothing else.
    #[test]
    fn a_flag_never_authorises_removing_an_unlabelled_resource() {
        let theirs = candidate("his-database", Ownership::Unlabelled);
        assert!(!permitted(&theirs, Confirmed::ByAFlag));
        assert!(!permitted(&theirs, Confirmed::NotAtAll));
        assert!(permitted(&theirs, Confirmed::ByAPersonThisRun));
    }

    #[test]
    fn armadas_own_may_go_on_a_flag_but_not_on_a_preview() {
        let ours = candidate("armada-vol", Ownership::Armada);
        assert!(permitted(&ours, Confirmed::ByAFlag));
        assert!(permitted(&ours, Confirmed::ByAPersonThisRun));
        assert!(!permitted(&ours, Confirmed::NotAtAll));
    }

    /// A preview removes nothing, whoever asked and however they asked.
    #[test]
    fn an_unconfirmed_run_removes_nothing_at_all() {
        for owner in [Ownership::Armada, Ownership::Unlabelled] {
            assert!(!permitted(&candidate("x", owner), Confirmed::NotAtAll));
        }
    }
}
