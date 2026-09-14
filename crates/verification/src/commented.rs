//! Whether a line of a patch is a comment, read against the file it is in.
//!
//! **A comment is never an assertion**, so a test-content flag citing one is
//! not raised — decided here after the answer, at no cost. The marker is read
//! against the file's language because `#` opens a comment in Python and an
//! attribute in Rust. **A language not named here gets only the markers no
//! language writes code with**, so an unknown file is flagged, not excused.

/// Languages whose line comments open with `#`.
const HASH: &[&str] = &[
    "py", "rb", "sh", "bash", "zsh", "yml", "yaml", "toml", "pl", "r", "ex", "exs", "jl", "tf",
    "ps1", "cmake", "mk",
];

/// Files named without an extension whose comments open with `#`.
const HASH_NAMES: &[&str] = &["makefile", "gnumakefile", "gemfile", "rakefile"];

/// Languages whose line comments open with `--`.
const DASHES: &[&str] = &["sql", "lua", "hs", "elm"];

/// Languages whose line comments open with `;`.
const SEMICOLON: &[&str] = &["clj", "cljs", "el", "lisp", "scm", "rkt"];

/// Whether `text`, one line of `path` without its diff marker, is a comment.
///
/// **A line, not a span.** The inside of a block comment opened on an earlier
/// line reads as code here; that errs towards raising the flag.
pub(crate) fn is_comment(path: &str, text: &str) -> bool {
    let line = text.trim_start();
    // No language writes a statement opening with these.
    if ["//", "/*", "*/", "<!--"]
        .iter()
        .any(|marker| line.starts_with(marker))
    {
        return true;
    }
    let lower = path.to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    let extension = name.rsplit_once('.').map_or("", |(_, extension)| extension);
    (line.starts_with('#') && (HASH.contains(&extension) || HASH_NAMES.contains(&name)))
        || (line.starts_with("--") && DASHES.contains(&extension))
        || (line.starts_with(';') && SEMICOLON.contains(&extension))
}
