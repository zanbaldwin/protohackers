use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};

const BUFFER_SIZE: usize = 4096;

// Smoke Test (Echo Server)
pub fn handle_stream(mut stream: TcpStream) {
    let mut contents: Vec<u8> = vec![];
    let mut buffer = [0u8; BUFFER_SIZE];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                stream.write_all(&contents).unwrap();
                stream.flush().unwrap();
                stream.shutdown(Shutdown::Both).unwrap();
                return;
            },
            Ok(n) => contents.extend_from_slice(&buffer[..n]),
            Err(err) => panic!("Error processing stream: {err:?}"),
        }
    }
}
