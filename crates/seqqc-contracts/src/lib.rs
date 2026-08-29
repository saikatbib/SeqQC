//! Versioned `SeqQC` contracts bundled into the native application.
//!
//! Bundling makes contract resolution deterministic and keeps it independent
//! of DNS or network availability. `$id` values remain stable identifiers;
//! every `$ref` is resolved from this in-process registry.

/// One embedded JSON Schema document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemaDocument {
    pub name: &'static str,
    pub json: &'static str,
}

macro_rules! schema {
    ($name:literal) => {
        SchemaDocument {
            name: $name,
            json: include_str!(concat!("../../../schemas/v1/", $name)),
        }
    };
}

/// All v1 contracts available to the native engine.
pub const V1_SCHEMAS: [SchemaDocument; 9] = [
    schema!("artifact.schema.json"),
    schema!("common.schema.json"),
    schema!("completion-manifest.schema.json"),
    schema!("finding.schema.json"),
    schema!("metric.schema.json"),
    schema!("project.schema.json"),
    schema!("project-manifest.schema.json"),
    schema!("report.schema.json"),
    schema!("workflow.schema.json"),
];

/// Finds an embedded schema by its repository filename.
#[must_use]
pub fn v1_schema(name: &str) -> Option<&'static SchemaDocument> {
    V1_SCHEMAS.iter().find(|schema| schema.name == name)
}

#[cfg(test)]
mod tests {
    use super::{V1_SCHEMAS, v1_schema};

    #[test]
    fn embeds_the_complete_v1_registry() {
        assert_eq!(V1_SCHEMAS.len(), 9);
        assert!(v1_schema("report.schema.json").is_some());
        assert!(v1_schema("unknown.schema.json").is_none());
    }

    #[test]
    fn schemas_have_stable_ids_and_only_local_references() {
        for schema in V1_SCHEMAS {
            let expected_id = format!("https://schemas.seqqc.org/v1/{}", schema.name);
            assert!(
                schema.json.contains(&expected_id),
                "missing id for {}",
                schema.name
            );

            for line in schema.json.lines().filter(|line| line.contains("\"$ref\"")) {
                assert!(
                    !line.contains("http://") && !line.contains("https://"),
                    "runtime network reference in {}: {line}",
                    schema.name
                );
            }
        }
    }
}
