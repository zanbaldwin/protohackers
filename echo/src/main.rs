use common::{run, BUFFER_SIZE};
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};

fn main() {
    run(handle_stream_buffer, None, false);
}

// Smoke Test (Echo Server)
pub fn handle_stream_buffer(mut stream: TcpStream) {
    let mut contents: Vec<u8> = vec![];
    let mut buffer = [0u8; BUFFER_SIZE];
    'connected: loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                _ = stream.write_all(&contents);
                break 'connected;
            }
            Ok(n) => contents.extend_from_slice(&buffer[..n]),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => (),
            Err(err) => panic!("Error processing stream: {err:?}"),
        }
    }

    _ = stream.shutdown(Shutdown::Both);
}

// Smoke Test (HexCat)
pub fn handle_stream_immediate(mut stream: TcpStream) {
    let mut buffer = [0u8; BUFFER_SIZE];
    'connected: loop {
        match stream.read(&mut buffer) {
            Ok(0) => break 'connected,
            Ok(n) => {
                if stream.write_all(&buffer[..n]).is_err() {
                    break 'connected;
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => (),
            Err(err) => panic!("Error processing stream: {err:?}"),
        }
    }

    _ = stream.shutdown(Shutdown::Both);
}
