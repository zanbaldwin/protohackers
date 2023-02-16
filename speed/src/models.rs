use crate::{PlateNumber, SpeedMph, DAY_IN_SECONDS};
use std::cmp::{max, min};
use std::net::TcpStream;

use uuid::Uuid;

pub(crate) struct Camera {
    pub(crate) road: u16,
    pub(crate) mile_marker: u16,
    pub(crate) limit: u16,
}
pub(crate) struct Dispatcher {
    pub(crate) roads: Vec<u16>,
}
pub(crate) enum Client {
    Camera(Camera),
    Dispatcher(Dispatcher),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Report {
    pub(crate) plate: PlateNumber,
    pub(crate) road: u16,
    pub(crate) timestamp: u32,
    pub(crate) mile_marker: u16,
    pub(crate) limit: u16,
}
impl Report {
    pub(crate) fn new(
        plate: PlateNumber,
        timestamp: u32,
        road: u16,
        mile_marker: u16,
        limit: u16,
    ) -> Self {
        Self {
            plate,
            timestamp,
            road,
            mile_marker,
            limit,
        }
    }

    pub(crate) fn calculate_speed(&self, previous: &Self) -> Option<SpeedMph> {
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
pub(crate) struct Ticket {
    pub(crate) plate: PlateNumber,
    pub(crate) road: u16,
    pub(crate) report1: Report,
    pub(crate) report2: Report,
    pub(crate) speed: u16,
}
impl Ticket {
    pub(crate) fn from_reports(current: Report, previous: Report, speed: SpeedMph) -> Self {
        let speed: u16 = (speed as u16) * 100;
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

    pub(crate) fn get_days_applicable_to(&self) -> Vec<u32> {
        let day1 = self.report1.timestamp / DAY_IN_SECONDS;
        let day2 = self.report2.timestamp / DAY_IN_SECONDS;
        if day1 == day2 {
            vec![day1]
        } else {
            vec![day1, day2]
        }
    }
}

pub(crate) struct Connection {
    pub(crate) id: Uuid,
    pub(crate) stream: TcpStream,
    pub(crate) client: Option<Client>,
    pub(crate) heartbeat: Option<u32>,
}
impl Connection {
    pub(crate) fn new(stream: TcpStream) -> Self {
        Self {
            id: Uuid::new_v4(),
            stream,
            client: None,
            heartbeat: None,
        }
    }
}
