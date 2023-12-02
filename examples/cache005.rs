use std::sync::atomic::Ordering::Relaxed;
use std::thread::spawn;
use std::time::Instant;
use std::{hint::black_box, sync::atomic::AtomicU64};

#[repr(align(64))]
struct Aligned(AtomicU64);

static A: [Aligned; 3] = [
    Aligned(AtomicU64::new(0)),
    Aligned(AtomicU64::new(0)),
    Aligned(AtomicU64::new(0)),
];

fn main() {
    black_box(&A);

    black_box(spawn(|| loop {
        A[0].0.store(0, Relaxed);
        A[2].0.store(0, Relaxed);
    }));

    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        black_box(A[1].0.load(Relaxed));
    }

    println!("{} ms", start.elapsed().as_millis());
}
