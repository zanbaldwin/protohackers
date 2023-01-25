use protozackers::{server, THREAD_SLOW_DOWN};
use std::cmp::Ordering;
use std::collections::{HashMap, BTreeMap};
use std::thread;

const BUFFER_SIZE: usize = 1_000;
// When testing with netcat, strip newlines that are added to the ends of packets.
const SHOULD_HANDLE_NEWLINES: bool = false;

struct Session {
    id: i32, // Greater than zero.
    data: BTreeMap<i32, Vec<u8>>,
}
impl Session {
    fn new(id: i32) -> Self {
        Self { id, data: BTreeMap::new() }
    }
    fn is_missing_data(&self) -> bool {
        false
    }
    fn get_session_length(&self) -> usize {
        self.data.iter()
            .map(|(_, payload)| payload.len())
            .fold(0, |accumulator, item| accumulator + item)
    }
    fn get_largest_packet_length(&self) -> usize {
        self.data.iter()
            .map(|(_, payload)| payload.len())
            .fold(0, |accumulator, item| std::cmp::max(accumulator, item))
    }
    fn get_total_payload(&self) -> Result<Vec<u8>, ()> {
        if self.is_missing_data() {
            return Err(());
        }
        Ok(Vec::new())
    }
    fn get_ack_message(&self) -> String {
        format!("/ack/{}/{}", self.id, self.get_session_length())
    }
}

enum Command {
    Connect(i32),
    Data(i32, usize, Vec<u8>),
    Ack(i32, usize),
    Close(i32),
}
impl Command {
    pub fn parse_from_request(mut request: Vec<u8>) -> Result<Self, ()> {
        Err(())
    }
}

type Sessions = HashMap<i32, Session>;

fn main() {
    let socket = server::get_udp_listener(None);
    let mut sessions: Sessions = HashMap::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        match socket.recv_from(&mut buffer) {
            Ok((length, source)) => {
                let mut request: Vec<u8> = vec![];
                request.extend_from_slice(&buffer[..length]);

                if SHOULD_HANDLE_NEWLINES {
                    if let Some(&b'\n') = request.last() {
                        request.pop();
                    }
                }

                if let Ok(command) = Command::parse_from_request(request) {
                    match command {
                        Command::Connect(session_id) => {
                            let session = Session::new(session_id);
                            _ = socket.send_to(session.get_ack_message().as_bytes(), source);
                            sessions.insert(session_id, session);
                        },
                        Command::Data(session, position, data) => todo!(),
                        Command::Ack(session_id, length) => {
                            if let Some(session) = sessions.get(&session_id) {
                                if length > session.get_largest_packet_length() {
                                    match session.get_session_length().cmp(&length) {
                                        Ordering::Less => todo!("Dump"),
                                        Ordering::Equal => (),
                                        Ordering::Greater => todo!("Close session"),
                                    };
                                }
                            } else {
                                _ = socket.send_to(format!("/close/{session_id}/").as_bytes(), source);
                            }
                        },
                        Command::Close(session_id) => {
                            if let Some(session) = sessions.get(&session_id) {
                                let close_message = format!("/close/{}/", session.id);
                                _ = sessions.remove(&session_id);
                                _ =socket.send_to(close_message.as_bytes(), source);
                            }
                        },
                    };
                }
            },
            Err(_) => continue,
        };

        thread::sleep(THREAD_SLOW_DOWN);
    }
}
