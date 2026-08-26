//! `WireError` — a failure, once it has stopped being a cause chain.
//!
//! `docs/contracts/error-contract.md` is the authority here, and the shape
//! below is that contract's table. Five fields are always present, three are
//! present when they apply, and **`level` and `component` are not among either
//! set: this is an error, not a log line.**
//!
//! # What is not here yet
//!
//! `From<ArmadaError>` — the conversion this type exists for — because
//! `core-model` has no `ArmadaError` to convert from. When it lands, that impl
//! belongs here, beside the type it flattens into, and it is the one place a
//! traversable cause chain becomes an array of strings.
//!
//! No code is declared in this crate either. A code's declaration stays beside
//! the variant that raises it, and `cargo xtask verify-error-codes` collects
//! them; a code invented here would be in no manifest and mean nothing to the
//! lookup Bridge does.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{DroneId, JobId, StepId};

/// The process instance that emitted the error.
///
/// **Not Fleet's, necessarily.** Each process mints its own at start, so an
/// error raised inside Bridge before it ever reached Fleet carries a real id
/// rather than nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(String);

impl RunId {
    pub fn carried(value: impl Into<String>) -> Self {
        RunId(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A failure, flattened for the wire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireError {
    /// Opaque to Bridge: looked up, never parsed. An unknown code and a
    /// withdrawn code are the same event to a client, and both render.
    pub code: String,
    /// What renders when the lookup misses.
    pub message: String,
    pub run_id: RunId,
    /// May be empty. **Never absent**, so the fallback always has somewhere to
    /// read.
    pub fields: BTreeMap<String, WireValue>,
    /// The cause chain, flattened. May be a single entry. This is the step that
    /// costs: a traversable chain cannot cross a wire, so everything downstream
    /// of here reads strings.
    pub chain: Vec<String>,
    /// Absent on a failure that precedes a Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<JobId>,
    /// A retry is a second Drone id under one Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drone_id: Option<DroneId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<StepId>,
}

impl WireError {
    /// The five guaranteed fields, and nothing else. `fields` and `chain` start
    /// empty and present rather than absent.
    pub fn raised(code: impl Into<String>, message: impl Into<String>, run_id: RunId) -> WireError {
        WireError {
            code: code.into(),
            message: message.into(),
            run_id,
            fields: BTreeMap::new(),
            chain: Vec::new(),
            job_id: None,
            drone_id: None,
            step_id: None,
        }
    }

    /// The chain, outermost first.
    pub fn caused_by(mut self, chain: Vec<String>) -> WireError {
        self.chain = chain;
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: WireValue) -> WireError {
        self.fields.insert(key.into(), value);
        self
    }

    /// The Job this failure is about, once one exists.
    pub fn about_job(mut self, job_id: JobId) -> WireError {
        self.job_id = Some(job_id);
        self
    }
}

/// A value inside [`WireError::fields`].
///
/// Untagged, so a field is a JSON primitive rather than an object wrapping one
/// — the shape a TypeScript union reads directly.
///
/// Deliberately small, and it mirrors the log envelope's `FieldValue` variant
/// for variant: **there is no variant that can hold a `Secret<T>`**, so a
/// credential cannot be put into one without failing to compile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WireValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<WireValue>),
}

impl From<&core_model::FieldValue> for WireValue {
    fn from(value: &core_model::FieldValue) -> WireValue {
        match value {
            core_model::FieldValue::Str(text) => WireValue::Str(text.clone()),
            core_model::FieldValue::Int(number) => WireValue::Int(*number),
            core_model::FieldValue::Float(number) => WireValue::Float(*number),
            core_model::FieldValue::Bool(flag) => WireValue::Bool(*flag),
            core_model::FieldValue::List(values) => {
                WireValue::List(values.iter().map(WireValue::from).collect())
            }
        }
    }
}
