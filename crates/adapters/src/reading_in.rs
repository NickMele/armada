//! Reading a source into a Studio: what a Link's address names, and the call
//! or the file that fetches it. `#1293`, `docs/concepts/scout.md`.
//!
//! **Fleet fetches and the scout never reaches.** A scout's launch denies
//! `WebFetch`, `WebSearch` and `Bash` and comes up with no MCP server, which is
//! the confinement spike 017 measured; reading a source in widens none of it,
//! because Fleet fetches the bytes in its own process — as it already does for
//! a bare issue link in a request, [`crate::issue_lookup`] — and hands over the
//! text.
//!
//! **A session is found by this checkout's own project directory**, so another
//! repository's is not addressable rather than refused.

use std::fmt;
use std::path::{Path, PathBuf};

use adapter_traits::LookupCall;
use serde::Deserialize;

/// The forge this workspace already assumes, as its host appears in a link.
///
/// **Public so a fixture can spell one.** A test elsewhere that needs a forge
/// link would otherwise have to write the vendor's name, which is the thing
/// the gate keeps inside this crate.
pub const FORGE_HOST: &str = "github.com/";
/// Where the agent CLI keeps a session's transcript, under a person's home.
///
/// **Public so a fixture can write one**, for [`FORGE_HOST`]'s reason: a test
/// elsewhere that plants a transcript would otherwise spell the vendor's name.
pub const SESSIONS: &str = ".claude/projects";

/// The scheme Armada answers for itself, for the two sources with no address
/// anywhere else: its own Helm thread, and an agent session by id.
const OURS: &str = "armada:";
const A_SESSION: &str = "armada:session/";

/// How long Fleet waits on one fetch before the read-in fails.
pub const FETCH_SECONDS: u64 = 30;

/// The most a page fetch may return, in bytes, before `curl` stops reading.
const MOST_BYTES: u64 = 4_000_000;

/// The most issues one milestone puts on a Studio.
///
/// **A bound, because a milestone is unbounded and a Studio is laid out by
/// hand.** A hundred nodes landing at once is a board nobody can arrange.
pub const MOST_ISSUES: usize = 50;

/// What a Link's address names, where it names something a scout may be handed
/// or Fleet may read in by itself.
///
/// **Everything else is not here**, which is what makes a Link to a board or a
/// wiki stay a Link: its address is kept and its contents are not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// An issue on the forge.
    Issue {
        owner: String,
        repo: String,
        number: String,
    },
    /// A pull request on the forge.
    PullRequest {
        owner: String,
        repo: String,
        number: String,
    },
    /// A milestone on the forge, read in as its issues and no scout.
    Milestone {
        owner: String,
        repo: String,
        number: String,
    },
    /// A page on the web, at the address the person pasted and no other.
    Page { url: String },
    /// An agent session, as the transcript file to read.
    Session { file: PathBuf },
    /// This repository's Helm thread. Fleet knows where that is kept.
    Thread,
}

impl Source {
    /// The word a Studio records this source's kind as. **What it is, never
    /// whose it is** — `core_model::ScoutSourceKind` spells the same set.
    pub fn kind(&self) -> &'static str {
        match self {
            Source::Issue { .. } => "issue",
            Source::PullRequest { .. } => "pull_request",
            Source::Milestone { .. } => "milestone",
            Source::Page { .. } => "page",
            Source::Session { .. } => "session",
            Source::Thread => "thread",
        }
    }

    /// Whether reading this in is Fleet's own work rather than a scout's. A
    /// milestone is a list, and a model asked to echo one back would be cost
    /// spent on a transcription.
    pub fn read_by_fleet_alone(&self) -> bool {
        matches!(self, Source::Milestone { .. })
    }

    /// How the source is named in the sentence above its text.
    pub fn told(&self) -> String {
        match self {
            Source::Issue {
                owner,
                repo,
                number,
            } => format!("issue {number} of {owner}/{repo}, on the repository's forge"),
            Source::PullRequest {
                owner,
                repo,
                number,
            } => format!("pull request {number} of {owner}/{repo}, on the repository's forge"),
            Source::Milestone {
                owner,
                repo,
                number,
            } => format!("milestone {number} of {owner}/{repo}, on the repository's forge"),
            Source::Page { url } => format!("a web page at {url}"),
            Source::Session { .. } => {
                "an earlier agent session in this repository, as what was said in it".to_string()
            }
            Source::Thread => "this repository's own Helm thread".to_string(),
        }
    }
}

/// What `address` names, where it names a source at all.
///
/// `root` is the repository's checkout and `home` the person's home directory.
/// Between them they decide which session transcripts exist, so a session
/// belonging to another repository is not addressable.
pub fn source_of(address: &str, root: &str, home: &str) -> Option<Source> {
    let address = address.trim();
    if let Some(rest) = address.strip_prefix(OURS) {
        return match rest {
            "thread" => Some(Source::Thread),
            _ => address
                .strip_prefix(A_SESSION)
                .and_then(|id| session_file(id, root, home))
                .map(|file| Source::Session { file }),
        };
    }
    let Some((owner, repo, marker, number)) = forge_link(address) else {
        if !address.starts_with("http://") && !address.starts_with("https://") {
            return None;
        }
        // A link to anything else on the web is a page, which is a source.
        return Some(Source::Page {
            url: address.to_string(),
        });
    };
    Some(match marker {
        "issues" => Source::Issue {
            owner,
            repo,
            number,
        },
        "pull" => Source::PullRequest {
            owner,
            repo,
            number,
        },
        _ => Source::Milestone {
            owner,
            repo,
            number,
        },
    })
}

/// What a Link's address names on this repository's forge, where it names
/// anything there, and `None` for every other address — `#1379`.
///
/// **What Bridge is told, so it never reads an address itself.** Which host is
/// the forge is this crate's to know and the gate keeps the vendor's name
/// inside it, so a rule about issue links written in TypeScript or in
/// `crates/ipc` could not be written at all, let alone kept in step with
/// [`source_of`]. Fleet answers this on every Studio it puts on the wire.
///
/// **Read off the address every time, never kept on the record.** An address
/// is fixed the moment a Link is pasted, so a stored answer could only go
/// stale against the rule above it.
pub fn forge_named(address: &str) -> Option<ipc::StudioLinkForge> {
    let (_, _, marker, _) = forge_link(address.trim())?;
    Some(match marker {
        "issues" => ipc::StudioLinkForge::Issue,
        "pull" => ipc::StudioLinkForge::PullRequest,
        _ => ipc::StudioLinkForge::Milestone,
    })
}

/// [`forge_reference`] behind the scheme check, so one reading answers both
/// [`source_of`] and [`forge_named`].
fn forge_link(address: &str) -> Option<(String, String, &'static str, String)> {
    if !address.starts_with("http://") && !address.starts_with("https://") {
        return None;
    }
    forge_reference(address)
}

/// The owner, the repository, what the link names and its number.
///
/// Scans for the host literally, for [`crate::issue_lookup`]'s reason: a URL
/// parser bought for one path segment is a dependency.
fn forge_reference(address: &str) -> Option<(String, String, &'static str, String)> {
    let after_host = &address[address.find(FORGE_HOST)? + FORGE_HOST.len()..];
    let mut segments = after_host.split(['/', '?', '#', ' ', '\n']);
    let owner = segments.next()?;
    let repo = segments.next()?;
    let marker = match segments.next()? {
        // Two spellings of one pull request, and one of each of the others.
        "pull" | "pulls" => "pull",
        "issues" => "issues",
        "milestone" | "milestones" => "milestone",
        _ => return None,
    };
    let number = segments.next()?;
    if owner.is_empty() || repo.is_empty() || number.is_empty() {
        return None;
    }
    if !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((
        owner.to_string(),
        repo.to_string(),
        marker,
        number.to_string(),
    ))
}

/// The transcript file `id` names, if it is under this checkout's own project
/// directory or one of its worktrees', and `None` otherwise.
///
/// The CLI keys a project directory by the working directory it started in,
/// every character that is not a letter or a digit written as `-`. A worktree
/// lives under the checkout, so its key begins with the checkout's — which is
/// the whole of the check that a session belongs to this repository, made by
/// where the file is looked for rather than by reading it and deciding after.
fn session_file(id: &str, root: &str, home: &str) -> Option<PathBuf> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    let ours = keyed(root);
    let here = std::fs::read_dir(Path::new(home).join(SESSIONS)).ok()?;
    for entry in here.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name != ours && !name.starts_with(&format!("{ours}-")) {
            continue;
        }
        let file = entry.path().join(format!("{id}.jsonl"));
        if file.is_file() {
            return Some(file);
        }
    }
    None
}

/// A directory path as the CLI keys its project directory by.
fn keyed(path: &str) -> String {
    path.trim_end_matches('/')
        .chars()
        .map(|c| match c.is_ascii_alphanumeric() {
            true => c,
            false => '-',
        })
        .collect()
}

/// How a source's text is got.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fetch {
    /// Run these in order, and join what each printed with a newline.
    Calls(Vec<LookupCall>),
    /// Read this file.
    File(PathBuf),
    /// Fleet's own: this repository's Helm thread, wherever Fleet keeps it.
    Thread,
}

/// How to fetch `source`.
///
/// **Each call reduces its own answer to text**, with the forge's `--jq`
/// filter, so nothing here decodes a byte of a forge's JSON.
pub fn fetching(source: &Source) -> Fetch {
    match source {
        Source::Issue {
            owner,
            repo,
            number,
        } => Fetch::Calls(vec![forge_view(
            "issue",
            owner,
            repo,
            number,
            ISSUE_FIELDS,
            ISSUE,
        )]),
        Source::PullRequest {
            owner,
            repo,
            number,
        } => Fetch::Calls(vec![forge_view("pr", owner, repo, number, PR_FIELDS, PR)]),
        Source::Milestone {
            owner,
            repo,
            number,
        } => Fetch::Calls(vec![
            milestone_call(owner, repo, number),
            milestone_issues(owner, repo, number),
        ]),
        Source::Page { url } => Fetch::Calls(vec![LookupCall::rendered(
            "curl",
            vec![
                "--silent".into(),
                "--show-error".into(),
                "--location".into(),
                // Two hops is a canonical host and a trailing slash; a longer
                // chain is not the page that was pasted.
                "--max-redirs".into(),
                "2".into(),
                "--max-time".into(),
                FETCH_SECONDS.to_string(),
                "--max-filesize".into(),
                MOST_BYTES.to_string(),
                // Refused rather than handed over as the server's error page,
                // which reads to a scout as the page it asked for.
                "--fail".into(),
                url.clone(),
            ],
        )]),
        Source::Session { file } => Fetch::File(file.clone()),
        Source::Thread => Fetch::Thread,
    }
}

const ISSUE_FIELDS: &str = "number,title,state,labels,body,url";
const PR_FIELDS: &str = "number,title,state,labels,body,url,commits";

/// Plain text, built by the forge's own filter: labels and commits flattened to
/// a line each, so nothing that comes back is a JSON document.
const ISSUE: &str = r##""#" + (.number|tostring) + " " + .title + " (" + .state + ")\n" + .url + "\nLabels: " + ([.labels[].name] | join(", ")) + "\n\n" + .body"##;
const PR: &str = r##""#" + (.number|tostring) + " " + .title + " (" + .state + ")\n" + .url + "\nLabels: " + ([.labels[].name] | join(", ")) + "\nCommits:\n" + ([.commits[] | "- " + .oid[0:12] + " " + (.messageHeadline // "")] | join("\n")) + "\n\n" + .body"##;

/// The milestone's own line: its title and how many issues it holds in all.
const MILESTONE: &str = r#".title + "\t" + ((.open_issues + .closed_issues)|tostring)"#;

/// One line per issue: its address, a tab, then how it is named on a node.
const ISSUE_LINE: &str = r##".[] | select(.pull_request == null) | .html_url + "\t#" + (.number|tostring) + " " + .title + " — " + .state"##;

fn forge_view(
    what: &str,
    owner: &str,
    repo: &str,
    number: &str,
    fields: &str,
    filter: &str,
) -> LookupCall {
    LookupCall::rendered(
        "gh",
        vec![
            what.into(),
            "view".into(),
            number.into(),
            "--repo".into(),
            format!("{owner}/{repo}"),
            "--json".into(),
            fields.into(),
            "--jq".into(),
            filter.into(),
        ],
    )
}

fn milestone_call(owner: &str, repo: &str, number: &str) -> LookupCall {
    LookupCall::rendered(
        "gh",
        vec![
            "api".into(),
            format!("repos/{owner}/{repo}/milestones/{number}"),
            "--jq".into(),
            MILESTONE.into(),
        ],
    )
}

/// **Every issue, open and closed, and no pull request.** The endpoint counts a
/// pull request as an issue, and the filter is what drops them.
fn milestone_issues(owner: &str, repo: &str, number: &str) -> LookupCall {
    LookupCall::rendered(
        "gh",
        vec![
            "api".into(),
            "-X".into(),
            "GET".into(),
            format!("repos/{owner}/{repo}/issues"),
            "-F".into(),
            format!("milestone={number}"),
            "-F".into(),
            "state=all".into(),
            "-F".into(),
            "per_page=100".into(),
            "-F".into(),
            "sort=created".into(),
            "-F".into(),
            "direction=asc".into(),
            "--jq".into(),
            ISSUE_LINE.into(),
        ],
    )
}

/// One issue on a milestone, as a Studio keeps it: the address a Job is
/// dispatched from, and the line a person reads on its node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnIssue {
    pub address: String,
    pub named: String,
}

/// A milestone as it was read: what it is called, every issue that fits, and
/// how many it holds in all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneRead {
    pub title: String,
    pub issues: Vec<AnIssue>,
    /// Every issue on it, whether or not it fits in [`MilestoneRead::issues`].
    pub total: u64,
}

/// What [`fetching`] printed for a milestone: its own line, then an issue per
/// line. **Bounded at [`MOST_ISSUES`]**, and `total` is what says so.
pub fn milestone_read(printed: &str, number: &str) -> MilestoneRead {
    let mut lines = printed.lines().filter(|line| !line.trim().is_empty());
    let head = lines.next().unwrap_or_default();
    let (title, counted) = head.split_once('\t').unwrap_or((head, ""));
    let issues: Vec<AnIssue> = lines
        .filter_map(|line| line.split_once('\t'))
        .map(|(address, named)| AnIssue {
            address: address.trim().to_string(),
            named: named.trim().to_string(),
        })
        .collect();
    let total = counted.trim().parse().unwrap_or(issues.len() as u64);
    MilestoneRead {
        title: match title.trim().is_empty() {
            true => format!("Milestone {number}"),
            false => title.trim().to_string(),
        },
        issues: issues.into_iter().take(MOST_ISSUES).collect(),
        total,
    }
}

/// A page's headings and text, with its markup, scripts and styling left out.
///
/// **A reduction, not a renderer.** It drops `script` and `style` bodies whole,
/// unwraps every other tag, decodes the entities a document has to escape, and
/// collapses blank runs. What is left is prose in the document's own order.
pub fn text_of_a_page(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut rest = html;
    let mut in_tag = false;
    let mut skipping: Option<&'static str> = None;
    while let Some(next) = rest.find(['<', '>']) {
        let (before, at) = rest.split_at(next);
        if !in_tag && skipping.is_none() {
            out.push_str(before);
        }
        rest = &at[1..];
        if !at.starts_with('<') {
            in_tag = false;
            continue;
        }
        in_tag = true;
        let lowered = rest.get(..16).unwrap_or(rest).to_ascii_lowercase();
        for what in ["script", "style"] {
            if starts_a(&lowered, what) {
                skipping = match lowered.starts_with('/') {
                    true => None,
                    false => Some(what),
                };
            }
        }
        // A block that opens or closes is a line break, so prose written in
        // separate elements does not run into one sentence.
        if BREAKS.iter().any(|tag| starts_a(&lowered, tag)) {
            out.push('\n');
        }
    }
    if !in_tag && skipping.is_none() {
        out.push_str(rest);
    }
    tidied(&entities(&out))
}

/// The tags whose open or close is a line break in the text.
const BREAKS: &[&str] = &[
    "p",
    "br",
    "div",
    "li",
    "tr",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "section",
    "article",
    "header",
    "footer",
    "blockquote",
    "pre",
];

/// Whether a tag's text names `tag`, opening or closing, and not a longer name
/// that begins with it.
fn starts_a(lowered: &str, tag: &str) -> bool {
    for head in [tag.to_string(), format!("/{tag}")] {
        if let Some(rest) = lowered.strip_prefix(&head) {
            if rest.is_empty() || rest.starts_with([' ', '/', '\t', '\n', '>']) {
                return true;
            }
        }
    }
    false
}

/// The entities a document has to escape, written back as themselves.
fn entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

/// Every line trimmed, blank runs collapsed to one, the whole trimmed.
fn tidied(text: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() && lines.last().map(|last| last.is_empty()) != Some(false) {
            continue;
        }
        lines.push(line);
    }
    while lines.last().map(|last| last.is_empty()) == Some(true) {
        lines.pop();
    }
    lines.join("\n")
}

/// What was said in a session, as a transcript file holds it.
///
/// **What a person and an agent said, and nothing else.** The file also carries
/// thinking, tool calls, every result they returned and the harness's own
/// reminders; what was argued in the session is the prose, and the rest is the
/// agent's working.
pub fn text_of_a_session(transcript: &str) -> String {
    let mut said: Vec<String> = Vec::new();
    for line in transcript.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(turn) = ipc::decode::<SessionLine>("a session transcript line", line.as_bytes())
        else {
            continue;
        };
        let who = match turn.line.as_str() {
            "user" => "The person",
            "assistant" => "The agent",
            _ => continue,
        };
        let Some(message) = turn.message else {
            continue;
        };
        let text = prose(&message.content);
        if text.is_empty() {
            continue;
        }
        said.push(format!("{who}: {text}"));
    }
    said.join("\n\n")
}

/// The prose in one message: a string whole, or a list's text blocks. Thinking,
/// a tool call and a tool's result are not prose.
fn prose(content: &SaidContent) -> String {
    let blocks = match content {
        SaidContent::Prose(text) => return without_reminders(text),
        SaidContent::Blocks(blocks) => blocks,
    };
    blocks
        .iter()
        .filter_map(|block| match block {
            SaidBlock::Text { text } => Some(without_reminders(text)),
            SaidBlock::Other => None,
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// A reminder the harness put in a person's turn is the harness talking, so it
/// is not part of what was said.
fn without_reminders(text: &str) -> String {
    const OPEN: &str = "<system-reminder>";
    const CLOSE: &str = "</system-reminder>";
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(OPEN) {
        out.push_str(&rest[..at]);
        rest = match rest[at..].find(CLOSE) {
            Some(end) => &rest[at + end + CLOSE.len()..],
            None => "",
        };
    }
    out.push_str(rest);
    out.trim().to_string()
}

#[derive(Deserialize)]
struct SessionLine {
    #[serde(rename = "type")]
    line: String,
    message: Option<SaidMessage>,
}

#[derive(Deserialize)]
struct SaidMessage {
    content: SaidContent,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SaidContent {
    Prose(String),
    Blocks(Vec<SaidBlock>),
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum SaidBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

/// Text cut to `most` characters, and how many characters went.
///
/// **Reported, never silent**: a Finding that did not say what was cut would
/// claim a scout read a source whole when it read the front of one.
pub fn bounded(text: &str, most: usize) -> (String, u64) {
    let held = text.chars().count();
    if held <= most {
        return (text.to_string(), 0);
    }
    (text.chars().take(most).collect(), (held - most) as u64)
}

/// A Link whose address is not a source a scout reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaysALink {
    pub address: String,
}

impl fmt::Display for StaysALink {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "`{}` is not a source a scout reads, so it stays a Link: its address is kept and its \
             contents are not read",
            self.address
        )
    }
}
