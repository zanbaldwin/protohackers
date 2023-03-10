/**
 * Integration tests for Speed Daemon.
 * Unit test belong at the bottom of source files.
 *
 * Lesson I learnt getting this to work: integration tests (a `tests/` folder) will only work for LIBRARIES not BINARIES.
 * So change `main.rs` to `lib.rs` and then create a `bin/main.rs`, which has to IMPORT (extern crate) the library that was just created.
 */

#[cfg(test)]
mod test {
    use speed::Application;
    use std::thread;
    use std::time::Duration;
    use testing::{
        assert_client_receives_bytes, connect, listen_on_available_port, send_bytes_from,
    };

    const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);

    fn setup() -> u16 {
        let (listener, port) = listen_on_available_port();
        thread::spawn(move || Application::new().run(listener));
        port
    }

    #[test]
    #[ignore]
    fn heartbeat() {
        let port = setup();
        let mut client = connect(port);

        send_bytes_from!(client, "40 00 00 00 0a");
        assert_client_receives_bytes!(client, "41", Duration::from_millis(1100));
    }

    #[test]
    fn car() {
        let port = setup();
        let mut camera_one = connect(port);
        let mut camera_two = connect(port);
        let mut dispatcher = connect(port);

        // send_bytes_from!(camera_one, "80 03 11 0c 9d 00 64");
        {
            use ::std::io::Write;
            camera_one
                .write_all(
                    &testing::hex_str_to_u8s("80 03 11 0c 9d 00 64")
                        .expect("Invalid hex code provided for integration test."),
                )
                .expect("Could not write bytes to application.");
        }

        send_bytes_from!(camera_two, "80 03 11 0c a7 00 64");
        send_bytes_from!(dispatcher, "81 01");
        send_bytes_from!(camera_one, "20 07 56 48 30 30 4a 52 57 00 0a 61 0d");
        send_bytes_from!(camera_two, "20 07 56 48 30 30 4a 52 57 00 0a 62 39");
        send_bytes_from!(dispatcher, "03 11");

        // assert_client_receives_bytes!(
        //     dispatcher,
        //     "21 07 56 48 30 30 4a 52 57 03 11 0c 9d 00 0a 61 0d 0c a7 00 0a 62 39 2e e0",
        //     DEFAULT_TIMEOUT
        // );
        {
            use std::io::Read;
            let hex = "21 07 56 48 30 30 4a 52 57 03 11 0c 9d 00 0a 61 0d 0c a7 00 0a 62 39 2e e0";
            let client = &mut dispatcher;
            client
                .set_read_timeout(Some(DEFAULT_TIMEOUT))
                .expect("Could not set read timeout.");

            let bytes = testing::hex_str_to_u8s(hex)
                .expect("Invalid hex code provided for integration test.");
            let mut reader = client.take(bytes.len() as u64);
            let mut payload: Vec<u8> = ::std::vec::Vec::with_capacity(bytes.len());
            let mut buffer = [0u8; 512];

            let now = ::std::time::Instant::now();
            loop {
                match reader.read(&mut buffer) {
                    Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => (),
                    Err(e) => panic!("Client connection errored: {e:?}"),
                    Ok(0) => panic!("Client connection dropped."),
                    Ok(n) => {
                        payload.extend_from_slice(&buffer[..n]);
                        if payload.len() >= bytes.len() {
                            break;
                        }
                    }
                };
                if now.elapsed() > DEFAULT_TIMEOUT {
                    panic!("Timeout reached waiting for expected payload.");
                }
                // Don't hog the CPU core.
                ::std::thread::sleep(::std::time::Duration::from_millis(1));
            }
            assert_eq!(&bytes, &payload[..bytes.len()]);
            client
                .set_read_timeout(None)
                .expect("Could not unset read timeout.");
        }
    }

    #[test]
    #[ignore]
    fn multiple_tickets() {
        let port = setup();

        let mut broken_camera = connect(port);
        send_bytes_from!(broken_camera, "80 00 00");

        let mut first_camera = connect(port);
        send_bytes_from!(first_camera, "80 1a 47 0d 18 00 50");
        let mut second_camera = connect(port);
        send_bytes_from!(second_camera, "80 1a 47 0d 23 00 50");
        let mut third_camera = connect(port);
        send_bytes_from!(third_camera, "80 1a 47 0d 2e 00 50");

        let mut dispatcher = connect(port);
        send_bytes_from!(dispatcher, "81 01");

        send_bytes_from!(second_camera, "20 07 52 56 36 30 55 58 50 02 16 d0 8f");
        send_bytes_from!(dispatcher, "1a 47");
        send_bytes_from!(first_camera, "20 07 52 56 36 30 55 58 50 02 16 cf 61");
        send_bytes_from!(third_camera, "20 07 52 56 36 30 55 58 50 02 16 d1 a9");

        assert_client_receives_bytes!(
            dispatcher,
            "21 07 52 56 36 30 55 58 50 1a 47 0d 18 02 16 cf 61 0d 23 02 16 d0 8f 33 2c",
            DEFAULT_TIMEOUT
        );
    }

    #[test]
    #[ignore]
    fn multiple_cars() {
        let port = setup();
        let mut first_camera = connect(port);
        let mut second_camera = connect(port);
        let mut dispatcher = connect(port);

        send_bytes_from!(first_camera, "80 a7 22 00 0a 00 3c");
        send_bytes_from!(second_camera, "80 a7 22 04 ca 00 3c");
        send_bytes_from!(second_camera, "20 07 4e 5a 37 38 51 59 55 00 f7 88 c4");
        send_bytes_from!(first_camera, "20 07 50 50 34 37 41 44 4c 00 f7 88 11");
        send_bytes_from!(dispatcher, "81 01");
        send_bytes_from!(dispatcher, "a7 22");
        send_bytes_from!(first_camera, "20 07 4e 5a 37 38 51 59 55 00 f8 b8 8d 20 07 4e 58 32 31 4a 51 53 00 f7 87 ad 20 07 59 4e 31 31 50 52 43 00 f7 89 5f 20 07 47 55 30 38 51 45 54 00 f7 88 36");
        send_bytes_from!(second_camera, "20 07 47 55 30 38 51 45 54 00 f8 74 5e 20 07 4e 58 32 31 4a 51 53 00 f8 32 ad 20 07 50 50 34 37 41 44 4c 00 f8 62 bc");

        assert_client_receives_bytes!(
            dispatcher,
            "21 07 47 55 30 38 51 45 54 a7 22 00 0a 00 f7 88 36 04 ca 00 f8 74 5e 1c 20",
            DEFAULT_TIMEOUT
        );
        assert_client_receives_bytes!(
            dispatcher,
            "21 07 4e 58 32 31 4a 51 53 a7 22 00 0a 00 f7 87 ad 04 ca 00 f8 32 ad 27 10",
            DEFAULT_TIMEOUT
        );
        assert_client_receives_bytes!(
            dispatcher,
            "21 07 50 50 34 37 41 44 4c a7 22 00 0a 00 f7 88 11 04 ca 00 f8 62 bc 1e 78",
            DEFAULT_TIMEOUT
        );
    }
}
