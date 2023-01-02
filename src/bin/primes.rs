extern crate primes;
extern crate serde_json;

use protozackers::{server, ASCII_NEWLINE, BUFFER_SIZE};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};

fn main() {
    server::run(handle_stream, None);
}

const MALFORMED_RESPONSE: [u8; 5] = [69, 82, 82, 79, 82]; // "ERROR"
const SHOULD_ERROR_ON_TRAILING: bool = true;

#[derive(Deserialize, Debug)]
struct PrimeRequest {
    method: String,
    number: f64,
}

#[derive(Serialize, Debug)]
struct PrimeResponse {
    method: String,
    prime: bool,
}

pub fn handle_stream(mut stream: TcpStream) {
    let mut lines: Vec<u8> = vec![];
    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

    loop {
        // Handle JSON lines.
        while let Some(position) = lines.iter().position(|&byte| byte == ASCII_NEWLINE) {
            let json_bytes = &lines[0..position];
            let prime_request: PrimeRequest = match serde_json::from_slice(json_bytes) {
                Ok(request) => request,
                Err(_) => {
                    malformed_request(stream);
                    return;
                }
            };
            if prime_request.method != *"isPrime" {
                malformed_request(stream);
                return;
            }
            let prime_response: PrimeResponse = PrimeResponse {
                method: "isPrime".to_string(),
                prime: is_prime(prime_request.number),
            };
            let mut response_str = match serde_json::to_string(&prime_response) {
                Ok(string) => string,
                Err(err) => panic!("{:?}", err),
            };
            response_str.push('\n');
            stream.write_all(&response_str.into_bytes()).unwrap();

            lines.drain(0..position + 1);
        }

        match stream.read(&mut buffer) {
            Ok(0) => {
                end_request(stream, &lines);
                return;
            }
            Ok(n) => lines.extend_from_slice(&buffer[0..n]),
            Err(err) => panic!("{:?}", err),
        };
    }
}

fn malformed_request(mut stream: TcpStream) {
    stream.write(&MALFORMED_RESPONSE).unwrap_or_default();
    stream.shutdown(Shutdown::Both).unwrap_or_default();
}

fn end_request(mut stream: TcpStream, lines: &Vec<u8>) {
    if SHOULD_ERROR_ON_TRAILING && !lines.is_empty() {
        stream.write(&MALFORMED_RESPONSE).unwrap_or_default();
    }
    stream.shutdown(Shutdown::Both).unwrap_or_default();
}

fn is_prime(number: f64) -> bool {
    if number.fract() == 0.0 {
        // Waaay to lazy to do this manually.
        // See primes repository for a badly-written Aktins sieve implementation.
        primes::is_prime(number as u64)
    } else {
        false
    }
}
