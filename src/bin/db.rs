use protozackers::{server, THREAD_SLOW_DOWN};
use std::collections::HashMap;
use std::thread;

const BUFFER_SIZE: usize = 1_000;
const VERSION_KEY: &[u8] = b"version";
const VERSION_STRING: &[u8] = b"Zan's Key-Value Store 0.1.0";
// When testing with netcat, strip newlines that are added to the ends of packets.
const SHOULD_HANDLE_NEWLINES: bool = false;

struct Database {
    items: HashMap<Vec<u8>, Vec<u8>>,
}
impl Database {
    fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }
    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        if key.as_slice() == VERSION_KEY {
            return;
        }
        self.items.insert(key, value);
    }
    fn query(&self, key: &[u8]) -> Option<&[u8]> {
        if key == VERSION_KEY {
            return Some(VERSION_STRING);
        }
        match self.items.get(key) {
            Some(value) => Some(value.as_slice()),
            None => None,
        }
    }
}

fn main() {
    let socket = server::get_udp_listener(None);
    let mut database = Database::new();

    let mut buffer = [0u8; BUFFER_SIZE];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((length, source)) => {
                let mut request: Vec<u8> = vec![];
                request.extend_from_slice(&buffer[..length]);

                if SHOULD_HANDLE_NEWLINES {
                    if let Some(&b'\n') = request.last() {
                        request.pop();
                    }
                }

                if let Some(position) = request.iter().position(|&byte| byte == b'=') {
                    let mut key: Vec<u8> = vec![];
                    key.extend_from_slice(request.drain(..position + 1).as_slice());
                    // Remove the assignment operator (=).
                    key.pop();
                    database.insert(key, request);
                } else if let Some(value) = database.query(&request) {
                    let mut response: Vec<u8> = vec![];
                    response.extend_from_slice(&request);
                    response.push(b'=');
                    response.extend_from_slice(value);
                    if SHOULD_HANDLE_NEWLINES {
                        response.push(b'\n');
                    }
                    let _ = socket.send_to(&response, source);
                }
            }
            Err(_) => continue,
        };
        thread::sleep(THREAD_SLOW_DOWN);
    }
}
