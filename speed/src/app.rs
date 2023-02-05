use crate::{
    handles,
    io::{ClientInput, Message, ServerError, ServerOutput},
    types, utils, Camera, Client, Connection, Dispatcher, Report, Ticket, SPEED_ERROR_MARGIN,
};
use common::THREAD_SLOW_DOWN;
use std::collections::HashMap;
use std::net::{Shutdown, TcpListener};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

pub(crate) struct Application {
    connections: HashMap<Uuid, Connection>,
    pending_tickets: HashMap<types::RoadId, Vec<Ticket>>,
    reports: HashMap<types::PlateNumber, Report>,
    days_issued: types::IssuedTickets,
}
impl Application {
    pub(crate) fn new() -> Self {
        Self {
            connections: HashMap::new(),
            pending_tickets: HashMap::new(),
            reports: HashMap::new(),
            days_issued: HashMap::new(),
        }
    }

    pub(crate) fn run(mut self, listener: TcpListener) -> ! {
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
                thread::spawn(move || {
                    handles::connection(thread_id, thread_stream, thread_transmitter)
                });
            }

            if let Ok(message) = conn_rx.try_recv() {
                self.handle_message(message);
            }

            thread::sleep(THREAD_SLOW_DOWN);
        }
    }

    pub(crate) fn handle_message(&mut self, message: Message) {
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
                        thread::spawn(move || handles::heartbeat(heartbeat_stream, interval));
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

    pub(crate) fn process_report(&mut self, report: Report) -> Option<Ticket> {
        let previous: Option<Report> = self.reports.remove(&report.plate);
        self.reports.insert(report.plate.clone(), report.clone());

        if let Some(previous) = previous {
            if let Some(speed) = report.calculate_speed(&previous) {
                if speed > ((report.limit as types::SpeedMph) + SPEED_ERROR_MARGIN) {
                    let ticket = Ticket::from_reports(report, previous, speed);
                    return Some(ticket);
                }
            }
        }
        None
    }

    pub(crate) fn issue_ticket(&mut self, ticket: Ticket) {
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

    pub(crate) fn close_connection(&mut self, id: &Uuid, error: Option<ServerError>) {
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

    pub(crate) fn parse_input_buffer(buffer: &mut Vec<u8>) -> Result<Option<ClientInput>, ()> {
        let (result, drain) = match buffer.first() {
            Some(&types::MESSAGE_TYPE_PLATE) => Self::process_buffer_plate(buffer),
            Some(&types::MESSAGE_TYPE_WANT_HEARTBEAT) => Self::process_buffer_heartbeat(buffer),
            Some(&types::MESSAGE_TYPE_AM_CAMERA) => Self::process_buffer_camera(buffer),
            Some(&types::MESSAGE_TYPE_AM_DISPATCHER) => Self::process_buffer_dispatcher(buffer),
            Some(_) => (Err(()), 0),
            None => (Ok(None), 0),
        };

        if let Ok(Some(_)) = result {
            buffer.drain(..drain);
        }

        result
    }

    pub(crate) fn process_buffer_plate(buffer: &[u8]) -> types::BufferMatch {
        if let Some(plate_length) = buffer.get(1) {
            let plate_length = plate_length.to_owned() as usize;
            let drain = 1 + 1 + plate_length + 4;
            if buffer.len() >= drain {
                let message = &buffer[0..drain];
                let plate: types::PlateNumber = message[2..2 + plate_length].to_owned();
                if let Ok(timestamp) =
                    utils::to_u32(&message[1 + 1 + plate_length..1 + 1 + plate_length + 4])
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

    pub(crate) fn process_buffer_heartbeat(buffer: &[u8]) -> types::BufferMatch {
        let drain = 5;
        if buffer.len() >= drain {
            let message = &buffer[..drain];
            if let Ok(deciseconds) = utils::to_u32(&message[1..5]) {
                (Ok(Some(ClientInput::WantHeartbeat(deciseconds))), drain)
            } else {
                (Err(()), 0)
            }
        } else {
            (Ok(None), 0)
        }
    }

    pub(crate) fn process_buffer_camera(buffer: &[u8]) -> types::BufferMatch {
        let drain: usize = 7;
        if buffer.len() >= drain {
            let message = &buffer[..drain];
            if let Ok(road) = utils::to_u16(&message[1..3]) {
                if let Ok(mile_marker) = utils::to_u16(&message[3..5]) {
                    if let Ok(limit) = utils::to_u16(&message[5..7]) {
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

    pub(crate) fn process_buffer_dispatcher(buffer: &[u8]) -> types::BufferMatch {
        if let Some(road_count) = buffer.get(1) {
            let road_count = road_count.to_owned() as usize;
            let drain = 1 + 1 + (road_count * 2);
            if buffer.len() >= 1 + 1 + (road_count * 2) {
                let message = &buffer[..drain];
                let mut roads: Vec<types::RoadId> = Vec::new();
                for i in 0..road_count {
                    let position = 1 + 1 + (2 * i);
                    if let Ok(road) = utils::to_u16(&message[position..position + 2]) {
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
