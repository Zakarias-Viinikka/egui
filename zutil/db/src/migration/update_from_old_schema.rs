use db_wrapper::mascot::LiveForever;

use crate::migration::schema_versions::SchemaVersion;
use crate::migration::schemas::{version1, version2, version3};

pub fn update_once(current: SchemaVersion, db_wrapper: &LiveForever) -> SchemaVersion {
    match current {
        SchemaVersion::Version0 => {
            version1::update_from_old_version(db_wrapper);
            SchemaVersion::Version1
        }
        SchemaVersion::Version1 => {
            version2::update_from_old_version(db_wrapper);
            SchemaVersion::Version2
        }
        SchemaVersion::Version2 => {
            version3::update_from_old_version(db_wrapper);
            SchemaVersion::Version3
        }
        SchemaVersion::Version3 => SchemaVersion::Version3,
    }
}

pub fn update_until_newest_version(
    mut current: SchemaVersion,
    db_wrapper: &LiveForever,
) -> SchemaVersion {
    loop {
        let next = update_once(current, db_wrapper);
        if next == current {
            return next;
        }
        current = next;
    }
}
