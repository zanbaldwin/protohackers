use crate::{types, utils, Ticket};
use std::io::Write;
use std::net::TcpStream;
use uuid::Uuid;

#[derive(Debug)]
pub(crate) enum ClientInput {
    Plate(types::PlateNumber, types::Timestamp),
    WantHeartbeat(types::HeartbeatInterval),
    IAmCamera(types::RoadId, types::MileMarker, types::SpeedLimit),
    IAmDispatcher(Vec<types::RoadId>),
    StreamEnded,
    StreamErrored,
}
pub(crate) struct Message {
    pub(crate) from: Uuid,
    pub(crate) input: ClientInput,
}
pub(crate) enum ServerOutput {
    Error(ServerError),
    Ticket(Ticket),
    Heartbeat,
}
impl ServerOutput {
    pub(crate) fn write(&self, stream: &mut TcpStream) -> bool {
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
                response.push(types::MESSAGE_TYPE_ERROR);
                response.push(error_string.len() as u8);
                response.extend_from_slice(error_string.as_bytes());
            }
            Self::Ticket(ticket) => {
                response.push(types::MESSAGE_TYPE_TICKET);
                response.push(ticket.plate.len() as u8);
                response.extend(&ticket.plate);
                response.extend_from_slice(&ticket.road.to_be_bytes());
                response.extend_from_slice(&ticket.report1.mile_marker.to_be_bytes());
                response.extend_from_slice(&ticket.report1.timestamp.to_be_bytes());
                response.extend_from_slice(&ticket.report2.mile_marker.to_be_bytes());
                response.extend_from_slice(&ticket.report2.timestamp.to_be_bytes());
                response.extend_from_slice(&ticket.speed.to_be_bytes());
            }
            Self::Heartbeat => response.push(types::MESSAGE_TYPE_HEARTBEAT),
        };
        eprintln!(">>> {}", utils::u8s_to_hex_str(&response));
        stream.write_all(&response).is_ok()
    }
}
#[derive(Debug)]
pub(crate) enum ServerError {
    Unknown,
    AlreadyDeclared,
    NotDeclared,
    AlreadyBeating,
    NotACamera,
    InvalidStream,
}
