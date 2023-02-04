pub(crate) fn to_u32(bytes: &[u8]) -> Result<u32, ()> {
    let bytes: [u8; 4] = match bytes.try_into() {
        Ok(bytes) => bytes,
        Err(_) => return Err(()),
    };
    Ok(u32::from_be_bytes(bytes))
}

pub(crate) fn to_u16(bytes: &[u8]) -> Result<u16, ()> {
    let bytes: [u8; 2] = match bytes.try_into() {
        Ok(bytes) => bytes,
        Err(_) => return Err(()),
    };
    Ok(u16::from_be_bytes(bytes))
}

pub(crate) fn u8s_to_hex_str(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn hex_str_to_u8s(hex: &str) -> Result<Vec<u8>, ()> {
    let stripped = hex
        .chars()
        .filter(char::is_ascii_hexdigit)
        .collect::<Vec<char>>();
    if stripped.len() % 2 != 0 {
        return Err(());
    }
    stripped
        .chunks(2)
        .map(|double_hex| double_hex.iter().collect::<String>())
        .map(|hex_string| u8::from_str_radix(&hex_string, 16).map_err(|_| ()))
        .collect::<Result<Vec<_>, ()>>()
}
