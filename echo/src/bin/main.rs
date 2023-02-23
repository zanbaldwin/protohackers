use common::run;
use echo::handle_stream_immediate;

fn main() {
    run(handle_stream_immediate, None, false);
}
