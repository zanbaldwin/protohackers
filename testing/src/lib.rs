use std::net::{TcpListener, TcpStream};

pub fn find_available_port() -> Option<u16> {
    (8000..u16::MAX).find(|port| match TcpListener::bind(("127.0.0.1", *port)) {
        Ok(_) => true,
        Err(_) => false,
    })
}

pub fn connect(port: u16) -> TcpStream {
    TcpStream::connect(("127.0.0.1", port)).expect("Could not connect to integration server.")
}

pub fn u8s_to_hex_str(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn hex_str_to_u8s(hex: &str) -> Result<Vec<u8>, ()> {
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

#[macro_export]
macro_rules! send_bytes_from {
    ($s:expr, $h:expr) => {{
        // use crate::hex_str_to_u8s;
        use std::io::Write;
        _ = $s.write_all(
            &$crate::hex_str_to_u8s($h).expect("Invalid hex code provided for integration test."),
        );
    }};
}

#[macro_export]
macro_rules! assert_client_receives_bytes (
    ($s:expr, $h:expr, $d:expr) => {{
        use std::io::Read;
        use std::time::Duration;

        let client = &mut $s;
        let bytes = $crate::hex_str_to_u8s($h).expect("Invalid hex code provided for integration test.");
        let mut buffer: Vec<u8> = Vec::new();
        client.set_read_timeout(Some($d)).expect("Could not set read timeout.");
        match client.take(bytes.len() as u64).read_to_end(&mut buffer) {
            Err(e)  => panic!("Client connection errored: {e:?}"),
            Ok(_) => assert_eq!(bytes, buffer),
        };
        client.set_read_timeout(None).expect("Could not unset read timeout.");
    }};
    ($s:expr, $h:expr) => {{
        use std::io::Read;
        use std::time::Duration;

        let client = &mut $s;
        let bytes = $crate::hex_str_to_u8s($h).expect("Invalid hex code provided for integration test.");
        let mut buffer: Vec<u8> = Vec::new();
        client.set_read_timeout(Some(Duration::from_secs(1))).expect("Could not set read timeout.");
        match client.take(bytes.len() as u64).read_to_end(&mut buffer) {
            Err(e)  => panic!("Client connection errored: {e:?}"),
            Ok(_) => assert_eq!(bytes, buffer),
        };
        client.set_read_timeout(None).expect("Could not unset read timeout.");
    }};
);

#[macro_export]
macro_rules! assert_client_not_receives_bytes (
    ($s:expr, $h:expr, $d:expr) => {};
);
