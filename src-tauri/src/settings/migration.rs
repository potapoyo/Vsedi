use crate::{
    errors::{AppError, AppResult, ErrorCode},
    models::settings::{IgnoreTemplateSettings, CURRENT_SCHEMA_VERSION, DEFAULT_OS_IGNORE_RULES},
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
                serde_json::to_value(IgnoreTemplateSettings::default())
                    .expect("default ignore templates serialize"),
            );
        }
        if schema_version < 4 {
            if let Some(projects) = object
                .get_mut("recentProjects")
                .and_then(Value::as_array_mut)
            {
                for project in projects {
                    if let Some(project) = project.as_object_mut() {
                        project
                            .entry("tags".to_owned())
                            .or_insert_with(|| Value::Array(Vec::new()));
                    }
                }
            }
        }
        if schema_version < 5 {
            object.insert("repositorySettings".to_owned(), Value::Array(Vec::new()));
        }
        if schema_version < 6 {
            if let Some(projects) = object
                .get_mut("recentProjects")
                .and_then(Value::as_array_mut)
            {
                for project in projects {
                    if let Some(project) = project.as_object_mut() {
                        let tags = match project.remove("category") {
                            Some(Value::String(category)) if !category.trim().is_empty() => {
                                Value::Array(vec![Value::String(category)])
                            }
                            Some(Value::Array(tags)) => Value::Array(tags),
                            _ => project
                                .remove("tags")
                                .unwrap_or_else(|| Value::Array(Vec::new())),
                        };
                        project.insert("tags".to_owned(), tags);
                    }
                }
            }
        }
        if schema_version < 7 {
            add_missing_os_ignore_rules(object);
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

fn add_missing_os_ignore_rules(object: &mut serde_json::Map<String, Value>) {
    let templates = object.entry("ignoreTemplates").or_insert_with(|| {
        serde_json::to_value(IgnoreTemplateSettings::default())
            .expect("default ignore templates serialize")
    });
    let Some(templates) = templates.as_object_mut() else {
        *templates = serde_json::to_value(IgnoreTemplateSettings::default())
            .expect("default ignore templates serialize");
        return;
    };
    let rules = templates
        .entry("unityRules")
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(rules) = rules.as_array_mut() else {
        *rules = Value::Array(Vec::new());
        return;
    };
    for rule in DEFAULT_OS_IGNORE_RULES {
        if !rules.iter().any(|existing| existing.as_str() == Some(rule)) {
            rules.push(Value::String((*rule).to_owned()));
        }
    }
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

        assert_eq!(migrated["schemaVersion"], 7);
        assert_eq!(migrated["vpmTrackingPolicy"], "EXCLUDE_PACKAGES");
        assert_eq!(
            migrated["ignoreTemplates"]["unityRules"][0],
            "/[Ll]ibrary/*"
        );
        assert!(migrated["ignoreTemplates"]["unityRules"]
            .as_array()
            .is_some_and(|rules| rules.iter().any(|rule| rule == ".DS_Store")));
        assert!(!migrated["ignoreTemplates"]["unityRules"]
            .as_array()
            .is_some_and(|rules| rules.iter().any(|rule| rule == "Icon")));
        assert_eq!(migrated["recentProjects"][0]["path"], "/project");
        assert_eq!(migrated["recentProjects"][0]["tags"], serde_json::json!([]));
        assert!(migrated["repositorySettings"]
            .as_array()
            .is_some_and(Vec::is_empty));
    }

    #[test]
    fn schema_three_adds_project_tags() {
        let migrated = migrate(
            json!({
                "schemaVersion": 3,
                "recentProjects": [{ "path": "/project", "lastOpenedAt": "2026-08-12T00:00:00Z" }],
                "vpmTrackingPolicy": "EXCLUDE_PACKAGES",
                "ignoreTemplates": { "unityRules": [], "vpmExcludeRules": [] }
            }),
            3,
        )
        .expect("migration");

        assert_eq!(migrated["schemaVersion"], 7);
        assert_eq!(migrated["recentProjects"][0]["tags"], serde_json::json!([]));
        assert!(migrated["repositorySettings"]
            .as_array()
            .is_some_and(Vec::is_empty));
    }

    #[test]
    fn schema_four_adds_repository_settings() {
        let migrated = migrate(
            json!({
                "schemaVersion": 4,
                "recentProjects": [],
                "vpmTrackingPolicy": "INCLUDE_PACKAGES",
                "ignoreTemplates": { "unityRules": [], "vpmExcludeRules": [] }
            }),
            4,
        )
        .expect("migration");

        assert_eq!(migrated["schemaVersion"], 7);
        assert!(migrated["repositorySettings"]
            .as_array()
            .is_some_and(Vec::is_empty));
    }

    #[test]
    fn schema_five_converts_project_category_to_a_tag() {
        let migrated = migrate(
            json!({
                "schemaVersion": 5,
                "recentProjects": [{ "path": "/project", "category": " Avatar " }],
                "repositorySettings": []
            }),
            5,
        )
        .expect("migration");

        assert_eq!(migrated["schemaVersion"], 7);
        assert_eq!(migrated["recentProjects"][0]["tags"], json!([" Avatar "]));
        assert!(migrated["recentProjects"][0].get("category").is_none());
    }

    #[test]
    fn schema_six_adds_os_ignore_rules_without_removing_custom_rules() {
        let migrated = migrate(
            json!({
                "schemaVersion": 6,
                "recentProjects": [],
                "ignoreTemplates": {
                    "unityRules": ["custom-cache/", ".DS_Store"],
                    "vpmExcludeRules": []
                },
                "repositorySettings": []
            }),
            6,
        )
        .expect("migration");

        assert_eq!(migrated["schemaVersion"], 7);
        assert_eq!(
            migrated["ignoreTemplates"]["unityRules"][0],
            "custom-cache/"
        );
        let rules = migrated["ignoreTemplates"]["unityRules"]
            .as_array()
            .expect("unity rules");
        assert_eq!(rules.iter().filter(|rule| *rule == ".DS_Store").count(), 1);
        assert!(rules.iter().any(|rule| rule == "Thumbs.db"));
    }
}
