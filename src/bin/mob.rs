use core::panic;
use protozackers::{server, BUFFER_SIZE, THREAD_SLOW_DOWN};
use regex::bytes::Regex;
use std::borrow::Cow;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::thread;

const UPSTREAM_SERVER: &str = "chat.protohackers.com:16963";
const BOGUSCOIN_MATCHER: &str = "^7[a-zA-Z0-9]{25,34}$";
const TONY_BOGUSCOIN_ADDRESS: &[u8] = b"7YWHMfk9JZe0LM0g1ZauHuiSxhI";

struct Spoofer {
    re: Regex,
}

fn main() {
    let listener = server::get_tcp_listener(None);
    loop {
        if let Ok((victim, _)) = listener.accept() {
            let upstream: TcpStream = match TcpStream::connect(UPSTREAM_SERVER) {
                Ok(stream) => stream,
                Err(_) => {
                    _ = victim.shutdown(Shutdown::Both);
                    continue;
                }
            };

            let victim_writer = match victim.try_clone() {
                Ok(stream) => stream,
                Err(_) => continue,
            };
            let upstream_writer = match upstream.try_clone() {
                Ok(stream) => stream,
                Err(_) => continue,
            };

            thread::spawn(move || handle_stream(upstream, victim_writer));
            thread::spawn(move || handle_stream(victim, upstream_writer));
        }
        thread::sleep(THREAD_SLOW_DOWN);
    }
}

fn handle_stream(mut upstream: TcpStream, mut downstream: TcpStream) {
    let spoofer = Spoofer::new();
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut queue: Vec<u8> = Vec::new();
    loop {
        match upstream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => queue.extend_from_slice(&buffer[..n]),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => (),
            Err(_) => break,
        }

        while let Some(position) = queue.iter().position(|&byte| byte == b'\n') {
            _ = downstream.write_all(&spoofer.replace(queue.drain(..position + 1).as_slice()));
        }

        thread::sleep(THREAD_SLOW_DOWN);
    }
    _ = upstream.shutdown(Shutdown::Both);
    _ = downstream.shutdown(Shutdown::Both);
}

impl Spoofer {
    fn new() -> Self {
        Self {
            re: match Regex::new(BOGUSCOIN_MATCHER) {
                Ok(re) => re,
                Err(_) => panic!("Invalid Regular Expression."),
            },
        }
    }

    fn replace(&self, buffer: &[u8]) -> Vec<u8> {
        // The Rust crate "regex" does not support lookahead/lookbehind;
        // split into lines then words before PCRE replace.
        let mut lines: Vec<Vec<u8>> = Vec::new();
        for line in buffer.split(|byte| byte == &b'\n') {
            let spoofed_line = line
                .split(|byte| byte == &b' ')
                .map(|word| match self.re.replace(word, TONY_BOGUSCOIN_ADDRESS) {
                    Cow::Owned(vec) => vec,
                    Cow::Borrowed(buffer) => {
                        let mut result: Vec<u8> = Vec::new();
                        result.extend_from_slice(buffer);
                        result
                    }
                })
                .collect::<Vec<Vec<u8>>>()
                .join(&b' ');
            lines.push(spoofed_line);
        }
        lines.join(&b'\n')
    }
}
