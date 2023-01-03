extern crate primes;
extern crate serde_json;

use protozackers::{server, ASCII_NEWLINE, BUFFER_SIZE};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write, ErrorKind};
use std::net::{Shutdown, TcpStream};

fn main() {
    server::run(handle_stream, None);
}

const MALFORMED_RESPONSE: [u8; 5] = [69, 82, 82, 79, 82]; // "ERROR"

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
    let mut queue: Vec<u8> = vec![];
    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

    'connected: loop {
        match stream.read(&mut buffer) {
            Ok(0) => break 'connected,
            Ok(n) => queue.extend_from_slice(&buffer[0..n]),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => (),
            Err(err) => panic!("Error processing stream: {err:?}"),
        };

        while let Some(position) = queue.iter().position(|&byte| byte == ASCII_NEWLINE) {
            let mut line: Vec<u8> = vec![];
            line.extend_from_slice(queue.drain(..position + 1).as_slice());

            let response: Vec<u8> = match process_json(&line) {
                Ok(response) => response,
                Err(response) => {
                    let _ = stream.write_all(response);
                    break 'connected;
                },
            };

            let _ = stream.write_all(&response);
        }
    }

    let _ = stream.shutdown(Shutdown::Both);
}

fn process_json(json: &[u8]) -> Result<Vec<u8>, &[u8; 5]> {
    let err: Result<Vec<u8>, &[u8 ;5]> = Err(&MALFORMED_RESPONSE);
    let request: PrimeRequest = match serde_json::from_slice(json) {
        Ok(request) => request,
        Err(_) => return err,
    };
    if request.method != *"isPrime" {
        return err;
    }
    let result = PrimeResponse {
        method: "isPrime".to_string(),
        prime: if request.number.fract() == 0.0 { primes::is_prime(request.number as u64) } else { false },
    };
    let response: String = match serde_json::to_string(&result) {
        Ok(response) => response,
        Err(_) => return err,
    };
    Ok(response.into_bytes())
}
