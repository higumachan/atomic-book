use atomic_wait::{wait, wake_one};
use std::cell::UnsafeCell;
use std::hint::black_box;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

struct Mutex<T> {
    // 0: unlocked
    // 1: locked
    state: AtomicU32,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for Mutex<T> where T: Send {}

impl<T> Mutex<T> {
    fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            value: UnsafeCell::new(value),
        }
    }

    fn lock(&self) -> MutexGuard<T> {
        while self.state.swap(1, Ordering::Acquire) == 1 {
            // すでにロックされている
            wait(&self.state, 1);
        }
        MutexGuard { mutex: self }
    }
}

struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

unsafe impl<T> Sync for MutexGuard<'_, T> where T: Sync {}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.state.store(0, Ordering::Release);
        wake_one(&self.mutex.state)
    }
}

fn main() {
    let counter = Mutex::new(0);
    black_box(&counter);

    let start = Instant::now();
    std::thread::scope(|s| {
        let t1 = s.spawn({
            || {
                for i in 0..1000_000_0 {
                    let mut c = counter.lock();
                    *c += 1;
                }
            }
        });
        let t2 = s.spawn({
            || {
                for i in 0..1000_000_0 {
                    let mut c = counter.lock();
                    *c += 2;
                }
            }
        });
        t1.join().unwrap();
        t2.join().unwrap();
    });
    assert_eq!(*counter.lock(), 30000000);
    println!("{} ms", start.elapsed().as_millis());

    let counter = Mutex::new(0);
    black_box(&counter);

    // 誰もいない場合
    let start = Instant::now();
    std::thread::scope(|s| {
        let t1 = s.spawn({
            || {
                for i in 0..1000_000_0 {
                    let mut c = black_box(counter.lock());
                    *c += 1;
                }
            }
        });
        t1.join().unwrap();
    });
    assert_eq!(*counter.lock(), 10000000);
    println!("{} ms", start.elapsed().as_millis());
}
