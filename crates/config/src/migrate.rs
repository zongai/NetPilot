//! Versioned config migration (NP-029).

use crate::{ConfigDocument, ConfigError};

/// Current schema written by this build.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Migrate a document up to [`CURRENT_SCHEMA_VERSION`].
pub fn migrate_document(mut doc: ConfigDocument) -> Result<ConfigDocument, ConfigError> {
    if doc.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(ConfigError::UnsupportedVersion(doc.schema_version));
    }
    while doc.schema_version < CURRENT_SCHEMA_VERSION {
        doc = migrate_one_step(doc)?;
    }
    Ok(doc)
}

fn migrate_one_step(mut doc: ConfigDocument) -> Result<ConfigDocument, ConfigError> {
    match doc.schema_version {
        // Legacy: treat missing/0 as version 1 with empty defaults already via serde.
        0 => {
            doc.schema_version = 1;
            Ok(doc)
        }
        v => Err(ConfigError::UnsupportedVersion(v)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_zero_to_one() {
        let mut doc = ConfigDocument::empty();
        doc.schema_version = 0;
        let m = migrate_document(doc).unwrap();
        assert_eq!(m.schema_version, 1);
    }

    #[test]
    fn rejects_future() {
        let mut doc = ConfigDocument::empty();
        doc.schema_version = 99;
        assert!(matches!(
            migrate_document(doc),
            Err(ConfigError::UnsupportedVersion(99))
        ));
    }

    #[test]
    fn identity_on_current() {
        let doc = ConfigDocument::empty();
        let m = migrate_document(doc.clone()).unwrap();
        assert_eq!(m, doc);
    }
}
