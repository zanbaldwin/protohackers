use crate::io::ClientInput;
use std::collections::HashMap;

pub(crate) const MESSAGE_TYPE_ERROR: u8 = 0x10;
pub(crate) const MESSAGE_TYPE_PLATE: u8 = 0x20;
pub(crate) const MESSAGE_TYPE_TICKET: u8 = 0x21;
pub(crate) const MESSAGE_TYPE_WANT_HEARTBEAT: u8 = 0x40;
pub(crate) const MESSAGE_TYPE_HEARTBEAT: u8 = 0x41;
pub(crate) const MESSAGE_TYPE_AM_CAMERA: u8 = 0x80;
pub(crate) const MESSAGE_TYPE_AM_DISPATCHER: u8 = 0x81;

pub(crate) type RoadId = u16;
pub(crate) type MileMarker = u16;
pub(crate) type SpeedLimit = u16;
pub(crate) type RecordedSpeed = u16;
pub(crate) type SpeedMph = f32;
pub(crate) type Timestamp = u32;
pub(crate) type PlateNumber = Vec<u8>;
pub(crate) type HeartbeatInterval = u32;
pub(crate) type BufferMatch = (Result<Option<ClientInput>, ()>, usize);
pub(crate) type IssuedTickets = HashMap<Vec<u8>, Vec<u32>>;
