use crate::{
    errors::{AppError, AppResult, ErrorCode},
    models::settings::{IgnoreTemplateSettings, CURRENT_SCHEMA_VERSION},
};
use serde_json::Value;

pub fn migrate(mut value: Value, schema_version: u32) -> AppResult<Value> {
    if schema_version > CURRENT_SCHEMA_VERSION {
        return Err(AppError::simple(
            ErrorCode::SettingsUnsupportedSchema,
            "この設定ファイルは新しい Vsedi で作成されています。",
            "validate_settings_schema",
        ));
    }

    if let Some(object) = value.as_object_mut() {
        if schema_version < 2 {
            object.insert(
                "vpmTrackingPolicy".to_owned(),
                Value::from("EXCLUDE_PACKAGES"),
            );
        }
        if schema_version < 3 {
            object.insert(
                "ignoreTemplates".to_owned(),
                serde_json::to_value(IgnoreTemplateSettings::default()).expect("default ignore templates serialize"),
            );
        }
        if schema_version < CURRENT_SCHEMA_VERSION {
            object.insert(
                "schemaVersion".to_owned(),
                Value::from(CURRENT_SCHEMA_VERSION),
            );
        }
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::migrate;
    use serde_json::json;

    #[test]
    fn schema_one_adds_initialization_defaults() {
        let migrated = migrate(
            json!({
                "schemaVersion": 1,
                "recentProjects": [{ "path": "/project", "lastOpenedAt": null }]
            }),
            1,
        )
        .expect("migration");

        assert_eq!(migrated["schemaVersion"], 3);
        assert_eq!(migrated["vpmTrackingPolicy"], "EXCLUDE_PACKAGES");
        assert_eq!(migrated["ignoreTemplates"]["unityRules"][0], "/[Ll]ibrary/*");
        assert_eq!(migrated["recentProjects"][0]["path"], "/project");
    }
}
