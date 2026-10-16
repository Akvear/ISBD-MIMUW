use rand::rngs::StdRng;
use rand::{Rng, RngExt, SeedableRng, rng};
use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

mod hash;
use hash::sum_block_hash;

pub struct RandomOrderIterator {
    elements: Vec<u64>,
    rng: Box<dyn Rng>,
}

impl RandomOrderIterator {
    pub fn with_seed(size: u64, seed: u64) -> Self {
        Self {
            elements: (0 as u64..size).collect(),
            rng: Box::new(StdRng::seed_from_u64(seed)),
        }
    }

    pub fn new(size: u64) -> Self {
        Self {
            elements: (0..size).collect(),
            rng: Box::new(rng()),
        }
    }
}

impl Iterator for RandomOrderIterator {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.elements.is_empty() {
            None
        } else {
            let rand_idx = self.rng.random_range(0..self.elements.len());
            Some(self.elements.swap_remove(rand_idx))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.elements.len(), Some(self.elements.len()))
    }
}

const BLOCK_SIZE: u64 = 512;
const RAND_SEED: u64 = 1234;

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

    let metadata = file.metadata();
    if metadata.is_err() {
        eprintln!("Error opening file: {}", args[1]);
        return;
    }
    let total_size = metadata.unwrap().len();
    let num_blocks = (total_size + BLOCK_SIZE - 1) / BLOCK_SIZE;
    let random_iter = RandomOrderIterator::with_seed(num_blocks, RAND_SEED);

    let offsets: Arc<Vec<u64>> = Arc::new(random_iter.collect());
    let total_tasks = offsets.len();
    let current_task = Arc::new(AtomicUsize::new(0));

    // let mut buffer = vec![0u8; BLOCK_SIZE as usize];
    let mut result = vec![0u8; 16];

    // initialize the computing thread
    let channel_buffer_size = 64;
    let (tx, rx) = std::sync::mpsc::sync_channel(channel_buffer_size);

    let compute_thread = std::thread::spawn(move || {
        // receive from the channel
        for received in rx {
            let block_hash = md5::compute(received);
            sum_block_hash(&mut result, &block_hash);
        }

        result
    });

    // initialize the reading threads
    let num_threads = 16;
    let mut threads = Vec::new();

    for _ in 0..num_threads {
        let tx_clone = tx.clone();
        let file_clone = file.try_clone().unwrap();
        let current_task = Arc::clone(&current_task);
        let offsets = Arc::clone(&offsets);

        let thread = std::thread::spawn(move || {
            let mut buffer = vec![0u8; BLOCK_SIZE as usize];

            loop {
                let task_id = current_task.fetch_add(1, Ordering::Relaxed);
                if task_id >= total_tasks {
                    break;
                }

                let block_id = offsets[task_id];
                let offset = block_id * BLOCK_SIZE;
                let bytes_to_read = std::cmp::min(BLOCK_SIZE, total_size - offset) as usize;

                let read_result = file_clone.read_exact_at(&mut buffer[..bytes_to_read], offset);
                if read_result.is_err() {
                    eprintln!(
                        "Error reading file at offset {}: {}",
                        offset,
                        read_result.unwrap_err()
                    );
                    continue;
                }

                tx_clone.send(buffer.clone()).unwrap();
            }
        });
        threads.push(thread);
    }

    for thread in threads {
        thread.join().unwrap();
    }
    drop(tx);
    let result = compute_thread.join().unwrap();

    print!("{} 0x", args[1].clone());
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");
    println!("Time: {:?}", start.elapsed());
}
