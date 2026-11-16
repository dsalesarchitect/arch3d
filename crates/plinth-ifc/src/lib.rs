//! # plinth-ifc
//!
//! Standards-compliant IFC4 STEP physical file exporter and parser.
//! Conforms to buildingSMART IFC4 Coordination View schema.

pub mod guid;
pub mod parser;
pub mod step;
pub mod writer;

pub use guid::uuid_to_ifc_guid;
pub use parser::{parse_ifc, parse_ifc4};
pub use writer::export_ifc4;
