//! The verbs, and the one place their answers are rendered.
//!
//! Every command is **parse args → call into the core → render**
//! (`ARCHITECTURE.md` §1.3). What lives here is the sequencing of adapter calls
//! — the shell attempting what the core proposed — and nothing that decides
//! anything on its own.

pub mod check;
pub mod clean;
pub mod config;
pub mod dispatch;
pub mod doctor;
pub mod fleet;
pub mod guild;
pub mod init;
pub mod machine;
pub mod preflight;
pub mod services;
pub mod skills;
pub mod status;

use armada_core::config::{self as config_contract, ResolvedConfig};
use armada_core::ctx::{Clock, Fetch, Run};
use armada_core::envelope::{
    AnswerData, BoardData, CheckData, CheckDryRun, CleanData, CleanDryRun, DispatchData,
    DoctorData, Envelope, FleetLsData, GuildBundleData, GuildInitData, GuildSyncData, InboxData,
    InitData, InitDryRun, KillData, MachineInitData, ScanData, ServicesData, SkillsData, SpawnData,
    StatusData, UpDryRun, VerifyData,
};
use armada_core::workspace::Workspace;
use armada_manifest::config_file;
use armada_manifest::machine::MachineConfig;

use crate::app::App;

/// One verb's answer, so the renderer and the exit path exist once.
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    /// `armada manifest init`.
    Init(Box<Envelope<InitData>>),
    /// `armada manifest init --dry-run`.
    InitDryRun(Box<Envelope<InitDryRun>>),
    /// `armada manifest clean`.
    Clean(Box<Envelope<CleanData>>),
    /// `armada manifest clean --dry-run`.
    CleanDryRun(Box<Envelope<CleanDryRun>>),
    /// `armada manifest up`.
    Up(Box<Envelope<ServicesData>>),
    /// `armada manifest up --dry-run`.
    UpDryRun(Box<Envelope<UpDryRun>>),
    /// `armada manifest down`.
    Down(Box<Envelope<ServicesData>>),
    /// `armada manifest status`.
    Status(Box<Envelope<StatusData>>),
    /// `armada manifest check`.
    Check(Box<Envelope<CheckData>>),
    /// `armada manifest check --dry-run`.
    CheckDryRun(Box<Envelope<CheckDryRun>>),
    /// A dispatched `commands:` entry.
    Dispatch(Box<Envelope<DispatchData>>),
    /// `armada manifest config scan`.
    Scan(Box<Envelope<ScanData>>),
    /// `armada manifest config verify`.
    Verify(Box<Envelope<VerifyData>>),
    /// `armada manifest skills`, or `skills show <name>`.
    Skills(Box<Envelope<SkillsData>>),
    /// `armada init` — the machine, not a workspace.
    MachineInit(Box<Envelope<MachineInitData>>),
    /// `armada doctor`.
    Doctor(Box<Envelope<DoctorData>>),
    /// `armada guild pull` and `armada guild push`.
    GuildSync(Box<Envelope<GuildSyncData>>),
    /// `armada guild init`.
    GuildInit(Box<Envelope<GuildInitData>>),
    /// `armada guild export` and `armada guild import`.
    GuildBundle(Box<Envelope<GuildBundleData>>),
    /// `armada fleet spawn`.
    Spawn(Box<Envelope<SpawnData>>),
    /// `armada fleet ls`.
    FleetLs(Box<Envelope<FleetLsData>>),
    /// `armada fleet board`.
    Board(Box<Envelope<BoardData>>),
    /// `armada fleet kill`.
    Kill(Box<Envelope<KillData>>),
    /// `armada fleet inbox`.
    Inbox(Box<Envelope<InboxData>>),
    /// `armada fleet answer`.
    Answer(Box<Envelope<AnswerData>>),
}

impl Output {
    /// The envelope, rendered.
    pub fn to_json(&self) -> String {
        match self {
            Output::Init(e) => e.to_json(),
            Output::InitDryRun(e) => e.to_json(),
            Output::Clean(e) => e.to_json(),
            Output::CleanDryRun(e) => e.to_json(),
            Output::Up(e) => e.to_json(),
            Output::UpDryRun(e) => e.to_json(),
            Output::Down(e) => e.to_json(),
            Output::Status(e) => e.to_json(),
            Output::Check(e) => e.to_json(),
            Output::CheckDryRun(e) => e.to_json(),
            Output::Dispatch(e) => e.to_json(),
            Output::Scan(e) => e.to_json(),
            Output::Verify(e) => e.to_json(),
            Output::Skills(e) => e.to_json(),
            Output::MachineInit(e) => e.to_json(),
            Output::Doctor(e) => e.to_json(),
            Output::GuildSync(e) => e.to_json(),
            Output::GuildInit(e) => e.to_json(),
            Output::GuildBundle(e) => e.to_json(),
            Output::Spawn(e) => e.to_json(),
            Output::FleetLs(e) => e.to_json(),
            Output::Board(e) => e.to_json(),
            Output::Kill(e) => e.to_json(),
            Output::Inbox(e) => e.to_json(),
            Output::Answer(e) => e.to_json(),
        }
    }

    /// The process exit code.
    ///
    /// **A dispatched child's code passes through verbatim and is not
    /// remapped.** Armada did not decide the outcome, so it does not get to
    /// classify it — and scripts return meaningful codes their own callers
    /// already depend on. The ambiguity that creates against Armada's own `1`–`6`
    /// is resolved by the envelope rather than by renumbering: **Armada's own
    /// error codes can only occur when the child never ran**, and
    /// `data.dispatched` says which happened.
    pub fn exit_code(&self) -> u8 {
        match self {
            Output::Dispatch(envelope) if envelope.data.dispatched => {
                match envelope.data.child_exit {
                    Some(code) => code as u8,
                    // Killed by a signal: the shell convention, and the same
                    // carve-out `ARCHITECTURE.md` §1.6 makes for Armada's own
                    // 130 and 141.
                    None => 128,
                }
            }
            Output::Init(e) => e.exit_code(),
            Output::InitDryRun(e) => e.exit_code(),
            Output::Clean(e) => e.exit_code(),
            Output::CleanDryRun(e) => e.exit_code(),
            Output::Up(e) => e.exit_code(),
            Output::UpDryRun(e) => e.exit_code(),
            Output::Down(e) => e.exit_code(),
            Output::Status(e) => e.exit_code(),
            Output::Check(e) => e.exit_code(),
            Output::CheckDryRun(e) => e.exit_code(),
            Output::Dispatch(e) => e.exit_code(),
            Output::Scan(e) => e.exit_code(),
            Output::Verify(e) => e.exit_code(),
            Output::Skills(e) => e.exit_code(),
            Output::MachineInit(e) => e.exit_code(),
            Output::Doctor(e) => e.exit_code(),
            Output::GuildSync(e) => e.exit_code(),
            Output::GuildInit(e) => e.exit_code(),
            Output::GuildBundle(e) => e.exit_code(),
            Output::Spawn(e) => e.exit_code(),
            Output::FleetLs(e) => e.exit_code(),
            Output::Board(e) => e.exit_code(),
            Output::Kill(e) => e.exit_code(),
            Output::Inbox(e) => e.exit_code(),
            Output::Answer(e) => e.exit_code(),
        }
    }
}

/// Read, parse and resolve this workspace's `armada.yml`.
///
/// **Loading is not verifying.** No schema runs here: the structs reject
/// anything they cannot turn into a typed value, and everything needing a
/// second part of the document or the filesystem is `armada manifest config verify`'s
/// (PLAN.md §4.1.1).
pub fn load_config<R: Run, C: Clock, F: Fetch>(
    app: &App<R, C, F>,
) -> Result<(Workspace, ResolvedConfig), armada_core::error::ArmadaError> {
    let workspace = app.ctx.workspace()?.clone();
    let text = config_file::read(&workspace.config_path())?;
    let parsed = config_contract::parse(&text, &workspace.config_label)?;
    let resolved = config_contract::resolve(
        parsed,
        &app.machine.config_defaults(),
        &workspace.config_label,
    )?;
    Ok((workspace, resolved))
}

/// Read another workspace's config, given its path — for `clean --project` and
/// `clean --all`, which act on directories that are not this one.
///
/// Returns `None` rather than failing: a sibling whose config no longer parses
/// is still a workspace whose ports and containers must be reclaimed, and
/// refusing to clean it because of a syntax error in a file Armada is not going
/// to execute would strand exactly the resources this verb exists to release.
pub fn load_foreign_config(
    root: &std::path::Path,
    machine: &MachineConfig,
) -> Option<ResolvedConfig> {
    let text = std::fs::read_to_string(root.join("armada.yml")).ok()?;
    let parsed = config_contract::parse(&text, "armada.yml").ok()?;
    config_contract::resolve(parsed, &machine.config_defaults(), "armada.yml").ok()
}
