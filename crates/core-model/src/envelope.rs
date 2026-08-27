//! The field contract every log line carries.
//!
//! Armada emits from three independent places — Fleet, Bridge and a Drone —
//! into three sinks that already exist. Nothing else guarantees a line from one
//! can be joined to a line from another. This is that guarantee.
//!
//! # Why this exists in M0 rather than when logging is written
//!
//! Retrofitting a line shape after five crates are already logging is a rewrite
//! of all five. And `actor` is the field that cannot be reconstructed
//! afterwards at all: **a line that did not record who caused it never will.**
//!
//! # What this module does not own
//!
//! Sink paths, retention and redaction. `Redactor` runs *after* an envelope is
//! assembled, never before, and nothing here is exempt from it — `fields` is
//! where a leaked credential would most plausibly land.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// A ULID.
///
/// **Lexicographic sort is chronological**, which is the whole reason it is not
/// a UUIDv4: merging three emitters' lines into one ordered view costs a string
/// sort and nothing else.
///
/// There is no constructor that mints one here, and that is deliberate.
/// **Fleet is the sole authority for the ids that name records** — `job_id` and
/// `drone_id`. Bridge and Drones echo what they were handed, which is what
/// makes the join reliable rather than best-effort: an id invented by something
/// that does not own the record joins to nothing.
///
/// `run_id` is the exception, because it names the emitter rather than a
/// record, and each process mints its own. That minting belongs to the process
/// that knows when it started, not to this crate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ulid(String);

impl Ulid {
    /// Carry an id that something else minted.
    pub fn carried(value: impl Into<String>) -> Self {
        Ulid(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// RFC3339, UTC, millisecond precision.
///
/// A newtype rather than a formatted `String`, so the one place that formats a
/// clock reading is the one place that has to agree with the rest of them.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(String);

impl Timestamp {
    pub fn from_rfc3339(value: impl Into<String>) -> Self {
        Timestamp(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Milliseconds since the epoch, or `None` where the text is not the shape
    /// this type promises.
    ///
    /// **The one reader of an instant in the workspace**, and it is here rather
    /// than beside the clock because the clock's own note says so: the moment
    /// something wants arithmetic on an instant, the type holding it is what
    /// changes. git wants a number for a commit's signature, and deriving it
    /// from the string Fleet was handed is what keeps the clock injected the
    /// whole way down.
    pub fn epoch_millis(&self) -> Option<i64> {
        let text = self.0.as_bytes();
        // `YYYY-MM-DDTHH:MM:SS` is fixed width; the fraction and the zone that
        // follow it are read by position for the same reason.
        if text.len() < 19 {
            return None;
        }
        let field = |from: usize, to: usize| -> Option<i64> {
            core::str::from_utf8(&text[from..to]).ok()?.parse().ok()
        };
        let millis = match text.get(19) {
            Some(b'.') if text.len() >= 23 => field(20, 23)?,
            Some(b'.') => return None,
            _ => 0,
        };
        let day = days_from_civil(field(0, 4)?, field(5, 7)?, field(8, 10)?);
        let seconds = day * 86_400 + field(11, 13)? * 3_600 + field(14, 16)? * 60 + field(17, 19)?;
        Some(seconds * 1_000 + millis)
    }
}

/// Howard Hinnant's `days_from_civil`, the exact inverse of the
/// `civil_from_days` Fleet's clock formats with. Carried rather than depended
/// on, for the reason that one is: the whole of it is below.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    // March-based year, so the leap day falls at the end of a cycle.
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Lowercase, always.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    /// The spelling a sink writes. Here rather than at the sink, so three
    /// emitters cannot disagree about the case of `warn`.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Level::Trace => "trace",
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

/// Emitter identity. **A closed set** — a new emitter is a decision, not a
/// string somebody passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Component {
    Fleet,
    BridgeMain,
    BridgeUi,
    Drone,
}

impl Component {
    /// The spelling a sink writes, hyphenated as the contract's table has it.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Component::Fleet => "fleet",
            Component::BridgeMain => "bridge-main",
            Component::BridgeUi => "bridge-ui",
            Component::Drone => "drone",
        }
    }
}

/// Who caused the line. Three ways, and the separation is baked in now because
/// adding it later is a schema migration over recorded history.
///
/// Verification source is orthogonal to this and is null for a human.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    Human,
    Fleet,
    Drone,
}

impl Actor {
    /// Every variant. **`domain/` has no row for this one** — the actor
    /// vocabulary is the log envelope's, not the registry's, so the order is
    /// the enum's own and there is no key to be verbatim against.
    pub const ALL: &'static [Actor] = &[Actor::Human, Actor::Fleet, Actor::Drone];

    /// The wire value. `job_events.actor` and `scope_revisions[].approved_by`
    /// are both stored from here, so the two spellings cannot diverge.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Actor::Human => "human",
            Actor::Fleet => "fleet",
            Actor::Drone => "drone",
        }
    }

    /// Read a stored value back. `None` where it is not one of the three,
    /// which is a row written by something that did not share this enum.
    pub fn from_wire(value: &str) -> Option<Actor> {
        Actor::ALL.iter().copied().find(|a| a.as_wire() == value)
    }
}

/// A value inside [`Envelope::fields`].
///
/// Deliberately small. `fields` is for structured data, and a type that can
/// hold anything is a type that ends up holding a formatted sentence.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<FieldValue>),
}

/// One line, from any emitter.
///
/// # The absent-versus-null rule
///
/// A key is either present with a value or absent. **Never present and null.**
/// Every conditional field below is an `Option`, and a sink writing one must
/// omit the key rather than write a null — because `workspace` being absent is
/// itself the signal that a line is Job-scoped rather than workspace-scoped,
/// and a null would say something different from nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Envelope {
    // Always present. These four are constructor arguments rather than fields
    // you can forget, so "always" is a fact about the type.
    ts: Timestamp,
    level: Level,
    component: Component,
    /// The **emitting** process's instance, minted by that process at start.
    ///
    /// Fleet's changes on every restart, which is what makes a restart visible
    /// in the log without anything announcing it. Bridge and a Drone mint their
    /// own — a Drone outlives a Fleet restart under `setsid`, and Bridge runs
    /// before it has reached Fleet at all, so one Fleet-owned value could not
    /// describe either. It is the one id an emitter mints for itself; `job_id`
    /// and `drone_id` name records Fleet owns and are only ever echoed.
    run_id: Ulid,
    /// Never carries an interpolated id. Ids are fields, and any query targets a
    /// field — nothing greps `msg`. Same discipline as `store` being the only
    /// crate that deserializes: the structured path is the only path.
    msg: String,

    // Present under a stated condition, absent otherwise.
    job_id: Option<Ulid>,
    /// A retry is a second `drone_id` under one `job_id`.
    drone_id: Option<Ulid>,
    /// From the WorkflowDef, never generated.
    step_id: Option<String>,
    /// **Single-valued, and omitted when the line is not scoped to one
    /// workspace.** A Convoy-spanning line carries `job_id` and no `workspace`;
    /// the full set a Job spans is recorded once at Job creation and persisted
    /// in `job_manifests`.
    ///
    /// An array was rejected: it taxes every query on a hot-path field forever,
    /// and most lines concern one workspace or none.
    workspace: Option<String>,
    /// `fleet` only. Supplied by its tracing layer.
    target: Option<String>,
    /// `fleet` only. Supplied by its tracing layer.
    span: Option<String>,
    /// All structured data. **Nothing structured belongs at the top level.**
    fields: BTreeMap<String, FieldValue>,
}

impl Envelope {
    /// The four fields that are always present, and nothing else.
    pub fn new(
        ts: Timestamp,
        level: Level,
        component: Component,
        run_id: Ulid,
        msg: impl Into<String>,
    ) -> Self {
        Envelope {
            ts,
            level,
            component,
            run_id,
            msg: msg.into(),
            job_id: None,
            drone_id: None,
            step_id: None,
            workspace: None,
            target: None,
            span: None,
            fields: BTreeMap::new(),
        }
    }

    /// The correlation spine.
    pub fn in_job(mut self, job_id: Ulid) -> Self {
        self.job_id = Some(job_id);
        self
    }

    pub fn by_drone(mut self, drone_id: Ulid) -> Self {
        self.drone_id = Some(drone_id);
        self
    }

    pub fn at_step(mut self, step_id: impl Into<String>) -> Self {
        self.step_id = Some(step_id.into());
        self
    }

    /// Scope the line to exactly one workspace. A line that spans several does
    /// not call this.
    pub fn in_workspace(mut self, workspace: impl Into<String>) -> Self {
        self.workspace = Some(workspace.into());
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: FieldValue) -> Self {
        self.fields.insert(key.into(), value);
        self
    }

    pub fn ts(&self) -> &Timestamp {
        &self.ts
    }
    pub fn level(&self) -> Level {
        self.level
    }
    pub fn component(&self) -> Component {
        self.component
    }
    pub fn run_id(&self) -> &Ulid {
        &self.run_id
    }
    pub fn msg(&self) -> &str {
        &self.msg
    }
    pub fn job_id(&self) -> Option<&Ulid> {
        self.job_id.as_ref()
    }
    pub fn drone_id(&self) -> Option<&Ulid> {
        self.drone_id.as_ref()
    }
    pub fn step_id(&self) -> Option<&str> {
        self.step_id.as_deref()
    }
    pub fn workspace(&self) -> Option<&str> {
        self.workspace.as_deref()
    }
    pub fn fields(&self) -> &BTreeMap<String, FieldValue> {
        &self.fields
    }
}

/// The audit sink's line: an envelope plus who caused it.
///
/// `actor` lives here rather than on [`Envelope`] because only one of the three
/// sinks carries it, and a field that is meaningful in one place and absent in
/// two is a field people forget to set in the two.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditLine {
    pub envelope: Envelope,
    pub actor: Actor,
}

/// The names a Drone receives its spine under.
///
/// Set in the spawn configuration, so hooks and transcripts carry the join
/// without anything parsing a prompt.
pub mod env_keys {
    pub const JOB_ID: &str = "ARMADA_JOB_ID";
    pub const DRONE_ID: &str = "ARMADA_DRONE_ID";
    pub const STEP_ID: &str = "ARMADA_STEP_ID";
}

#[cfg(test)]
mod tests {
    use super::Timestamp;

    fn millis(text: &str) -> Option<i64> {
        Timestamp::from_rfc3339(text).epoch_millis()
    }

    #[test]
    fn the_epoch_itself_is_zero() {
        assert_eq!(millis("1970-01-01T00:00:00.000Z"), Some(0));
    }

    #[test]
    fn a_reading_carries_its_milliseconds() {
        assert_eq!(millis("1970-01-01T00:00:01.234Z"), Some(1_234));
    }

    #[test]
    fn the_fraction_is_optional_because_the_zone_may_follow_the_seconds() {
        assert_eq!(millis("1970-01-02T00:00:00Z"), Some(86_400_000));
    }

    #[test]
    fn it_is_the_inverse_of_the_formatting_fleet_does() {
        // Each is a case that goes wrong in hand-written date arithmetic: a
        // leap day, a century that is not a leap year, and the last
        // millisecond of a year.
        for (text, expected) in [
            ("2024-02-29T12:00:00.000Z", 1_709_208_000_000),
            ("1900-03-01T00:00:00.000Z", -2_203_891_200_000),
            ("2026-12-31T23:59:59.999Z", 1_798_761_599_999),
        ] {
            assert_eq!(millis(text), Some(expected), "{text}");
        }
    }

    #[test]
    fn text_that_is_not_an_instant_is_none_rather_than_a_guess() {
        for wrong in ["", "yesterday", "2026-08-23", "2026-08-23T12:00:00.Z"] {
            assert_eq!(millis(wrong), None, "{wrong}");
        }
    }
}
