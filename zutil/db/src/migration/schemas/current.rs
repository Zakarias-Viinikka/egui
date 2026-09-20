use crate::migration::helpers::ADbTable;
use crate::migration::schema_versions::{CURRENT_VERSION, SchemaVersion};
use crate::migration::schemas::{version0, version1, version2, version3};

pub fn entire_table() -> Vec<ADbTable> {
    match CURRENT_VERSION {
        SchemaVersion::Version0 => version0::entire_table(),
        SchemaVersion::Version1 => version1::entire_table(),
        SchemaVersion::Version2 => version2::entire_table(),
        SchemaVersion::Version3 => version3::entire_table(),
    }
}
