/// The state of an HTTP session as visible to the application layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// No valid session cookie present.
    Anonymous,
    /// Session exists and has not expired or been revoked.
    Authenticated,
    /// Session record found but past its `expires_at` / revoked.
    Expired,
}
