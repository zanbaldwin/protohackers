use crate::{
    io::{ClientInput, Message, ServerOutput},
    parser, utils,
};
use common::{BUFFER_SIZE, THREAD_SLOW_DOWN};
use std::io::{ErrorKind, Read};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::Sender;
use std::thread;
use uuid::Uuid;

pub(crate) fn connection(id: Uuid, mut stream: TcpStream, transmitter: Sender<Message>) {
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut queue: Vec<u8> = Vec::new();
    let mut parse = false;

    let end_reason: ClientInput;
    'connected: loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                end_reason = ClientInput::StreamEnded;
                break 'connected;
            }
            Ok(n) => {
                parse = true;
                queue.extend_from_slice(&buffer[..n]);
                eprintln!("{id}: {}", utils::u8s_to_hex_str(&buffer[..n]));
            }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                end_reason = ClientInput::StreamErrored;
                break 'connected;
            }
        };

        if parse {
            parse = false;
            'parse: loop {
                match parser::nom(&queue) {
                    // Not enough data has been received by the TCP stream, go back and fetch more.
                    Ok(None) => break 'parse,
                    Ok(Some((input, drain))) => {
                        _ = transmitter.send(Message { from: id, input });
                        queue.drain(..drain);
                    }
                    Err(_) => {
                        end_reason = ClientInput::StreamErrored;
                        break 'connected;
                    }
                }
            }
        }

        thread::sleep(THREAD_SLOW_DOWN);
    }

    _ = transmitter.send(Message {
        from: id,
        input: end_reason,
    });
}

pub(crate) fn heartbeat(mut stream: TcpStream, interval: std::time::Duration) {
    'heartbeat: loop {
        thread::sleep(interval);
        if !ServerOutput::Heartbeat.write(&mut stream) {
            _ = stream.shutdown(Shutdown::Both);
            break 'heartbeat;
        }
    }
}
