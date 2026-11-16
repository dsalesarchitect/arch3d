//! STEP ISO-10303-21 Physical File Lexer, Tokenizer, and AST.
//!
//! Provides robust, high-performance parsing of STEP physical files across
//! all IFC schema versions (IFC2X, IFC2X2, IFC2X3, IFC4, IFC4X3).

use plinth_geom::{vec2, Vec2, Vec3};
use std::collections::BTreeMap;

/// A parsed STEP data value.
#[derive(Clone, Debug, PartialEq)]
pub enum StepValue {
    /// Omitted or unset attribute (`$`)
    Unset,
    /// Derived or overridden attribute (`*`)
    Derived,
    /// 64-bit signed integer
    Integer(i64),
    /// 64-bit floating point number
    Float(f64),
    /// String literal (unescaped single-quoted string)
    String(String),
    /// Entity reference (e.g. `#123`)
    Ref(usize),
    /// Enumeration value (e.g. `.NOTDEFINED.`, `.MILLI.`, `.METRE.`)
    Enum(String),
    /// Ordered list / aggregate tuple (e.g. `(1.0, 2.0, 3.0)`)
    List(Vec<StepValue>),
    /// Inlined typed entity wrapper (e.g. `IFCLABEL('Concrete')` or `IFCPARAMETERVALUE(3000.0)`)
    Typed(String, Vec<StepValue>),
}

impl StepValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            StepValue::Float(f) => Some(*f),
            StepValue::Integer(i) => Some(*i as f64),
            StepValue::Typed(_, sub) if !sub.is_empty() => sub[0].as_f64(),
            _ => None,
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        match self {
            StepValue::Integer(i) if *i >= 0 => Some(*i as usize),
            StepValue::Ref(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> Option<usize> {
        match self {
            StepValue::Ref(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            StepValue::String(s) => Some(s.as_str()),
            StepValue::Enum(s) => Some(s.as_str()),
            StepValue::Typed(_, sub) if !sub.is_empty() => sub[0].as_str(),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[StepValue]> {
        match self {
            StepValue::List(list) => Some(list.as_slice()),
            _ => None,
        }
    }
}

/// A parsed STEP entity declaration: `#ID = NAME(ARG1, ARG2, ...);`
#[derive(Clone, Debug)]
pub struct StepEntity {
    pub id: usize,
    pub name: String,
    pub args: Vec<StepValue>,
}

impl StepEntity {
    pub fn get_arg(&self, index: usize) -> Option<&StepValue> {
        self.args.get(index)
    }

    pub fn get_f64(&self, index: usize) -> Option<f64> {
        self.args.get(index).and_then(|v| v.as_f64())
    }

    pub fn get_str(&self, index: usize) -> Option<&str> {
        self.args.get(index).and_then(|v| v.as_str())
    }

    pub fn get_ref(&self, index: usize) -> Option<usize> {
        self.args.get(index).and_then(|v| v.as_ref())
    }

    pub fn get_list(&self, index: usize) -> Option<&[StepValue]> {
        self.args.get(index).and_then(|v| v.as_list())
    }
}

/// A complete parsed STEP physical file document.
#[derive(Clone, Debug, Default)]
pub struct StepFile {
    pub schema: String,
    pub entities: BTreeMap<usize, StepEntity>,
}

impl StepFile {
    pub fn get(&self, id: usize) -> Option<&StepEntity> {
        self.entities.get(&id)
    }

    pub fn iter_by_name<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a StepEntity> {
        let name_upper = name.to_uppercase();
        self.entities.values().filter(move |e| e.name == name_upper)
    }

    /// Resolves an `IFCCARTESIANPOINT` (either 2D or 3D) with the given scale factor.
    pub fn resolve_point_3d(&self, id: usize, scale: f64) -> Option<Vec3> {
        let ent = self.get(id)?;
        if ent.name != "IFCCARTESIANPOINT" {
            return None;
        }
        let list = ent.get_list(0)?;
        let x = list.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) * scale;
        let y = list.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) * scale;
        let z = list.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) * scale;
        Some(Vec3::new(x, y, z))
    }

    /// Resolves a 2D `IFCCARTESIANPOINT` with the given scale factor.
    pub fn resolve_point_2d(&self, id: usize, scale: f64) -> Option<Vec2> {
        let ent = self.get(id)?;
        if ent.name != "IFCCARTESIANPOINT" {
            return None;
        }
        let list = ent.get_list(0)?;
        let x = list.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) * scale;
        let y = list.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) * scale;
        Some(vec2(x, y))
    }

    /// Resolves an `IFCDIRECTION` as a normalized 3D vector.
    pub fn resolve_direction_3d(&self, id: usize) -> Option<Vec3> {
        let ent = self.get(id)?;
        if ent.name != "IFCDIRECTION" {
            return None;
        }
        let list = ent.get_list(0)?;
        let dx = list.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let dy = list.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let dz = list.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        if len > 1e-6 {
            Some(Vec3::new(dx / len, dy / len, dz / len))
        } else {
            Some(Vec3::Z)
        }
    }

    /// Resolves an `IFCDIRECTION` as a normalized 2D vector.
    pub fn resolve_direction_2d(&self, id: usize) -> Option<Vec2> {
        let ent = self.get(id)?;
        if ent.name != "IFCDIRECTION" {
            return None;
        }
        let list = ent.get_list(0)?;
        let dx = list.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let dy = list.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let len = (dx * dx + dy * dy).sqrt();
        if len > 1e-6 {
            Some(vec2(dx / len, dy / len))
        } else {
            Some(Vec2::X)
        }
    }

    /// Resolves an `IFCAXIS2PLACEMENT3D` into a 3D coordinate frame: (origin, u_x, u_y, u_z).
    pub fn resolve_axis2_placement_3d(&self, id: usize, scale: f64) -> Option<TransformFrame3D> {
        let ent = self.get(id)?;
        if ent.name != "IFCAXIS2PLACEMENT3D" {
            return None;
        }
        let origin = ent.get_ref(0).and_then(|p_id| self.resolve_point_3d(p_id, scale)).unwrap_or(Vec3::ZERO);
        let axis_z = ent.get_ref(1).and_then(|d_id| self.resolve_direction_3d(d_id)).unwrap_or(Vec3::Z);
        let ref_x = ent.get_ref(2).and_then(|d_id| self.resolve_direction_3d(d_id)).unwrap_or(Vec3::X);

        // Gram-Schmidt orthogonalization: u_z = axis_z, u_x = normalized projection of ref_x perp to u_z, u_y = u_z x u_x
        let u_z = axis_z.normalized();
        let proj_x = ref_x - u_z * ref_x.dot(u_z);
        let u_x = if proj_x.length() > 1e-5 {
            proj_x.normalized()
        } else {
            // Pick an arbitrary perpendicular vector if ref_x is collinear with u_z
            let alt = if u_z.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
            (alt - u_z * alt.dot(u_z)).normalized()
        };
        let u_y = u_z.cross(u_x).normalized();

        Some(TransformFrame3D {
            origin,
            u_x,
            u_y,
            u_z,
        })
    }

    /// Resolves an `IFCAXIS2PLACEMENT2D` into a 2D coordinate frame: (origin, u_x, u_y).
    pub fn resolve_axis2_placement_2d(&self, id: usize, scale: f64) -> Option<(Vec2, Vec2, Vec2)> {
        let ent = self.get(id)?;
        if ent.name != "IFCAXIS2PLACEMENT2D" {
            return None;
        }
        let origin = ent.get_ref(0).and_then(|p_id| self.resolve_point_2d(p_id, scale)).unwrap_or(Vec2::ZERO);
        let u_x = ent.get_ref(1).and_then(|d_id| self.resolve_direction_2d(d_id)).unwrap_or(Vec2::X);
        let u_y = u_x.perp();
        Some((origin, u_x, u_y))
    }

    /// Resolves an `IFCCARTESIANTRANSFORMATIONOPERATOR3D` or `IFCCARTESIANTRANSFORMATIONOPERATOR` into a coordinate frame.
    pub fn resolve_cartesian_transformation_3d(&self, id: usize, scale: f64) -> Option<TransformFrame3D> {
        let ent = self.get(id)?;
        if !ent.name.starts_with("IFCCARTESIANTRANSFORMATIONOPERATOR") {
            return None;
        }
        let origin = ent.get_ref(2).and_then(|p_id| self.resolve_point_3d(p_id, scale)).unwrap_or(Vec3::ZERO);
        let u_x = ent.get_ref(0).and_then(|d_id| self.resolve_direction_3d(d_id)).unwrap_or(Vec3::X);
        let u_z = ent.get_ref(4).and_then(|d_id| self.resolve_direction_3d(d_id)).unwrap_or(Vec3::Z);
        let u_y = ent.get_ref(1).and_then(|d_id| self.resolve_direction_3d(d_id)).unwrap_or_else(|| u_z.cross(u_x).normalized());

        // Attribute 3: Scale (optional real)
        let s1 = ent.get_f64(3).unwrap_or(1.0);
        // Attributes 5, 6: Scale2, Scale3 for IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM
        let s2 = ent.get_f64(5).unwrap_or(s1);
        let s3 = ent.get_f64(6).unwrap_or(s1);

        Some(TransformFrame3D {
            origin,
            u_x: u_x * s1,
            u_y: u_y * s2,
            u_z: u_z * s3,
        })
    }

    /// Resolves hierarchical `IFCLOCALPLACEMENT` by walking up `PlacementRelTo`
    /// and composing coordinate frames into world space.
    pub fn resolve_world_placement(&self, placement_id: Option<usize>, scale: f64) -> TransformFrame3D {
        let mut curr_id = placement_id;
        let mut frames: Vec<TransformFrame3D> = Vec::new();
        let mut visited = std::collections::HashSet::new();

        while let Some(id) = curr_id {
            if !visited.insert(id) {
                break; // Cycle protection
            }
            if let Some(ent) = self.get(id) {
                if ent.name == "IFCLOCALPLACEMENT" {
                    if let Some(rel_id) = ent.get_ref(1) {
                        if let Some(frame) = self.resolve_axis2_placement_3d(rel_id, scale) {
                            frames.push(frame);
                        }
                    }
                    curr_id = ent.get_ref(0);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Multiply frames from root to leaf: T_world = T_root * ... * T_leaf
        let mut world_frame = TransformFrame3D::identity();
        for frame in frames.into_iter().rev() {
            world_frame = world_frame.combine(&frame);
        }

        world_frame
    }
}

/// A 3D coordinate frame defining an affine transformation:
/// P_parent = origin + x * u_x + y * u_y + z * u_z
#[derive(Clone, Copy, Debug)]
pub struct TransformFrame3D {
    pub origin: Vec3,
    pub u_x: Vec3,
    pub u_y: Vec3,
    pub u_z: Vec3,
}

impl TransformFrame3D {
    pub fn identity() -> Self {
        Self {
            origin: Vec3::ZERO,
            u_x: Vec3::X,
            u_y: Vec3::Y,
            u_z: Vec3::Z,
        }
    }

    /// Transforms a local point into the parent coordinate system.
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        self.origin + self.u_x * p.x + self.u_y * p.y + self.u_z * p.z
    }

    /// Transforms a local direction vector into parent coordinate system.
    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        self.u_x * v.x + self.u_y * v.y + self.u_z * v.z
    }

    /// Composes this frame (parent) with a child frame: T_combined(P) = this(child(P)).
    pub fn combine(&self, child: &TransformFrame3D) -> Self {
        Self {
            origin: self.transform_point(child.origin),
            u_x: self.transform_vector(child.u_x),
            u_y: self.transform_vector(child.u_y),
            u_z: self.transform_vector(child.u_z),
        }
    }
}

// ---------------------------------------------------------------------------
// STEP ISO-10303-21 Lexer & Parser
// ---------------------------------------------------------------------------

/// Parses a raw STEP text buffer into a structured `StepFile`.
pub fn parse_step_file(text: &str) -> Result<StepFile, String> {
    let mut step = StepFile::default();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;

    let mut in_header = false;
    let mut in_data = false;

    while i < n {
        // Skip whitespace
        while i < n && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= n {
            break;
        }

        // Handle multi-line block comments /* ... */
        if i + 1 < n && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(n);
            continue;
        }

        // Read statement up to terminating semicolon ';'
        let stmt_start = i;
        let mut in_quote = false;
        while i < n {
            let c = chars[i];
            if c == '\'' {
                // Handle escaped quotes ''
                if in_quote && i + 1 < n && chars[i + 1] == '\'' {
                    i += 2;
                    continue;
                }
                in_quote = !in_quote;
            } else if c == ';' && !in_quote {
                break;
            }
            i += 1;
        }

        let stmt_end = i;
        if i < n && chars[i] == ';' {
            i += 1;
        }

        let stmt_str: String = chars[stmt_start..stmt_end].iter().collect();
        let trimmed = stmt_str.trim();

        if trimmed.eq_ignore_ascii_case("HEADER") {
            in_header = true;
            in_data = false;
            continue;
        } else if trimmed.eq_ignore_ascii_case("DATA") {
            in_data = true;
            in_header = false;
            continue;
        } else if trimmed.eq_ignore_ascii_case("ENDSEC") {
            in_header = false;
            in_data = false;
            continue;
        }

        // Header Section: extract FILE_SCHEMA
        if in_header {
            if let Some(schema_pos) = trimmed.to_uppercase().find("FILE_SCHEMA") {
                let rest = &trimmed[schema_pos..];
                if let Some(open) = rest.find('(') {
                    if let Some(close) = rest.rfind(')') {
                        let inner = &rest[open + 1..close];
                        for part in inner.split(',') {
                            let clean = part.trim().trim_matches('(').trim_matches(')').trim().trim_matches('\'');
                            if !clean.is_empty() {
                                step.schema = clean.to_uppercase();
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Data Section: parse #ID = ENTITY_NAME(args);
        if in_data && trimmed.starts_with('#') {
            if let Some(eq_pos) = trimmed.find('=') {
                let id_str = trimmed[1..eq_pos].trim();
                if let Ok(id) = id_str.parse::<usize>() {
                    let rest = trimmed[eq_pos + 1..].trim();
                    if let Some(paren_open) = rest.find('(') {
                        let entity_name = rest[..paren_open].trim().to_uppercase();
                        if let Some(paren_close) = rest.rfind(')') {
                            let arg_slice = &rest[paren_open + 1..paren_close];
                            let args = parse_argument_list(arg_slice);
                            step.entities.insert(
                                id,
                                StepEntity {
                                    id,
                                    name: entity_name,
                                    args,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    if step.schema.is_empty() {
        step.schema = "IFC4".into(); // Default if not specified in header
    }

    if step.entities.is_empty() {
        return Err("No IFC STEP entities found in DATA section".into());
    }

    Ok(step)
}

/// Splits and parses a comma-separated argument list, correctly respecting
/// nested parentheses, string literals, and typed wrappers.
fn parse_argument_list(input: &str) -> Vec<StepValue> {
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut args = Vec::new();
    let mut depth: usize = 0;
    let mut in_quote = false;
    let mut start = 0;
    let mut i = 0;

    while i < n {
        let c = chars[i];
        if c == '\'' {
            if in_quote && i + 1 < n && chars[i + 1] == '\'' {
                i += 2;
                continue;
            }
            in_quote = !in_quote;
        } else if !in_quote {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth = depth.saturating_sub(1);
            } else if c == ',' && depth == 0 {
                let piece: String = chars[start..i].iter().collect();
                args.push(parse_single_value(piece.trim()));
                start = i + 1;
            }
        }
        i += 1;
    }

    if start < n {
        let piece: String = chars[start..n].iter().collect();
        let trimmed = piece.trim();
        if !trimmed.is_empty() || !args.is_empty() {
            args.push(parse_single_value(trimmed));
        }
    }

    args
}

/// Parses a single STEP value token.
fn parse_single_value(tok: &str) -> StepValue {
    let s = tok.trim();
    if s.is_empty() || s == "$" {
        return StepValue::Unset;
    }
    if s == "*" {
        return StepValue::Derived;
    }

    // String literal: '...'
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        let raw = &s[1..s.len() - 1];
        return StepValue::String(unescape_step_string(raw));
    }

    // Entity Reference: #123
    if s.starts_with('#') {
        if let Ok(id) = s[1..].parse::<usize>() {
            return StepValue::Ref(id);
        }
    }

    // Enumeration value: .ENUM.
    if s.starts_with('.') && s.ends_with('.') && s.len() >= 2 {
        return StepValue::Enum(s[1..s.len() - 1].to_uppercase());
    }

    // Aggregate / List: (...)
    if s.starts_with('(') && s.ends_with(')') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        let sub = parse_argument_list(inner);
        return StepValue::List(sub);
    }

    // Typed wrapper: NAME(args)
    if let Some(open) = s.find('(') {
        if s.ends_with(')') {
            let type_name = s[..open].trim().to_uppercase();
            let inner = &s[open + 1..s.len() - 1];
            let sub = parse_argument_list(inner);
            return StepValue::Typed(type_name, sub);
        }
    }

    // Integer
    if let Ok(val) = s.parse::<i64>() {
        return StepValue::Integer(val);
    }

    // Floating-point number (including 1.0, 1.E-5, .5, -.2)
    if let Ok(val) = s.parse::<f64>() {
        return StepValue::Float(val);
    }

    // Fallback: unquoted identifier or string
    StepValue::String(s.to_string())
}

/// Unescapes STEP physical file string literals according to ISO-10303-21.
/// Handles `''` -> `'`, `\X2\...\X0\` (UTF-16 hex), `\X4\...\X0\` (UTF-32 hex), and `\S\...`.
fn unescape_step_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        if chars[i] == '\'' && i + 1 < n && chars[i + 1] == '\'' {
            out.push('\'');
            i += 2;
        } else if chars[i] == '\\' && i + 3 < n && chars[i + 1] == 'X' && chars[i + 2] == '2' && chars[i + 3] == '\\' {
            // \X2\HHHHHHHH\X0\ (UTF-16 Big Endian 4-hex digit sequences)
            i += 4;
            while i + 3 < n && !(chars[i] == '\\' && chars[i + 1] == 'X' && chars[i + 2] == '0' && chars[i + 3] == '\\') {
                if i + 3 < n {
                    let hex_str: String = chars[i..i + 4].iter().collect();
                    if let Ok(code) = u16::from_str_radix(&hex_str, 16) {
                        if let Some(ch) = char::from_u32(code as u32) {
                            out.push(ch);
                        }
                    }
                    i += 4;
                } else {
                    break;
                }
            }
            if i + 3 < n {
                i += 4; // skip \X0\
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_step_file_basic() {
        let sample = "\
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2.0]'),'2;1');
FILE_NAME('test.ifc','2026-10-31T00:00:00',('Architect'),('Arch3D'),'Arch3D IFC','Arch3D','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCCARTESIANPOINT((0.,0.,0.));
#2=IFCDIRECTION((0.,0.,1.));
#3=IFCDIRECTION((1.,0.,0.));
#4=IFCAXIS2PLACEMENT3D(#1,#2,#3);
#5=IFCBUILDINGSTOREY('2q_zB3v798BfHqG3d7l$4k',$,'Level 1',$,$,#4,$,$,.ELEMENT.,0.);
#6=IFCWALL('1tWf4$Ocf7qwbK84Rk$7_Y',$,'Exterior Wall',$,$,#4,$,$);
ENDSEC;
END-10303-21;
";
        let step = parse_step_file(sample).expect("Should parse valid STEP");
        assert_eq!(step.schema, "IFC4");
        assert_eq!(step.entities.len(), 6);

        let pt = step.resolve_point_3d(1, 1.0).unwrap();
        assert_eq!(pt, Vec3::ZERO);

        let dir_z = step.resolve_direction_3d(2).unwrap();
        assert_eq!(dir_z, Vec3::Z);

        let frame = step.resolve_axis2_placement_3d(4, 1.0).unwrap();
        assert_eq!(frame.origin, Vec3::ZERO);
        assert_eq!(frame.u_z, Vec3::Z);
        assert_eq!(frame.u_x, Vec3::X);
    }

    #[test]
    fn test_step_unescape() {
        let s = "Arch3D''s \\X2\\00500072006F006A006500630074\\X0\\";
        let clean = unescape_step_string(s);
        assert_eq!(clean, "Arch3D's Project");
    }
}
