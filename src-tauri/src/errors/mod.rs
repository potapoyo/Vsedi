use serde::{Deserialize, Serialize};
use std::{fmt, io, path::Path};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    EnvGitVersionFailed,
    ProjectNotFound,
    ProjectPermissionDenied,
    ProjectInvalidUnity,
    ProjectUnsupportedKind,
    SettingsReadFailed,
    SettingsWriteFailed,
    SettingsInvalidJson,
    SettingsInvalidLogLevel,
    SettingsUnsupportedSchema,
    SettingsBackupFailed,
    FilesystemReadFailed,
    FilesystemWriteFailed,
    PermissionDenied,
    RepositoryInvalid,
    WorktreeReadFailed,
    RepositoryStateChanged,
    RepositoryInitializeFailed,
    IgnoreRulesApplyFailed,
    SaveMemoInvalid,
    SaveNoChanges,
    SaveConflict,
    SaveExistingStagedChanges,
    SaveAddFailed,
    SaveCommitFailed,
    HistoryReadFailed,
    DiffReadFailed,
    InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub technical_detail: Option<String>,
    pub operation: Option<String>,
    pub may_have_mutated: bool,
}

impl AppError {
    pub fn new(
        code: ErrorCode,
        message: impl Into<String>,
        operation: Option<impl Into<String>>,
        technical_detail: Option<impl Into<String>>,
        may_have_mutated: bool,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            technical_detail: technical_detail.map(Into::into),
            operation: operation.map(Into::into),
            may_have_mutated,
        }
    }

    pub fn simple(code: ErrorCode, message: impl Into<String>, operation: &'static str) -> Self {
        Self::new(
            code,
            message,
            Some(operation),
            Option::<String>::None,
            false,
        )
    }

    pub fn with_detail(
        code: ErrorCode,
        message: impl Into<String>,
        operation: &'static str,
        detail: impl Into<String>,
        may_have_mutated: bool,
    ) -> Self {
        Self::new(
            code,
            message,
            Some(operation),
            Some(detail),
            may_have_mutated,
        )
    }

    pub fn from_io(
        code: ErrorCode,
        operation: &'static str,
        path: &Path,
        error: &io::Error,
    ) -> Self {
        let detail = format!("{}: {}", path.display(), error);
        let code = if error.kind() == io::ErrorKind::PermissionDenied {
            ErrorCode::PermissionDenied
        } else {
            code
        };
        Self::with_detail(
            code,
            "ファイル操作に失敗しました。",
            operation,
            detail,
            false,
        )
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({:?})", self.message, self.code)
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = Result<T, AppError>;
