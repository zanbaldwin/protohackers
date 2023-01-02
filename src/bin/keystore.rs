use protozackers::{server, BUFFER_SIZE};
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};

fn main() {
    server::run(handle_stream, None);
}

struct AssetPrice {
    timestamp: i32,
    price: i32,
}
impl AssetPrice {
    fn new(timestamp: i32, price: i32) -> Self {
        Self { timestamp, price }
    }
}

pub fn handle_stream(mut stream: TcpStream) {
    let mut store: Vec<AssetPrice> = vec![];
    let mut queue: Vec<u8> = vec![];
    let mut input: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let session_addr = stream.peer_addr().unwrap();

    'connected: loop {
        while queue.len() >= 9 {
            let message = queue.drain(0..9);
            let message_bytes = message.as_slice();
            //let message: &[u8] = queue.drain(0..9).as_slice();
            match &message_bytes[0] {
                73 | 105 => handle_insert(message_bytes, &mut store, &session_addr),
                81 | 113 => {
                    let result: i32 = handle_query(message_bytes, &store, &session_addr);
                    let response: &[u8] = &result.to_be_bytes();
                    eprintln!("Response: {result} ({response:?}");
                    stream.write_all(response).unwrap();
                }
                _ => break 'connected,
            }
        }

        match stream.read(&mut input) {
            Ok(0) => break,
            Ok(length) => queue.extend_from_slice(&input[..length]),
            Err(err) => panic!("{err:?}"),
        };
    }
    stream.shutdown(Shutdown::Both).unwrap_or_default();
}

fn handle_insert(message: &[u8], store: &mut Vec<AssetPrice>, session: &SocketAddr) {
    let timestamp: i32 = buf_to_i32(&message[1..5]);
    let price: i32 = buf_to_i32(&message[5..9]);
    let asset: AssetPrice = AssetPrice::new(timestamp, price);
    eprintln!(
        "Received Insert: session={session}, timestamp={timestamp} ({:?}), price={price} ({:?})",
        &message[1..5],
        &message[5..9],
    );
    store.push(asset);
}

fn handle_query(message: &[u8], store: &[AssetPrice], session: &SocketAddr) -> i32 {
    let min = buf_to_i32(&message[1..5]);
    let max = buf_to_i32(&message[5..9]);
    eprintln!(
        "Received Query: session={session}, min={min} ({:?}), max={max} ({:?})",
        &message[1..5],
        &message[5..9],
    );
    let prices_within_daterange = store
        .iter()
        .filter(|asset: &&AssetPrice| -> bool { asset.timestamp >= min && asset.timestamp <= max })
        .map(|asset: &AssetPrice| -> i32 { asset.price.to_owned() });
    let mut count: usize = 0;
    let mut total: i128 = 0;
    for price in prices_within_daterange {
        // Too lazy to figure out how to satisfy the trait for .sum();
        count += 1;
        total += price as i128;
    }
    let average = match count {
        0 => 0,
        count => total / (count as i128),
    };
    eprintln!("                total={total} count={count} average={average}");
    average as i32
}

fn buf_to_i32(input: &[u8]) -> i32 {
    let (bytes, _) = input.split_at(std::mem::size_of::<i32>());
    i32::from_be_bytes(bytes.try_into().unwrap())
}
