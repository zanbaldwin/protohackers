mod server;
mod echo;
mod prime;
mod keystore;
mod chat;

use std::{env, process::exit};

const DEFAULT_PORT: u16 = 8096;
const BUFFER_SIZE: usize = 4096;
const ASCII_NEWLINE: u8 = 10;

fn main() {
    let args: Vec<String> = env::args().collect();
    let function_name: &str = if args.len() >= 2 {args[1].as_str() } else { help() };
    let port: u16 = if args.len() >= 3 { args[2].parse::<u16>().expect("Invalid Port Number.") } else { DEFAULT_PORT };

    let stream_handler = match function_name {
        "echo" => echo::handle_stream,
        "prime" => prime::handle_stream,
        "keystore" => keystore::handle_stream,
        "chat" => chat::handle_stream,
        _ => help(),
    };

    server::run(port, stream_handler);
}

fn help() -> ! {
    eprintln!("Usage");
    eprintln!("    protozackers <function> [<port>]");
    eprintln!("");
    eprintln!("Valid functions are:");
    eprintln!("    echo     (Problem 0: Smoke Test)");
    eprintln!("    primes   (Problem 1: Prime Time)");
    eprintln!("    keystore (Problem 2: Means to an End)");
    eprintln!("    chat     (Problem 3: Budget Chat)");
    eprintln!("");
    eprintln!("If not specified, the default port is {DEFAULT_PORT}.");
    exit(1);
}