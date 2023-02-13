extern crate speed;

use common::get_tcp_listener;
use speed::Application;

fn main() {
    let listener = get_tcp_listener(None);
    Application::new().run(listener);
}
