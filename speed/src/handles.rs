use crate::{
    app::Application,
    io::{ClientInput, Message, ServerOutput},
    utils,
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

    let end_reason: ClientInput;
    'connected: loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                end_reason = ClientInput::StreamEnded;
                break 'connected;
            }
            Ok(n) => {
                queue.extend_from_slice(&buffer[..n]);
                eprintln!("{id}: {}", utils::u8s_to_hex_str(&buffer[..n]));
            }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                end_reason = ClientInput::StreamErrored;
                break 'connected;
            }
        };

        'parse: loop {
            match Application::parse_input_buffer(&mut queue) {
                Ok(None) => break 'parse,
                Ok(Some(input)) => _ = transmitter.send(Message { from: id, input }),
                Err(_) => {
                    end_reason = ClientInput::StreamErrored;
                    break 'connected;
                }
            };
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
