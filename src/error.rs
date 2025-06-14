#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidMetadata,
    FileNotFound,
    FileNameTooLong,
    FileNameInvalidUtf8,
    FailedIO,
}
