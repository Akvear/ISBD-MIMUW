use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;

mod hash;
use hash::stretch_md5;
use hash::sum_block_hash;

const BLOCK_SIZE: u64 = 1024 * 1024;
const K: u64 = 1000;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        println!("Usage: cargo run --release --bin pipeline filepath block_size K");
        return;
    }
    let start = std::time::Instant::now();

    let block_size = args[2].parse().unwrap_or(BLOCK_SIZE);
    let k = args[3].parse().unwrap_or(K);

    let file_res = File::open(args[1].clone());
    if file_res.is_err() {
        eprintln!("File not found: {}", args[1]);
        return;
    }
    let file = file_res.unwrap();

    let metadata = file.metadata();
    if metadata.is_err() {
        eprintln!("Error opening file: {}", args[1]);
        return;
    }
    let total_size = metadata.unwrap().len();
    let num_blocks = (total_size + block_size - 1) / block_size;

    let channel_size = 64;
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(channel_size);

    let compute_thread = std::thread::spawn(move || {
        let mut result = vec![0u8; 16];
        for received in rx {
            let block_hash = stretch_md5(received.as_slice(), k);
            sum_block_hash(&mut result, &block_hash);
        }
        result
    });

    let read_thread = std::thread::spawn(move || {
        let mut buffer = vec![0u8; block_size as usize];
        for block_idx in 0..num_blocks {
            let offset = block_idx * block_size;
            let bytes_to_read = std::cmp::min(block_size, total_size - offset) as usize;

            // read
            let target_buf = &mut buffer[..bytes_to_read];
            file.read_exact_at(target_buf, offset)?;

            // send to compute thread
            tx.send(target_buf.to_vec()).unwrap();
        }
        Ok::<(), std::io::Error>(())
    });

    if let Err(err) = read_thread.join().unwrap() {
        eprintln!("Error reading file: {}", err);
        return;
    }

    let result = compute_thread.join().unwrap();

    print!("{} 0x", args[1].clone());
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");
    println!("Time: {:?}", start.elapsed());
}
