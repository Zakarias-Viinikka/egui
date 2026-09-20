#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SchemaVersion {
    Version0,
    Version1,
    Version2,
    Version3,
}

pub const CURRENT_VERSION: SchemaVersion = SchemaVersion::Version3;

impl SchemaVersion {
    pub fn to_int(self) -> i64 {
        match self {
            SchemaVersion::Version0 => 0,
            SchemaVersion::Version1 => 1,
            SchemaVersion::Version2 => 2,
            SchemaVersion::Version3 => 3,
        }
    }

    pub fn from_int(v: i64) -> Option<Self> {
        match v {
            0 => Some(SchemaVersion::Version0),
            1 => Some(SchemaVersion::Version1),
            2 => Some(SchemaVersion::Version2),
            3 => Some(SchemaVersion::Version3),
            _ => None,
        }
    }
}
