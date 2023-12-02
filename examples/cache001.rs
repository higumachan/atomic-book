use std::sync::atomic::Ordering::Relaxed;
use std::time::Instant;
use std::{hint::black_box, sync::atomic::AtomicU64};

static A: AtomicU64 = AtomicU64::new(0);

fn main() {
    black_box(&A);

    let start = Instant::now();

    for _ in 0..1_000_000_000 {
        black_box(A.load(Relaxed));
    }

    println!("{} ms", start.elapsed().as_millis());
}
