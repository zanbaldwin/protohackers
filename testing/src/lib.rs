use std::net::{TcpListener, TcpStream};

pub fn listen_on_available_port() -> (TcpListener, u16) {
    let Ok(listener) = TcpListener::bind(("127.0.0.1", 0)) else {
        panic!("Could not find an available port to run integration tests.");
    };
    let Ok(addr) = listener.local_addr() else {
        panic!("Could not determine OS-assigned port.");
    };
    println!(
        "Integration test: running application on port {}.",
        addr.port()
    );
    (listener, addr.port())
}

pub fn connect(port: u16) -> TcpStream {
    let stream =
        TcpStream::connect(("127.0.0.1", port)).expect("Could not connect to integration server.");
    // TODO: Does this need to be non-blocking?
    stream
        .set_nonblocking(true)
        .expect("Could not set stream to non-blocking.");
    stream
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

        let client = &mut $s;
        client.set_read_timeout(Some($d)).expect("Could not set read timeout.");

        let bytes = $crate::hex_str_to_u8s($h).expect("Invalid hex code provided for integration test.");
        let mut reader = client.take(bytes.len() as u64);
        let mut payload: Vec<u8> = ::std::vec::Vec::with_capacity(bytes.len());
        let mut buffer = [0u8; 512];

        let now = ::std::time::Instant::now();
        loop {
            match reader.read(&mut buffer) {
                Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => (),
                Err(e)  => panic!("Client connection errored: {e:?}"),
                Ok(0) => panic!("Client connection dropped."),
                Ok(n) => {
                    payload.extend_from_slice(&buffer[..n]);
                    if payload.len() >= bytes.len() {
                        break;
                    }
                },
            };
            if now.elapsed() > $d {
                panic!("Timeout reached waiting for expected payload.");
            }
            // Don't hog the CPU core.
            ::std::thread::sleep(::std::time::Duration::from_millis(1));
        }
        assert_eq!(&bytes, &payload[..bytes.len()]);
        client.set_read_timeout(None).expect("Could not unset read timeout.");
    }};
);
