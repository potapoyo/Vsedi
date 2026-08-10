use crate::{
    errors::{AppError, AppResult, ErrorCode},
    models::settings::CURRENT_SCHEMA_VERSION,
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

    if schema_version < CURRENT_SCHEMA_VERSION {
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "schemaVersion".to_owned(),
                Value::from(CURRENT_SCHEMA_VERSION),
            );
        }
    }
    Ok(value)
}
