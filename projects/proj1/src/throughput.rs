use memmap::Mmap;
use std::env;
use std::fs::File;

mod hash;
use hash::sum_block_hash;

const BLOCK_SIZE: u64 = 512;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: cargo run --release --bin latency filepath");
        return;
    }
    let start = std::time::Instant::now();

    let file_res = File::open(args[1].clone());
    if file_res.is_err() {
        eprintln!("File not found: {}", args[1]);
        return;
    }
    let file = file_res.unwrap();

    // SAFETY:
    // The file exists and is not mutated by any other program
    let mmap = unsafe { Mmap::map(&file).expect("failed to create mmap") };

    let metadata = file.metadata();
    if metadata.is_err() {
        eprintln!("Error opening file: {}", args[1]);
        return;
    }

    let mut result = vec![0u8; 16];

    for chunk in mmap.chunks(BLOCK_SIZE as usize) {
        let block_hash = md5::compute(chunk);
        sum_block_hash(&mut result, &block_hash);
    }

    print!("{} 0x", args[1].clone());
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");
    println!("Time: {:?}", start.elapsed());
}
