use crate::models::LogSnapshot;
use crate::{
    errors::{AppError, AppResult, ErrorCode},
    platform::paths::app_log_dir,
};
use std::{
    collections::VecDeque,
    fs,
    io::Write,
    path::Path,
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime},
};
use tauri::AppHandle;
use tracing_appender::non_blocking::WorkerGuard;
#[cfg(not(feature = "native-ui-test"))]
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, reload, EnvFilter};
use url::Url;

const RETENTION: Duration = Duration::from_secs(30 * 24 * 60 * 60);
pub const LOG_LEVELS: [&str; 5] = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

type LogFilterUpdater = Box<dyn Fn(EnvFilter) -> Result<(), String> + Send + Sync>;
static LOG_FILTER_UPDATER: OnceLock<Mutex<Option<LogFilterUpdater>>> = OnceLock::new();

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
    let configured_level = std::env::var("VSEDI_LOG_LEVEL").unwrap_or_else(|_| "INFO".to_owned());
    let level = normalize_log_level(&configured_level).unwrap_or("INFO");
    let filter = EnvFilter::new(level.to_ascii_lowercase());
    let (filter_layer, filter_handle) = reload::Layer::new(filter);
    let subscriber = tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(writer)
                .with_ansi(false)
                .with_target(false),
        )
        .with(filter_layer);
    #[cfg(feature = "native-ui-test")]
    let initialization = tracing::subscriber::set_global_default(subscriber);
    #[cfg(not(feature = "native-ui-test"))]
    let initialization = subscriber.try_init();
    initialization.map_err(|error| {
        AppError::with_detail(
            ErrorCode::InternalError,
            "アプリケーションログを初期化できませんでした。",
            "initialize_logging",
            error.to_string(),
            false,
        )
    })?;
    let updater: LogFilterUpdater = Box::new(move |filter| {
        filter_handle
            .reload(filter)
            .map_err(|error| error.to_string())
    });
    *LOG_FILTER_UPDATER
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| {
            AppError::simple(
                ErrorCode::InternalError,
                "ログ設定を更新できません。",
                "initialize_logging",
            )
        })? = Some(updater);
    tracing::info!(
        operation = "initialize_logging",
        retention_days = 30,
        log_level = level,
        "application logging initialized"
    );
    Ok(guard)
}

pub fn normalize_log_level(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_uppercase().as_str() {
        "ERROR" => Some("ERROR"),
        "WARN" | "WARNING" => Some("WARN"),
        "INFO" => Some("INFO"),
        "DEBUG" => Some("DEBUG"),
        "TRACE" => Some("TRACE"),
        _ => None,
    }
}

pub fn set_log_level(value: &str) -> AppResult<&'static str> {
    let level = normalize_log_level(value).ok_or_else(|| {
        AppError::simple(
            ErrorCode::SettingsInvalidLogLevel,
            "ログレベルは ERROR / WARN / INFO / DEBUG / TRACE のいずれかを指定してください。",
            "set_log_level",
        )
    })?;
    let filter = EnvFilter::new(level.to_ascii_lowercase());
    if let Some(updater) = LOG_FILTER_UPDATER
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| {
            AppError::simple(
                ErrorCode::InternalError,
                "ログ設定を更新できません。",
                "set_log_level",
            )
        })?
        .as_ref()
    {
        updater(filter).map_err(|error| {
            AppError::with_detail(
                ErrorCode::InternalError,
                "ログレベルを適用できませんでした。",
                "set_log_level",
                error.to_string(),
                false,
            )
        })?;
    }
    Ok(level)
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

pub fn read_recent_logs(app: &AppHandle, max_lines: usize) -> AppResult<LogSnapshot> {
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

    let current_file = files
        .last()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned());
    let unlimited = max_lines == usize::MAX;
    let mut lines = VecDeque::new();
    for file in files {
        let content = fs::read_to_string(&file).map_err(|error| {
            AppError::from_io(
                ErrorCode::FilesystemReadFailed,
                "read_log_file",
                &file,
                &error,
            )
        })?;
        for line in redact_for_export(&content).lines() {
            if max_lines == 0 && !unlimited {
                continue;
            }
            lines.push_back(line.to_owned());
            if !unlimited && lines.len() > max_lines {
                lines.pop_front();
            }
        }
    }

    Ok(LogSnapshot {
        lines: lines.into_iter().collect(),
        current_file,
    })
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
    let Some(separator) = line.find(['=', ':']) else {
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
    use super::{normalize_log_level, redact_for_export, sanitize_remote_url, sanitize_text};

    #[test]
    fn accepts_supported_log_levels_case_insensitively() {
        assert_eq!(normalize_log_level("trace"), Some("TRACE"));
        assert_eq!(normalize_log_level(" warning "), Some("WARN"));
        assert_eq!(normalize_log_level("INFO"), Some("INFO"));
        assert_eq!(normalize_log_level("verbose"), None);
    }

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
