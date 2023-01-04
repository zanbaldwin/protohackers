extern crate uuid;

use protozackers::{server, ASCII_NEWLINE, BUFFER_SIZE, SLOW_DOWN_MILLISECONDS};
use std::collections::HashMap;
use std::io::{Read, Write, ErrorKind};
use std::net::{Shutdown, TcpStream};
use std::thread;
use std::time::Duration;
use uuid::Uuid;
use std::sync::mpsc::{self, Sender};

const WELCOME_MESSAGE: &str = "Welcome to budgetchat! What shall I call you?\n";

fn string_to_vec(bytes: String) -> Vec<u8> {
    let mut vec: Vec<u8> = vec![];
    vec.extend_from_slice(bytes.as_bytes());
    vec
}

#[derive(Debug)]
enum Command {
    Join(Uuid, String),
    Leave(Uuid, String),
    Message(Uuid, String, Vec<u8>),
}
impl Command {
    fn get_broadcast(&self) -> Vec<u8> {
        let mut vec = match self {
            Command::Join(_, name) => string_to_vec(format!("* {name} has entered the room")),
            Command::Leave(_, name) => string_to_vec(format!("* {name} has left the room")),
            Command::Message(_, name, input) => {
                let mut vec = string_to_vec(format!("[{name}] "));
                vec.extend_from_slice(input);
                vec
            },
        };
        vec.push(b'\n');
        vec
    }
}

struct Client {
    id: Uuid,
    stream: TcpStream,
    name: Option<String>,
}
impl Client {
    fn new(stream: TcpStream) -> Self {
        Self { id: Uuid::new_v4(), stream, name: None }
    }
    fn has_joined(&self) -> bool {
        self.name.is_some()
    }
    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }
}

fn main() {
    let listener = server::get_tcp_listener(None);
    let mut clients: HashMap<Uuid, Client> = HashMap::new();
    let (transmitter, receiver) = mpsc::channel::<Command>();

    loop {
        // Accept new connection, and spawn handler.
        if let Ok((stream, _remote_addr)) = listener.accept() {
            eprintln!("Accepting new TCP connection: {stream:?}");
            let client_stream = match stream.try_clone() {
                Ok(stream) => stream,
                Err(_) => continue,
            };
            let client: Client = Client::new(client_stream);
            let client_id: Uuid = client.id.to_owned();
            let client_transmitter: Sender<Command> = transmitter.clone();

            clients.insert(client_id.to_owned(), client);

            thread::spawn(move || {
                handle_stream(client_id, stream, client_transmitter);
            });
        }

        // Check for inter-thread commands.
        if let Ok(command) = receiver.try_recv() {
            eprintln!("Received command: {command:?}");
            handle_command(command, &mut clients);
        }

        thread::sleep(Duration::from_millis(SLOW_DOWN_MILLISECONDS));
    }
}

fn handle_command(command: Command, clients: &mut HashMap<Uuid, Client>) {
    let broadcast_message = command.get_broadcast();
    match command {
        Command::Join(id, name) => {
            let existing_names: Vec<String> = clients.iter()
                .filter(|(client_id, client)| client_id != &&id && client.has_joined())
                .map(|(_, client)| client.name.to_owned().unwrap())
                .collect();
            if let Some(client) = clients.get_mut(&id) {
                client.set_name(name);
                let _ = client.stream.write_all(&string_to_vec(format!("* The room contains: {}\n", existing_names.join(", "))));
            }
            broadcast_to_joined_clients_except(clients, id, &broadcast_message);
        },
        Command::Leave(id, _) => {
            clients.remove(&id);
            broadcast_to_joined_clients_except(clients, id, &broadcast_message);
        },
        Command::Message(id, _, _) => {
            broadcast_to_joined_clients_except(clients, id, &broadcast_message);
        },
    }
}

fn broadcast_to_joined_clients_except(clients: &mut HashMap<Uuid, Client>, except: Uuid, message: &[u8]) {
    for (client_id, client) in clients {
        if client_id != &except && client.has_joined() {
            let _ = client.stream.write_all(message);
        }
    }
}

fn handle_stream(id: Uuid, mut stream: TcpStream, transmitter: Sender<Command>) {
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut queue: Vec<u8> = vec![];
    let mut display_name: Option<String> = None;

    let _ = stream.write_all(WELCOME_MESSAGE.as_bytes());

    'connected: loop {
        // Process queue.
        while let Some(position) = queue.iter().position(|&byte| byte == ASCII_NEWLINE) {
            let mut line: Vec<u8> = vec![];
            line.extend_from_slice(queue.drain(..position + 1).as_slice());
            line.pop();

            let command: Command = match &display_name {
                Some(name) => Command::Message(id.to_owned(), name.to_owned(), line),
                None => match validate_name(line) {
                    Ok(name) => {
                        display_name = Some(name.to_owned());
                        Command::Join(id.to_owned(), name)
                    },
                    Err(_) => break 'connected,
                },
            };

            transmitter.send(command).expect("Failed to send command to main thread.");
        }

        // Read input stream.
        match stream.read(&mut buffer) {
            Ok(0) => {
                if let Some(name) = display_name {
                    transmitter.send(Command::Leave(id.to_owned(), name)).expect("Failed to send command to main thread.");
                }
                break 'connected;
            },
            Ok(n) => queue.extend_from_slice(&buffer[..n]),
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => break 'connected,
        }

        // Wait.
        thread::sleep(Duration::from_millis(SLOW_DOWN_MILLISECONDS));
    }

    let _ = stream.shutdown(Shutdown::Both);
}

fn validate_name(line: Vec<u8>) -> Result<String, ()> {
    if let Ok(name) = String::from_utf8(line) {
        if name.chars().all(char::is_alphanumeric) && !name.is_empty() {
            return Ok(name);
        }
    }
    Err(())
}
