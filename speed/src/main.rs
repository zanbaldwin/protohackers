extern crate uuid;

use common::{get_tcp_listener, BUFFER_SIZE, THREAD_SLOW_DOWN};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

const SPEED_ERROR_MARGIN: f32 = 0.4;
const DAY_IN_SECONDS: u32 = 86_400;

const MESSAGE_TYPE_ERROR: u8 = 0x10;
const MESSAGE_TYPE_PLATE: u8 = 0x20;
const MESSAGE_TYPE_TICKET: u8 = 0x21;
const MESSAGE_TYPE_WANT_HEARTBEAT: u8 = 0x40;
const MESSAGE_TYPE_HEARTBEAT: u8 = 0x41;
const MESSAGE_TYPE_AM_CAMERA: u8 = 0x80;
const MESSAGE_TYPE_AM_DISPATCHER: u8 = 0x81;

type RoadId = u16;
type MileMarker = u16;
type SpeedLimit = u16;
type RecordedSpeed = u16;
type SpeedMph = f32;
type Timestamp = u32;
type PlateNumber = Vec<u8>;
type HeartbeatInterval = u32;
type BufferMatch = (Result<Option<ClientInput>, ()>, usize);
type IssuedTickets = HashMap<Vec<u8>, Vec<u32>>;

struct Camera {
    road: RoadId,
    mile_marker: MileMarker,
    limit: SpeedLimit,
}
struct Dispatcher {
    roads: Vec<RoadId>,
}
enum Client {
    Camera(Camera),
    Dispatcher(Dispatcher),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Report {
    plate: PlateNumber,
    road: RoadId,
    timestamp: Timestamp,
    mile_marker: MileMarker,
    limit: SpeedLimit,
}
impl Report {
    fn new(
        plate: PlateNumber,
        timestamp: Timestamp,
        road: RoadId,
        mile_marker: MileMarker,
        limit: SpeedLimit,
    ) -> Self {
        Self {
            plate,
            timestamp,
            road,
            mile_marker,
            limit,
        }
    }

    fn calculate_speed(&self, previous: &Self) -> Option<SpeedMph> {
        if self.road == previous.road {
            let distance_in_miles: f32 = (max(self.mile_marker, previous.mile_marker)
                - min(self.mile_marker, previous.mile_marker))
                as f32;
            let seconds_taken: f32 = (max(self.timestamp, previous.timestamp)
                - min(self.timestamp, previous.timestamp))
                as f32;
            let speed_in_mph: f32 = distance_in_miles / (seconds_taken / 3600.0);
            Some(speed_in_mph)
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct Ticket {
    plate: PlateNumber,
    road: RoadId,
    report1: Report,
    report2: Report,
    speed: RecordedSpeed,
}
impl Ticket {
    fn from_reports(current: Report, previous: Report, speed: SpeedMph) -> Self {
        let speed: RecordedSpeed = (speed as RecordedSpeed) * 100;
        let plate = current.plate.to_owned();
        let road = current.road.to_owned();
        Self {
            plate,
            road,
            report1: min(previous.clone(), current.clone()),
            report2: max(previous, current),
            speed,
        }
    }

    fn get_days_applicable_to(&self) -> Vec<u32> {
        let day1 = self.report1.timestamp / DAY_IN_SECONDS;
        let day2 = self.report2.timestamp / DAY_IN_SECONDS;
        if day1 == day2 {
            vec![day1]
        } else {
            vec![day1, day2]
        }
    }
}

#[derive(Debug)]
enum ClientInput {
    Plate(PlateNumber, Timestamp),
    WantHeartbeat(HeartbeatInterval),
    IAmCamera(RoadId, MileMarker, SpeedLimit),
    IAmDispatcher(Vec<RoadId>),
    StreamEnded,
    StreamErrored,
}
struct Message {
    from: Uuid,
    input: ClientInput,
}

struct Connection {
    id: Uuid,
    stream: TcpStream,
    client: Option<Client>,
    heartbeat: Option<u32>,
}
impl Connection {
    fn new(stream: TcpStream) -> Self {
        Self {
            id: Uuid::new_v4(),
            stream,
            client: None,
            heartbeat: None,
        }
    }
}

struct Application {
    connections: HashMap<Uuid, Connection>,
    pending_tickets: HashMap<RoadId, Vec<Ticket>>,
    reports: HashMap<PlateNumber, Report>,
    days_issued: IssuedTickets,
}
impl Application {
    fn new() -> Self {
        Self {
            connections: HashMap::new(),
            pending_tickets: HashMap::new(),
            reports: HashMap::new(),
            days_issued: HashMap::new(),
        }
    }

    fn run(&mut self, listener: TcpListener) {
        let (conn_tx, conn_rx) = mpsc::channel::<Message>();
        loop {
            // Accept connection.
            if let Ok((stream, addr)) = listener.accept() {
                let connection = Connection::new(stream);

                println!("Accepting new connection {} from {addr}...", connection.id);

                let thread_id: Uuid = connection.id;
                let thread_transmitter = conn_tx.clone();
                let thread_stream = match connection.stream.try_clone() {
                    Ok(stream) => stream,
                    Err(_) => {
                        _ = connection.stream.shutdown(Shutdown::Both);
                        continue;
                    }
                };

                self.connections.insert(connection.id, connection);
                thread::spawn(move || handle_stream(thread_id, thread_stream, thread_transmitter));
            }

            if let Ok(message) = conn_rx.try_recv() {
                self.handle_message(message);
            }

            thread::sleep(THREAD_SLOW_DOWN);
        }
    }

    fn handle_message(&mut self, message: Message) {
        if let Some(connection) = self.connections.get_mut(&message.from) {
            println!("{}: {:?}", connection.id, message.input);
            match message.input {
                ClientInput::Plate(plate_number, timestamp) => match &connection.client {
                    Some(Client::Camera(camera)) => {
                        let report: Report = Report::new(
                            plate_number,
                            timestamp,
                            camera.road,
                            camera.mile_marker,
                            camera.limit,
                        );
                        if let Some(ticket) = self.process_report(report) {
                            let mut can_issue: bool = true;

                            let already_issued_days = self
                                .days_issued
                                .entry(ticket.plate.clone())
                                .or_insert(vec![]);
                            for applicable_day in ticket.get_days_applicable_to() {
                                if already_issued_days.contains(&applicable_day) {
                                    can_issue = false;
                                }
                                already_issued_days.push(applicable_day);
                            }

                            if can_issue {
                                self.issue_ticket(ticket);
                            }
                        }
                    }
                    Some(_) => self.close_connection(&message.from, Some(ServerError::NotACamera)),
                    None => self.close_connection(&message.from, Some(ServerError::NotDeclared)),
                },

                ClientInput::WantHeartbeat(deciseconds) => {
                    if connection.heartbeat.is_some() {
                        return self
                            .close_connection(&message.from, Some(ServerError::AlreadyBeating));
                    }

                    connection.heartbeat = Some(deciseconds);
                    if deciseconds > 0 {
                        let heartbeat_stream = match connection.stream.try_clone() {
                            Ok(stream) => stream,
                            Err(_) => {
                                self.close_connection(&message.from, Some(ServerError::Unknown));
                                return;
                            }
                        };
                        let interval = Duration::from_millis((deciseconds as u64) * 100);
                        println!(
                            "Pinging {} every {interval:?}.",
                            heartbeat_stream.peer_addr().unwrap()
                        );
                        thread::spawn(move || {
                            handle_heartbeat_interval(heartbeat_stream, interval)
                        });
                    }
                }

                ClientInput::IAmCamera(road, mile_marker, limit) => {
                    if connection.client.is_some() {
                        return self
                            .close_connection(&message.from, Some(ServerError::AlreadyDeclared));
                    }
                    connection.client = Some(Client::Camera(Camera {
                        road,
                        mile_marker,
                        limit,
                    }));
                }

                ClientInput::IAmDispatcher(roads) => {
                    if connection.client.is_some() {
                        return self
                            .close_connection(&message.from, Some(ServerError::AlreadyDeclared));
                    }
                    let dispatcher = Dispatcher { roads };
                    for road in &dispatcher.roads {
                        if let Some(tickets) = self.pending_tickets.get_mut(road) {
                            while let Some(ticket) = tickets.pop() {
                                ServerOutput::Ticket(ticket).write(&mut connection.stream);
                            }
                        }
                    }
                    connection.client = Some(Client::Dispatcher(dispatcher));
                }

                ClientInput::StreamErrored => {
                    self.close_connection(&message.from, Some(ServerError::InvalidStream))
                }
                ClientInput::StreamEnded => self.close_connection(&message.from, None),
            }
        }
    }

    fn process_report(&mut self, report: Report) -> Option<Ticket> {
        let previous: Option<Report> = self.reports.remove(&report.plate);
        self.reports.insert(report.plate.clone(), report.clone());

        if let Some(previous) = previous {
            if let Some(speed) = report.calculate_speed(&previous) {
                if speed > ((report.limit as SpeedMph) + SPEED_ERROR_MARGIN) {
                    let ticket = Ticket::from_reports(report, previous, speed);
                    return Some(ticket);
                }
            }
        }
        None
    }

    fn issue_ticket(&mut self, ticket: Ticket) {
        let dispatcher_id_for_road: Option<Uuid> = self
            .connections
            .iter()
            .filter(|(_id, connection)| -> bool {
                if let Some(Client::Dispatcher(dispatcher)) = &connection.client {
                    return dispatcher.roads.contains(&ticket.road);
                }
                false
            })
            .map(|(id, _)| id.to_owned())
            .next();

        if let Some(dispatcher_id) = dispatcher_id_for_road {
            if let Some(connection) = self.connections.get_mut(&dispatcher_id) {
                ServerOutput::Ticket(ticket.clone()).write(&mut connection.stream);
            }
        }

        if let Some(pending_tickets) = self.pending_tickets.get_mut(&ticket.road) {
            pending_tickets.push(ticket);
        } else {
            self.pending_tickets.insert(ticket.road, vec![ticket]);
        }
    }

    fn close_connection(&mut self, id: &Uuid, error: Option<ServerError>) {
        if let Some(connection) = self.connections.get_mut(id) {
            if let Some(error) = error {
                println!("ERROR ({}): {error:?}", connection.id);
                ServerOutput::Error(error).write(&mut connection.stream);
            } else {
                println!("Dropping connection {}...", connection.id);
            }
            _ = connection.stream.shutdown(Shutdown::Both);
            self.connections.remove(id);
        }
    }

    fn parse_input_buffer(buffer: &mut Vec<u8>) -> Result<Option<ClientInput>, ()> {
        let (result, drain) = match buffer.first() {
            Some(&MESSAGE_TYPE_PLATE) => Self::process_buffer_plate(buffer),
            Some(&MESSAGE_TYPE_WANT_HEARTBEAT) => Self::process_buffer_heartbeat(buffer),
            Some(&MESSAGE_TYPE_AM_CAMERA) => Self::process_buffer_camera(buffer),
            Some(&MESSAGE_TYPE_AM_DISPATCHER) => Self::process_buffer_dispatcher(buffer),
            Some(_) => (Err(()), 0),
            None => (Ok(None), 0),
        };

        if let Ok(Some(_)) = result {
            buffer.drain(..drain);
        }

        result
    }

    fn process_buffer_plate(buffer: &[u8]) -> BufferMatch {
        if let Some(plate_length) = buffer.get(1) {
            let plate_length = plate_length.to_owned() as usize;
            let drain = 1 + 1 + plate_length + 4;
            if buffer.len() >= drain {
                let message = &buffer[0..drain];
                let plate: PlateNumber = message[2..2 + plate_length].to_owned();
                if let Ok(timestamp) =
                    to_u32(&message[1 + 1 + plate_length..1 + 1 + plate_length + 4])
                {
                    (Ok(Some(ClientInput::Plate(plate, timestamp))), drain)
                } else {
                    (Err(()), 0)
                }
            } else {
                (Ok(None), 0)
            }
        } else {
            (Ok(None), 0)
        }
    }

    fn process_buffer_heartbeat(buffer: &[u8]) -> BufferMatch {
        let drain = 5;
        if buffer.len() >= drain {
            let message = &buffer[..drain];
            if let Ok(deciseconds) = to_u32(&message[1..5]) {
                (Ok(Some(ClientInput::WantHeartbeat(deciseconds))), drain)
            } else {
                (Err(()), 0)
            }
        } else {
            (Ok(None), 0)
        }
    }

    fn process_buffer_camera(buffer: &[u8]) -> BufferMatch {
        let drain: usize = 7;
        if buffer.len() >= drain {
            let message = &buffer[..drain];
            if let Ok(road) = to_u16(&message[1..3]) {
                if let Ok(mile_marker) = to_u16(&message[3..5]) {
                    if let Ok(limit) = to_u16(&message[5..7]) {
                        return (
                            Ok(Some(ClientInput::IAmCamera(road, mile_marker, limit))),
                            drain,
                        );
                    }
                }
            }
            (Err(()), 0)
        } else {
            (Ok(None), 0)
        }
    }

    fn process_buffer_dispatcher(buffer: &[u8]) -> BufferMatch {
        if let Some(road_count) = buffer.get(1) {
            let road_count = road_count.to_owned() as usize;
            let drain = 1 + 1 + (road_count * 2);
            if buffer.len() >= 1 + 1 + (road_count * 2) {
                let message = &buffer[..drain];
                let mut roads: Vec<RoadId> = Vec::new();
                for i in 0..road_count {
                    let position = 1 + 1 + (2 * i);
                    if let Ok(road) = to_u16(&message[position..position + 2]) {
                        roads.push(road);
                    } else {
                        return (Err(()), 0);
                    }
                }
                (Ok(Some(ClientInput::IAmDispatcher(roads))), drain)
            } else {
                (Ok(None), 0)
            }
        } else {
            (Ok(None), 0)
        }
    }
}

fn main() {
    let listener = get_tcp_listener(None);
    let mut app = Application::new();
    app.run(listener);
}

fn handle_stream(id: Uuid, mut stream: TcpStream, transmitter: Sender<Message>) {
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
                eprintln!("{id}:{}", u8s_to_hex_str(&buffer[..n]));
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

fn handle_heartbeat_interval(mut stream: TcpStream, interval: std::time::Duration) {
    'heartbeat: loop {
        thread::sleep(interval);
        if !ServerOutput::Heartbeat.write(&mut stream) {
            _ = stream.shutdown(Shutdown::Both);
            break 'heartbeat;
        }
    }
}

#[derive(Debug)]
enum ServerError {
    Unknown,
    AlreadyDeclared,
    NotDeclared,
    AlreadyBeating,
    NotACamera,
    InvalidStream,
}
enum ServerOutput {
    Error(ServerError),
    Ticket(Ticket),
    Heartbeat,
}
impl ServerOutput {
    fn write(&self, stream: &mut TcpStream) -> bool {
        let mut response: Vec<u8> = Vec::new();
        match self {
            Self::Error(error) => {
                let error_string = match error {
                    ServerError::Unknown => "Unknown Error",
                    ServerError::AlreadyDeclared => "Type Already Declared",
                    ServerError::NotDeclared => "Type Not Declared",
                    ServerError::AlreadyBeating => "Heartbeat Already Requested",
                    ServerError::NotACamera => "Not A Camera",
                    ServerError::InvalidStream => "Invalid Stream",
                }
                .to_string();
                response.push(MESSAGE_TYPE_ERROR);
                response.push(error_string.len() as u8);
                response.extend_from_slice(error_string.as_bytes());
            }
            Self::Ticket(ticket) => {
                response.push(MESSAGE_TYPE_TICKET);
                response.push(ticket.plate.len() as u8);
                response.extend(&ticket.plate);
                response.extend_from_slice(&ticket.road.to_be_bytes());
                response.extend_from_slice(&ticket.report1.mile_marker.to_be_bytes());
                response.extend_from_slice(&ticket.report1.timestamp.to_be_bytes());
                response.extend_from_slice(&ticket.report2.mile_marker.to_be_bytes());
                response.extend_from_slice(&ticket.report2.timestamp.to_be_bytes());
                response.extend_from_slice(&ticket.speed.to_be_bytes());
            }
            Self::Heartbeat => response.push(MESSAGE_TYPE_HEARTBEAT),
        };
        eprintln!(">>>{}", u8s_to_hex_str(&response));
        stream.write_all(&response).is_ok()
    }
}

fn to_u32(bytes: &[u8]) -> Result<u32, ()> {
    let bytes: [u8; 4] = match bytes.try_into() {
        Ok(bytes) => bytes,
        Err(_) => return Err(()),
    };
    Ok(u32::from_be_bytes(bytes))
}

fn to_u16(bytes: &[u8]) -> Result<u16, ()> {
    let bytes: [u8; 2] = match bytes.try_into() {
        Ok(bytes) => bytes,
        Err(_) => return Err(()),
    };
    Ok(u16::from_be_bytes(bytes))
}

fn u8s_to_hex_str(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!(" {byte:02x}")).collect()
}
