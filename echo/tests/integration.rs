//! Integration tests for Echo Server.

#[cfg(test)]
mod test {
    use common::THREAD_SLOW_DOWN;
    use std::thread;
    use std::time::Duration;
    use testing::{
        assert_client_receives_bytes, connect, listen_on_available_port, send_bytes_from,
    };

    const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);

    fn setup() -> u16 {
        let (listener, port) = listen_on_available_port();
        thread::spawn(move || loop {
            if let Ok((stream, _)) = listener.accept() {
                thread::spawn(move || echo::handle_stream_immediate(stream));
            }
            thread::sleep(THREAD_SLOW_DOWN);
        });
        port
    }

    #[test]
    fn echo_good_exact() {
        let port = setup();
        let mut client = connect(port);

        send_bytes_from!(client, "40 00 00 00 0a");
        assert_client_receives_bytes!(client, "40 00 00 00 0a", DEFAULT_TIMEOUT);
    }

    #[test]
    fn echo_good_extra() {
        let port = setup();
        let mut client = connect(port);

        send_bytes_from!(client, "40 00 00 00 0a");
        assert_client_receives_bytes!(client, "40 00", DEFAULT_TIMEOUT);
    }

    #[test]
    #[should_panic]
    fn echo_bad() {
        let port = setup();
        let mut client = connect(port);

        send_bytes_from!(client, "40 00 00 00 0a");
        assert_client_receives_bytes!(client, "40 00 12 00 0a", DEFAULT_TIMEOUT);
    }

    #[test]
    #[should_panic]
    fn echo_timeout() {
        let port = setup();
        let mut client = connect(port);

        send_bytes_from!(client, "40 00 00 00 0a");
        assert_client_receives_bytes!(client, "40 00 00 00 0a ab", DEFAULT_TIMEOUT);
    }
}
