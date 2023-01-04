use protozackers::{server, BUFFER_SIZE};
use std::io::{Read, Write, ErrorKind};
use std::net::{Shutdown, TcpStream};
use std::vec::Drain;

fn main() {
    server::run(handle_stream, None, false);
}

struct AssetPrice {
    timestamp: i32,
    price: i32,
}

fn to_i32(input: &[u8]) -> i32 {
    let (bytes, _) = input.split_at(std::mem::size_of::<i32>());
    i32::from_be_bytes(bytes.try_into().unwrap())
}

pub fn handle_stream(mut stream: TcpStream) {
    let mut store: Vec<AssetPrice> = vec![];
    let mut queue: Vec<u8> = vec![];
    let mut input: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

    'connected: loop {
        match stream.read(&mut input) {
            Ok(0) => break 'connected,
            Ok(length) => queue.extend_from_slice(&input[..length]),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => (),
            Err(err) => panic!("Error processing stream: {err:?}"),
        };

        while queue.len() >= 9 {
            let message: Drain<u8> = queue.drain(0..9);
            let bytes: &[u8] = message.as_slice();
            //let message: &[u8] = queue.drain(0..9).as_slice();
            match &bytes[0] {
                73 | 105 => store.push(AssetPrice {
                    timestamp: to_i32(&bytes[1..5]),
                    price: to_i32(&bytes[5..9]),
                }),
                81 | 113 => _ = stream.write_all(&handle_query(bytes, &store).to_be_bytes()),
                _ => break 'connected,
            }
        }
    }

    stream.shutdown(Shutdown::Both).unwrap_or_default();
}

fn handle_query(message: &[u8], store: &[AssetPrice]) -> i32 {
    let min = to_i32(&message[1..5]);
    let max = to_i32(&message[5..9]);
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
    average as i32
}
