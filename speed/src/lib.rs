mod handles;
mod io;
mod models;
mod parser;
mod utils;

use crate::{
    io::{ClientInput, Message, ServerError, ServerOutput},
    models::{Camera, Client, Connection, Dispatcher, Report, Ticket},
};
use common::THREAD_SLOW_DOWN;
use std::collections::HashMap;
use std::net::{Shutdown, TcpListener};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

const SPEED_ERROR_MARGIN: f32 = 0.4;
const DAY_IN_SECONDS: u32 = 86_400;

pub(crate) const MESSAGE_TYPE_ERROR: u8 = 0x10;
pub(crate) const MESSAGE_TYPE_PLATE: u8 = 0x20;
pub(crate) const MESSAGE_TYPE_TICKET: u8 = 0x21;
pub(crate) const MESSAGE_TYPE_WANT_HEARTBEAT: u8 = 0x40;
pub(crate) const MESSAGE_TYPE_HEARTBEAT: u8 = 0x41;
pub(crate) const MESSAGE_TYPE_AM_CAMERA: u8 = 0x80;
pub(crate) const MESSAGE_TYPE_AM_DISPATCHER: u8 = 0x81;

pub(crate) type SpeedMph = f32;
pub(crate) type PlateNumber = Vec<u8>;
pub(crate) type BufferMatch = Result<Option<(ClientInput, usize)>, ()>;
pub(crate) type IssuedTickets = HashMap<Vec<u8>, Vec<u32>>;

//////////////////////////////////

#[derive(Default)]
pub struct Application {
    connections: HashMap<Uuid, Connection>,
    pending_tickets: HashMap<u16, Vec<Ticket>>,
    reports: HashMap<PlateNumber, Report>,
    days_issued: IssuedTickets,
}
impl Application {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(mut self, listener: TcpListener) -> ! {
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
}
