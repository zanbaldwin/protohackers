use crate::error::Error;
use crate::models::io::{ClientInput, ClientMessage, ServerMessage, ServerOutput};
use crate::utils::u8s_to_hex_str;
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpListener,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{info, warn};
use uuid::Uuid;

mod error;
mod models;
mod parser;
pub(crate) mod utils;

const BUFFER_SIZE: usize = 4096;

const SPEED_ERROR_MARGIN: f32 = 0.4;
const DAY_IN_SECONDS: u32 = 86_400;

type ByteStr<'a> = &'a [u8];
type ByteString = Vec<u8>;

pub(crate) const MESSAGE_TYPE_ERROR: u8 = 0x10;
pub(crate) const MESSAGE_TYPE_PLATE: u8 = 0x20;
pub(crate) const MESSAGE_TYPE_TICKET: u8 = 0x21;
pub(crate) const MESSAGE_TYPE_WANT_HEARTBEAT: u8 = 0x40;
pub(crate) const MESSAGE_TYPE_HEARTBEAT: u8 = 0x41;
pub(crate) const MESSAGE_TYPE_AM_CAMERA: u8 = 0x80;
pub(crate) const MESSAGE_TYPE_AM_DISPATCHER: u8 = 0x81;

pub(crate) type SpeedMph = f32;
pub(crate) type PlateNumber = ByteString;

#[derive(Default)]
pub struct Application {
    write_connections: HashMap<Uuid, OwnedWriteHalf>,
    heartbeats: HashMap<Uuid, JoinHandle<()>>,
    // pending_tickets: HashMap<u16, Vec<Ticket>>,
    // reports: HashMap<PlateNumber, Report>,
}
impl Application {
    async fn run(mut self, listener: TcpListener) -> ! {
        let (send_incoming, receive_incoming) = channel(BUFFER_SIZE);
        let (send_outgoing, receive_outgoing) = channel(BUFFER_SIZE);

        tokio::spawn(self.handle_outgoing_messages(receive_outgoing));
        tokio::spawn(self.handle_messages(receive_incoming, send_outgoing));

        'run: loop {
            if let Ok((mut stream, socket)) = listener.accept().await {
                let (read_stream, write_stream) = stream.into_split();
                let id = uuid::Uuid::new_v4();
                self.write_connections.insert(id.clone(), write_stream);
                tokio::spawn(self.handle_incoming_messages(id, read_stream, send_incoming.clone()));
            }
        }
    }

    async fn handle_incoming_messages(&self, id: Uuid, mut stream: OwnedReadHalf, sender: Sender<ClientMessage>) {
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut queue: Vec<u8> = Vec::new();
        'read: loop {
            match stream.read(&mut buffer).await {
                // See https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read
                Ok(0) => break 'read,
                Ok(n) => {
                    queue.extend_from_slice(&buffer[..n]);
                    info!("Received from {id}: {}", utils::u8s_to_hex_str(&buffer[..n]));
                    if !self.parse(&id, &mut queue, &sender).await {
                        break 'read;
                    }
                },
                Err(_) => {
                    _ = sender.send(Self::client_error_msg(id, Error::InvalidStream)).await;
                    break 'read;
                },
            };
        }
    }

    async fn handle_outgoing_messages(&mut self, mut receiver: Receiver<ServerMessage>) {
        'write: loop {
            if let Some(message) = receiver.recv().await {
                if let Some(conn) = self.write_connections.get_mut(&message.to) {
                    let output = message.output.to_bytes();
                    if let Err(_) = conn.write_all(&output).await {
                        self.write_connections.remove(&message.to);
                        warn!("Shutdown connection {} due to write error.", message.to);
                        continue 'write;
                    };
                    info!(">>> {}", u8s_to_hex_str(&output));
                    if let ServerOutput::Error(e) = message.output {
                        _ = conn.shutdown().await;
                        self.write_connections.remove(&message.to);
                        warn!("Shutdown connection {} due to client error ({:?}).", message.to, e);
                    }
                }
            }
        }
    }

    async fn handle_messages(&mut self, mut receive: Receiver<ClientMessage>, sender: Sender<ServerMessage>) {
        'message: loop {
            if let Some(message) = receive.recv().await {
                match message.input {
                    ClientInput::Error(e) => {
                        _ = sender
                            .send(ServerMessage {
                                to: message.from,
                                output: ServerOutput::Error(e),
                            })
                            .await
                    },
                    ClientInput::WantHeartbeat(deciseconds) => {
                        if let Some(heartbeat) = self.heartbeats.get(&message.from) {
                            heartbeat.abort();
                        }
                        self.heartbeats.insert(
                            message.from,
                            tokio::spawn(Self::heartbeat(deciseconds as u64, message.from.clone(), sender.clone())),
                        );
                    },
                    ClientInput::IAmCamera(road, mile, limit) => todo!(),
                    ClientInput::IAmDispatcher(roads) => todo!(),
                    ClientInput::Plate(plate, timestamp) => todo!(),
                }
            }
        }
    }

    fn client_error_msg(id: Uuid, error: Error) -> ClientMessage {
        ClientMessage {
            from: id,
            input: ClientInput::Error(error),
        }
    }

    async fn parse(&self, id: &Uuid, queue: &mut Vec<u8>, sender: &Sender<ClientMessage>) -> bool {
        'parse: loop {
            match parser::nom(&queue) {
                // Not enough data has been received by the TCP stream, go back and fetch more.
                Ok(None) => break 'parse,
                Ok(Some((input, drain))) => {
                    _ = sender.send(ClientMessage { from: id.clone(), input });
                    queue.drain(..drain);
                },
                Err(e) => {
                    _ = sender.send(Self::client_error_msg(id.clone(), e)).await;
                    return false;
                },
            }
        }
        true
    }

    async fn heartbeat(deciseconds: u64, id: Uuid, sender: Sender<ServerMessage>) {
        let mut interval = interval(Duration::from_millis(deciseconds * 100));
        'heartbeat: loop {
            interval.tick().await;
            _ = sender
                .send(ServerMessage {
                    to: id.clone(),
                    output: ServerOutput::Heartbeat,
                })
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
