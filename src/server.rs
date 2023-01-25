use crate::{DEFAULT_PORT, THREAD_SLOW_DOWN};

use std::env;
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::thread;

pub fn get_tcp_listener(port: Option<u16>) -> TcpListener {
    let port = match port {
        Some(port) => port,
        None => get_port(),
    };
    let address: SocketAddr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener: TcpListener = TcpListener::bind(address).expect("Could not bind to port.");
    listener
        .set_nonblocking(true)
        .expect("Could not set TCP listener as non-blocking.");
    println!("Listening for TCP connections on port {port}...");
    listener
}

pub fn get_udp_listener(port: Option<u16>) -> UdpSocket {
    let port = match port {
        Some(port) => port,
        None => get_port(),
    };
    let address: SocketAddr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener: UdpSocket = UdpSocket::bind(address).expect("Could not bind to port.");
    eprintln!("Listening to UDP connections on port {port}...");
    listener
}

pub fn run<F>(stream_handler: F, port: Option<u16>, blocking: bool) -> !
where
    F: Fn(TcpStream) + Clone + Send + Sync + 'static,
{
    let listener = get_tcp_listener(port);
    loop {
        if let Ok((stream, _)) = listener.accept() {
            let thread_handler = stream_handler.clone();
            match blocking {
                true => thread_handler(stream),
                false => _ = thread::spawn(move || thread_handler(stream)),
            };
        }
        thread::sleep(THREAD_SLOW_DOWN);
    }
}

pub fn get_port() -> u16 {
    let args: Vec<String> = env::args().collect();
    let port: u16 = if args.len() >= 2 {
        args[1].parse::<u16>().expect("Invalid Port Number.")
    } else {
        DEFAULT_PORT
    };
    port
}
