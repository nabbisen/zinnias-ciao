//! Error conversion helpers for the SSR worker.

use zinnias_ciao_contracts::AppError;

/// Convert an `AppError` into a `worker::Error` for use with `?` in handlers.
/// The user_message is safe to include in a log; it never contains secrets.
pub fn to_worker_err(e: AppError) -> worker::Error {
    worker::Error::RustError(e.user_message.to_string())
}

/// Map any Result<T, AppError> into Result<T, worker::Error>.
pub trait IntoWorkerResult<T> {
    fn wk(self) -> worker::Result<T>;
}

impl<T> IntoWorkerResult<T> for Result<T, AppError> {
    fn wk(self) -> worker::Result<T> {
        self.map_err(to_worker_err)
    }
}
