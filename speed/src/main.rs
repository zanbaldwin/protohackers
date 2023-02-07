extern crate uuid;
pub mod app;
mod handles;
mod io;
mod types;
mod utils;

use crate::app::Application;
use common::get_tcp_listener;
use std::cmp::{max, min};
use std::net::TcpStream;
use uuid::Uuid;

const SPEED_ERROR_MARGIN: f32 = 0.4;
const DAY_IN_SECONDS: u32 = 86_400;

struct Camera {
    road: types::RoadId,
    mile_marker: types::MileMarker,
    limit: types::SpeedLimit,
}
struct Dispatcher {
    roads: Vec<types::RoadId>,
}
enum Client {
    Camera(Camera),
    Dispatcher(Dispatcher),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Report {
    plate: types::PlateNumber,
    road: types::RoadId,
    timestamp: types::Timestamp,
    mile_marker: types::MileMarker,
    limit: types::SpeedLimit,
}
impl Report {
    fn new(
        plate: types::PlateNumber,
        timestamp: types::Timestamp,
        road: types::RoadId,
        mile_marker: types::MileMarker,
        limit: types::SpeedLimit,
    ) -> Self {
        Self {
            plate,
            timestamp,
            road,
            mile_marker,
            limit,
        }
    }

    fn calculate_speed(&self, previous: &Self) -> Option<types::SpeedMph> {
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
    plate: types::PlateNumber,
    road: types::RoadId,
    report1: Report,
    report2: Report,
    speed: types::RecordedSpeed,
}
impl Ticket {
    fn from_reports(current: Report, previous: Report, speed: types::SpeedMph) -> Self {
        let speed: types::RecordedSpeed = (speed as types::RecordedSpeed) * 100;
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

fn main() {
    let listener = get_tcp_listener(None);
    Application::new().run(listener);
}
