use protozackers::{server, BUFFER_SIZE};
use std::io::{Read, Write, ErrorKind};
use std::net::{Shutdown, TcpStream};

fn main() {
    server::run(handle_stream, None, false);
}

// Smoke Test (Echo Server)
pub fn handle_stream(mut stream: TcpStream) {
    let mut contents: Vec<u8> = vec![];
    let mut buffer = [0u8; BUFFER_SIZE];
    'connected: loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                let _ = stream.write_all(&contents);
                break 'connected;
            }
            Ok(n) => contents.extend_from_slice(&buffer[..n]),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => (),
            Err(err) => panic!("Error processing stream: {err:?}"),
        }
    }

    let _ = stream.shutdown(Shutdown::Both);
}
