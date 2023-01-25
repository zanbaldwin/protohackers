use std::time::Duration;

pub mod server;

pub const ASCII_NEWLINE: u8 = 10;
pub const BUFFER_SIZE: usize = 1024;
pub const DEFAULT_PORT: u16 = 8096;
pub const THREAD_SLOW_DOWN: Duration = Duration::from_millis(100);
