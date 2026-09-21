use crate::migration::helpers::ADbTable;
use crate::migration::schema_versions::{CURRENT_VERSION, SchemaVersion};
use crate::migration::schemas::{version0, version1, version2, version3, version4, version5, version6, version7};

pub fn entire_table() -> Vec<ADbTable> {
    match CURRENT_VERSION {
        SchemaVersion::Version0 => version0::entire_table(),
        SchemaVersion::Version1 => version1::entire_table(),
        SchemaVersion::Version2 => version2::entire_table(),
        SchemaVersion::Version3 => version3::entire_table(),
        SchemaVersion::Version4 => version4::entire_table(),
        SchemaVersion::Version5 => version5::entire_table(),
        SchemaVersion::Version6 => version6::entire_table(),
        SchemaVersion::Version7 => version7::entire_table(),
    }
}
