/**
 * Integration tests for Speed Daemon.
 * Unit test belong at the bottom of source files.
 */

#[cfg(test)]
mod test {
    use common::get_tcp_listener;
    use speed::app::Application;
    use std::thread;
    use testing::{
        assert_client_not_receives_bytes, assert_client_receives_bytes, connect,
        find_available_port, hex_str_to_u8s, send_bytes_from,
    };

    fn setup() -> u16 {
        let Some(port) = find_available_port() else {
            panic!("Could not find an available port to run integration tests.");
        };
        let listener = get_tcp_listener(Some(port));
        thread::spawn(move || Application::new(listener).run());
        port
    }

    #[test]
    fn no_heartbeat() {
        let port = setup();

        let mut client = connect(port);

        send_bytes_from!(client, "40 00 00 00 00");
        assert_client_not_receives_bytes!(
            client,
            "41",
            /* timeout: */ Duration::from_millis(500)
        );
    }

    #[test]
    fn some_heartbeat() {
        let port = setup();
        let mut client = connect(port);

        send_bytes_from!(client, "40 00 00 00 0a");
        assert_client_receives_bytes!(
            client,
            "41",
            /* timeout: */ Duration::from_millis(1100)
        );
    }

    #[test]
    fn car() {
        let port = setup();
        let mut camera_one = connect(port);
        let mut camera_two = connect(port);
        let mut dispatcher = connect(port);

        send_bytes_from!(camera_one, "80 03 11 0c 9d 00 64");
        send_bytes_from!(camera_two, "80 03 11 0c a7 00 64");
        send_bytes_from!(dispatcher, "80 01");
        send_bytes_from!(camera_one, "20 07 56 48 30 30 4a 52 57 00 0a 61 0d");
        send_bytes_from!(camera_two, "20 07 56 48 30 30 4a 52 57 00 0a 62 39");
        send_bytes_from!(dispatcher, "03 11");

        assert_client_receives_bytes!(
            dispatcher,
            "21 07 56 48 30 30 4a 52 57 03 11 0c 9d 00 0a 61 0d 0c a7 00 0a 62 39 2e e0"
        );
    }

    #[test]
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
        send_bytes_from!(dispatcher, "80 01");

        send_bytes_from!(second_camera, "20 07 52 56 36 30 55 58 50 02 16 d0 8f");
        send_bytes_from!(dispatcher, "1a 47");
        send_bytes_from!(first_camera, "20 07 52 56 36 30 55 58 50 02 16 cf 61");
        send_bytes_from!(third_camera, "20 07 52 56 36 30 55 58 50 02 16 d1 a9");

        assert_client_receives_bytes!(
            dispatcher,
            "21 07 52 56 36 30 55 58 50 1a 47 0d 18 02 16 cf 61 0d 23 02 16 d0 8f 33 2c"
        );
    }

    #[test]
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
            "21 07 47 55 30 38 51 45 54 a7 22 00 0a 00 f7 88 36 04 ca 00 f8 74 5e 1c 20"
        );
        assert_client_receives_bytes!(
            dispatcher,
            "21 07 4e 58 32 31 4a 51 53 a7 22 00 0a 00 f7 87 ad 04 ca 00 f8 32 ad 27 10"
        );
        assert_client_receives_bytes!(
            dispatcher,
            "21 07 50 50 34 37 41 44 4c a7 22 00 0a 00 f7 88 11 04 ca 00 f8 62 bc 1e 78"
        );
    }
}
