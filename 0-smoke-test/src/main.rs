use std::env;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::thread;

const DEFAULT_PORT: u16 = 8096;
const BUFFER_SIZE: usize = 4096;

fn main() {
    let args: Vec<String> = env::args().collect();
    let port: u16 = if args.len() >= 2 { args[1].parse::<u16>().expect("Invalid Port Number.") } else { DEFAULT_PORT };
    echo(port);
}

fn echo(port: u16) {
    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).unwrap();
    println!("Starting Echo Server for Smoke Test on port {port}...");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("Handling incoming connection...");
                thread::spawn(|| {
                    handle_tcp_string(stream);
                });
            },
            Err(err) => {
                print!("Incoming TCP connection stream errored... ");
                println!("{:?}", err)
            }
        }
    }
}

fn handle_tcp_string(mut stream: TcpStream) {
    let mut contents: Vec<u8> = vec![];
    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                write_back(stream, contents).unwrap_or_default();
                return;
            },
            Ok(n) => contents.extend_from_slice(&buffer[0..n]),
            Err(err) => panic!("{:?}", err),
        };
    }
}

fn write_back(mut stream: TcpStream, contents: Vec<u8>) -> Result<(), std::io::Error> {
    stream.write_all(&contents)?;
    stream.flush()?;
    stream.shutdown(Shutdown::Write)?;
    Ok(())
}
