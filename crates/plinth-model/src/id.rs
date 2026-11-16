//! Strongly typed identifiers for BIM entities.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub const fn from_uuid(u: Uuid) -> Self {
                Self(u)
            }

            pub fn nil() -> Self {
                Self(Uuid::nil())
            }

            pub fn short_id(&self) -> String {
                let s = self.0.to_string();
                format!("{}-{}", $prefix, &s[..8])
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), &self.0.to_string()[..8])
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id!(ElementId, "ELM");
define_id!(LevelId, "LVL");
define_id!(GridId, "GRD");
define_id!(ViewId, "VIW");
define_id!(MaterialId, "MAT");
define_id!(SheetId, "SHT");
define_id!(ViewportId, "VPT");
