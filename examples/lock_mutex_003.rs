// lockされていて待機者がいない状態といる状態を分ける
use atomic_wait::{wait, wake_one};
use std::cell::UnsafeCell;
use std::hint::{black_box, spin_loop};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

struct Mutex<T> {
    // 0: unlocked
    // 1: locked not exist waiter
    // 2: locked exist waiter
    state: AtomicU32,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for Mutex<T> where T: Send {}

impl<T> Mutex<T> {
    #[inline]
    fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            value: UnsafeCell::new(value),
        }
    }

    #[inline]
    fn lock(&self) -> MutexGuard<T> {
        if self
            .state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            lock_contended(&self.state)
        }
        MutexGuard { mutex: self }
    }
}

#[cold]
fn lock_contended(state: &AtomicU32) {
    let mut spin_counter = 0;

    while state.load(Ordering::Relaxed) == 1 && spin_counter < 100 {
        spin_counter += 1;
        spin_loop();
    }

    if state
        .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        return;
    }

    while state.swap(2, Ordering::Acquire) != 0 {
        wait(&state, 2);
    }
}

struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

unsafe impl<T> Sync for MutexGuard<'_, T> where T: Sync {}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        if self.mutex.state.swap(0, Ordering::Release) == 2 {
            wake_one(&self.mutex.state);
        }
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
                    black_box(&c);
                    *c += 1;
                }
            }
        });
        let t2 = s.spawn({
            || {
                for i in 0..1000_000_0 {
                    let mut c = counter.lock();
                    black_box(&c);
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
                    black_box(&c);
                    *c += 1;
                }
            }
        });
        t1.join().unwrap();
    });
    assert_eq!(*counter.lock(), 10000000);
    println!("{} ms", start.elapsed().as_millis());
}
