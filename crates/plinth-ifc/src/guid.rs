//! buildingSMART IFC 22-character Globally Unique Identifier (IfcGloballyUniqueId).

use uuid::Uuid;

const BASE64_CHARS: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

/// Converts a 128-bit UUID into buildingSMART's 22-character compressed IFC GUID string.
pub fn uuid_to_ifc_guid(uuid: Uuid) -> String {
    let bytes = uuid.as_bytes();
    let mut out = [b'0'; 22];

    // buildingSMART conversion: 16 bytes (128 bits) -> 22 characters (each 6 bits, first character takes remaining 2 bits)
    let n0 = ((bytes[0] as u32) & 0x03) as usize;
    out[0] = BASE64_CHARS[n0];

    // Groups of 3 bytes into 4 6-bit chars
    let mut out_idx = 1;
    let mut i = 0;
    while i < 15 {
        let b0 = bytes[i] as u32;
        let b1 = bytes[i + 1] as u32;
        let b2 = bytes[i + 2] as u32;

        let val = (b0 << 16) | (b1 << 8) | b2;
        out[out_idx] = BASE64_CHARS[((val >> 18) & 0x3F) as usize];
        out[out_idx + 1] = BASE64_CHARS[((val >> 12) & 0x3F) as usize];
        out[out_idx + 2] = BASE64_CHARS[((val >> 6) & 0x3F) as usize];
        out[out_idx + 3] = BASE64_CHARS[(val & 0x3F) as usize];

        out_idx += 4;
        i += 3;
    }

    // Remaining last byte
    let b15 = bytes[15] as u32;
    out[21] = BASE64_CHARS[(b15 & 0x3F) as usize];

    String::from_utf8(out.to_vec()).unwrap_or_else(|_| "0000000000000000000000".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ifc_guid_length() {
        let g = uuid_to_ifc_guid(Uuid::new_v4());
        assert_eq!(g.len(), 22);
    }
}
