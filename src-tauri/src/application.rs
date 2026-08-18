//! UI-independent application facade.
//!
//! The Tauri commands and the Slint shell both call this module. Keeping the
//! operation names and DTOs here makes the migration boundary explicit: UI
//! frameworks may change, while Git and project safety rules stay in Rust.

use crate::{
    errors::AppResult,
    models::{EnvironmentDiagnostic, ProjectDiagnostic, VpmTrackingPolicy},
    services,
};

pub fn inspect_environment() -> AppResult<EnvironmentDiagnostic> {
    services::diagnostics::inspect_environment()
}

pub fn inspect_project(
    path: &str,
    vpm_tracking_policy: VpmTrackingPolicy,
) -> AppResult<ProjectDiagnostic> {
    services::projects::inspect_project(path, vpm_tracking_policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_facade_uses_the_service_layer() {
        let diagnostic = inspect_environment().expect("environment inspection should run");
        assert!(!diagnostic.platform.os.is_empty());
    }
}
