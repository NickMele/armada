//! A value that cannot be printed.

/// A credential.
///
/// **`Secret<T>` has no `Debug`, no `Display` and no `Serialize`.** That is the
/// whole design: `format!("{:?}", s)` fails to compile rather than logging a
/// credential, and the property cascades — a struct deriving `Debug` while
/// holding one does not compile either, so the mistake is caught at the place
/// that makes it.
///
/// This is the pattern the rest of the codebase follows: a narrow capability
/// type where the wrong call is unavailable at the call site, rather than a
/// broad type used correctly by convention. Every v1 failure was a convention
/// failure.
///
/// It is not a defence against a subprocess echoing a credential into its own
/// stderr, where the value is a plain string and the type system is out of the
/// picture. That is the `Redactor`'s job, at three sinks.
pub struct Secret<T>(T);

impl<T> Secret<T> {
    pub fn new(value: T) -> Self {
        Secret(value)
    }

    /// Read the value. Named so that every place a credential escapes the type
    /// is greppable in one search.
    pub fn expose(&self) -> &T {
        &self.0
    }
}
