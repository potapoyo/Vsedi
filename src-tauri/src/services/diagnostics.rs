use crate::{
    errors::AppResult,
    git::diagnostics as git_diagnostics,
    models::{EnvironmentDiagnostic, PlatformDiagnostic},
};

pub fn inspect_environment() -> AppResult<EnvironmentDiagnostic> {
    Ok(EnvironmentDiagnostic {
        platform: PlatformDiagnostic {
            os: std::env::consts::OS.to_owned(),
            architecture: std::env::consts::ARCH.to_owned(),
            supported: cfg!(windows) || cfg!(all(target_os = "macos", target_arch = "aarch64")),
        },
        git: git_diagnostics::inspect()?,
    })
}
