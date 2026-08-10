use crate::{
    errors::{AppError, AppResult, ErrorCode},
    platform::paths::app_log_dir,
};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::Mutex,
    time::{Duration, SystemTime},
};
use tauri::AppHandle;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use url::Url;

const RETENTION: Duration = Duration::from_secs(30 * 24 * 60 * 60);

pub struct LogGuard {
    _guard: Mutex<Option<WorkerGuard>>,
}

impl LogGuard {
    pub fn new(guard: WorkerGuard) -> Self {
        Self {
            _guard: Mutex::new(Some(guard)),
        }
    }
}

pub fn initialize(app: &AppHandle) -> AppResult<WorkerGuard> {
    let directory = app_log_dir(app)?;
    fs::create_dir_all(&directory).map_err(|error| {
        AppError::from_io(
            ErrorCode::FilesystemWriteFailed,
            "create_log_dir",
            &directory,
            &error,
        )
    })?;
    prune_old_logs(&directory);
    let appender = tracing_appender::rolling::daily(&directory, "vsedi.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter =
        EnvFilter::try_new(std::env::var("VSEDI_LOG_LEVEL").unwrap_or_else(|_| "info".to_owned()))
            .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(writer)
                .with_ansi(false)
                .with_target(false),
        )
        .with(filter)
        .try_init()
        .map_err(|error| {
            AppError::with_detail(
                ErrorCode::InternalError,
                "アプリケーションログを初期化できませんでした。",
                "initialize_logging",
                error.to_string(),
                false,
            )
        })?;
    tracing::info!(
        operation = "initialize_logging",
        retention_days = 30,
        "application logging initialized"
    );
    Ok(guard)
}

pub fn export_diagnostic_log(app: &AppHandle, destination: &Path) -> AppResult<()> {
    let source_dir = app_log_dir(app)?;
    let mut files = fs::read_dir(&source_dir)
        .map_err(|error| {
            AppError::from_io(
                ErrorCode::FilesystemReadFailed,
                "read_log_dir",
                &source_dir,
                &error,
            )
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("vsedi.log"))
        })
        .collect::<Vec<_>>();
    files.sort();

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::from_io(
                ErrorCode::FilesystemWriteFailed,
                "create_diagnostic_log_parent",
                parent,
                &error,
            )
        })?;
    }
    let mut output = fs::File::create(destination).map_err(|error| {
        AppError::from_io(
            ErrorCode::FilesystemWriteFailed,
            "create_diagnostic_log",
            destination,
            &error,
        )
    })?;
    for file in files {
        let content = fs::read_to_string(&file).map_err(|error| {
            AppError::from_io(
                ErrorCode::FilesystemReadFailed,
                "read_log_file",
                &file,
                &error,
            )
        })?;
        writeln!(
            output,
            "===== {} =====",
            file.file_name().unwrap_or_default().to_string_lossy()
        )
        .map_err(|error| {
            AppError::from_io(
                ErrorCode::FilesystemWriteFailed,
                "write_diagnostic_log",
                destination,
                &error,
            )
        })?;
        output
            .write_all(redact_for_export(&content).as_bytes())
            .map_err(|error| {
                AppError::from_io(
                    ErrorCode::FilesystemWriteFailed,
                    "write_diagnostic_log",
                    destination,
                    &error,
                )
            })?;
        output.write_all(b"\n").map_err(|error| {
            AppError::from_io(
                ErrorCode::FilesystemWriteFailed,
                "write_diagnostic_log",
                destination,
                &error,
            )
        })?;
    }
    Ok(())
}

pub fn sanitize_text(input: &str) -> String {
    input
        .lines()
        .map(redact_sensitive_line)
        .map(sanitize_url_tokens)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn sanitize_remote_url(input: &str) -> String {
    if let Ok(mut url) = Url::parse(input) {
        let _ = url.set_username("");
        let _ = url.set_password(None);
        url.set_query(None);
        url.set_fragment(None);
        return url.to_string();
    }

    // Git's scp-like form is not a URL understood by url::Url. Remove the
    // user-info portion while preserving the repository host and path.
    if let Some(at) = input.find('@') {
        if input[at + 1..].contains(':') {
            return input[at + 1..].to_owned();
        }
    }
    input.to_owned()
}

pub fn redact_for_export(input: &str) -> String {
    let mut output = sanitize_text(input);
    for variable in ["HOME", "USERPROFILE"] {
        if let Some(home) = std::env::var_os(variable) {
            let home = home.to_string_lossy();
            if !home.is_empty() {
                output = output.replace(home.as_ref(), "~");
            }
        }
    }
    output
}

fn redact_sensitive_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let sensitive_keys = [
        "password",
        "passwd",
        "token",
        "secret",
        "private_key",
        "private-key",
        "authorization",
    ];
    let Some(key) = sensitive_keys.iter().find(|key| lower.contains(*key)) else {
        return line.to_owned();
    };
    let Some(separator) = line.find(|character| character == '=' || character == ':') else {
        return format!("{key}=[REDACTED]");
    };
    format!("{}[REDACTED]", &line[..=separator])
}

fn sanitize_url_tokens(line: String) -> String {
    line.split_whitespace()
        .map(|token| {
            if token.starts_with("http://")
                || token.starts_with("https://")
                || (token.contains('@') && token.contains(':'))
            {
                sanitize_remote_url(token)
            } else {
                token.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn prune_old_logs(directory: &Path) {
    let now = SystemTime::now();
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let is_log = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("vsedi.log"));
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if is_log && now.duration_since(modified).unwrap_or_default() > RETENTION {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{redact_for_export, sanitize_remote_url, sanitize_text};

    #[test]
    fn strips_credentials_from_urls() {
        assert_eq!(
            sanitize_remote_url("https://user:pass@example.com/org/repo.git?token=secret"),
            "https://example.com/org/repo.git"
        );
        assert_eq!(
            sanitize_remote_url("git@github.com:org/repo.git"),
            "github.com:org/repo.git"
        );
    }

    #[test]
    fn redacts_secret_key_values() {
        let output = sanitize_text("token=abc123\nnormal=visible");
        assert!(output.contains("token=[REDACTED]"));
        assert!(output.contains("normal=visible"));
    }

    #[test]
    fn export_redacts_home_directory() {
        let home = std::env::var("HOME").expect("HOME must be set for this test");
        let output = redact_for_export(&format!("{home}/project"));
        assert_eq!(output, "~/project");
    }
}
