use std::net::TcpStream;

pub fn setup_connection() -> TcpStream {
    todo!();
}

pub fn i_am_camera(road: u16, mile_marker: u16, limit: u16) -> String {
    todo!();
}

pub fn i_am_dispatcher(roads: &[u16]) -> String {
    todo!();
}

pub fn plate(plate: &str, timestamp: u32) -> String {
    todo!();
}

#[macro_export]
macro_rules! send_bytes_from {
    ($s:expr, $h:expr) => {};
}

#[macro_export]
macro_rules! assert_client_receives_bytes (
    ($s:expr, $h:expr, $d:expr) => {};
    ($s:expr, $h:expr) => {};
);

#[macro_export]
macro_rules! assert_client_not_receives_bytes (
    ($s:expr, $h:expr, $d:expr) => {};
);
