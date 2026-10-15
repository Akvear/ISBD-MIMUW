use rand::rngs::StdRng;
use rand::{Rng, RngExt, SeedableRng, rng};
use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;

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

    // let mut buffer = vec![0u8; BLOCK_SIZE as usize];
    let mut result = vec![0u8; 16];

    // initialize the computing thread
    // play around with this value to see how it affects the performance
    let channel_buffer_size = 16;

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
    let condvar = std::sync::Arc::new(std::sync::Condvar::new());
    let read_queue = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
    let mut threads = Vec::new();

    for _ in 0..num_threads {
        let tx_clone = tx.clone();
        let file_clone = file.try_clone().unwrap();
        let read_queue_clone = read_queue.clone();
        let condvar_clone = condvar.clone();

        let thread = std::thread::spawn(move || {
            loop {
                let (bytes_to_read, offset) = {
                    let mut queue = read_queue_clone.lock().unwrap();
                    while queue.is_empty() {
                        queue = condvar_clone.wait(queue).unwrap();
                    }
                    queue.remove(0)
                };

                if bytes_to_read == 0 {
                    break;
                }

                let mut buffer = vec![0u8; bytes_to_read as usize];
                let read_result = file_clone.read_exact_at(&mut buffer, offset);
                if read_result.is_err() {
                    eprintln!(
                        "Error reading file at offset {}: {}",
                        offset,
                        read_result.unwrap_err()
                    );
                    continue;
                }

                tx_clone.send(buffer).unwrap();
            }
        });
        threads.push(thread);
    }

    for block_idx in random_iter {
        let offset = block_idx * BLOCK_SIZE;
        let bytes_to_read = std::cmp::min(BLOCK_SIZE, total_size - offset) as usize;

        read_queue.lock().unwrap().push((bytes_to_read, offset));
        condvar.notify_one();
    }

    // Notify all threads that they can stop
    for _ in 0..num_threads {
        read_queue.lock().unwrap().push((0, 0));
    }
    condvar.notify_all();
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
