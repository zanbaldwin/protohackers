use crate::error::Error;
use crate::models::Ticket;
use crate::{ByteString, MESSAGE_TYPE_ERROR, MESSAGE_TYPE_HEARTBEAT, MESSAGE_TYPE_TICKET};
use uuid::Uuid;

pub(crate) struct ClientMessage {
    pub(crate) from: Uuid,
    pub(crate) input: ClientInput,
}
#[cfg_attr(test, derive(PartialEq, Debug))]
pub(crate) enum ClientInput {
    Plate(ByteString, u32),
    WantHeartbeat(u32),
    IAmCamera(u16, u16, u16),
    IAmDispatcher(Vec<u16>),
    Error(Error),
}

pub(crate) struct ServerMessage {
    pub(crate) to: Uuid,
    pub(crate) output: ServerOutput,
}

pub(crate) enum ServerOutput {
    Error(Error),
    Ticket(Ticket),
    Heartbeat,
}
impl ServerOutput {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut output: ByteString = Vec::new();
        match self {
            Self::Heartbeat => output.push(MESSAGE_TYPE_HEARTBEAT),
            Self::Ticket(ticket) => {
                output.push(MESSAGE_TYPE_TICKET);
                output.push(ticket.plate.len() as u8);
                output.extend(&ticket.plate);
                output.extend_from_slice(&ticket.road.to_be_bytes());
                output.extend_from_slice(&ticket.report1.mile.to_be_bytes());
                output.extend_from_slice(&ticket.report1.timestamp.to_be_bytes());
                output.extend_from_slice(&ticket.report2.mile.to_be_bytes());
                output.extend_from_slice(&ticket.report2.timestamp.to_be_bytes());
                output.extend_from_slice(&ticket.speed.to_be_bytes());
            },
            Self::Error(error) => {
                let error = error.to_string().as_str().as_bytes();
                output.push(MESSAGE_TYPE_ERROR);
                output.push(error.len() as u8);
                output.extend_from_slice(error);
            },
        }
        output
    }
}
