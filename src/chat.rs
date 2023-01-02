extern crate serde_json;
extern crate primes;

use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};

use crate::{ASCII_NEWLINE, BUFFER_SIZE};

pub fn handle_stream(mut stream: TcpStream) {
}