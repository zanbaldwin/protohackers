use crate::PlateNumber;

pub(crate) mod io;

enum Client {
    Camera(Camera),
    Dispatcher(Dispatcher),
}

struct Camera {
    road: u16,
    marker: usize,
    limit: usize,
}
struct Dispatcher {
    roads: Vec<u16>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Report {
    pub(crate) plate: PlateNumber,
    pub(crate) road: u16,
    pub(crate) timestamp: u32,
    pub(crate) mile: u16,
    pub(crate) limit: u16,
}

pub(crate) struct Ticket {
    pub(crate) plate: PlateNumber,
    pub(crate) road: u16,
    pub(crate) report1: Report,
    pub(crate) report2: Report,
    pub(crate) speed: u16,
}
