use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;

pub fn run<F>(port: u16, stream_handler: F) where
    F: Fn(TcpStream) + Clone + Send + Sync + 'static
{
    let address: SocketAddr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener: TcpListener = TcpListener::bind(address).expect("Could not bind to port.");
    println!("Listening to connections on port {port}...");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let thread_handler = stream_handler.clone();
                thread::spawn(move || {
                    thread_handler(stream);
                });
            },
            Err(err) => eprintln!("Incoming TCP connection stream errored... {err:?}"),
        };
    }
}