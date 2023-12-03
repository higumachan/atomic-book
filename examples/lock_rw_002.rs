use atomic_wait::{wait, wake_all, wake_one};
use std::cell::UnsafeCell;
use std::hint::black_box;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

const WRITE_LOCK_STATE: u32 = u32::MAX;

pub struct RwLock<T> {
    state: AtomicU32,
    writer_wake_counter: AtomicU32,
    value: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            writer_wake_counter: AtomicU32::new(0),
            value: UnsafeCell::new(value),
        }
    }

    pub fn read(&self) -> ReadGuard<'_, T> {
        let mut s = self.state.load(Ordering::Relaxed);

        loop {
            if s < WRITE_LOCK_STATE {
                assert!(s != WRITE_LOCK_STATE - 1, "too many readers");
                match self.state.compare_exchange_weak(
                    s,
                    s + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return ReadGuard { rwlock: self },
                    Err(e) => s = e,
                }
            }
            if s == WRITE_LOCK_STATE {
                wait(&self.state, WRITE_LOCK_STATE);
                s = self.state.load(Ordering::Relaxed);
            }
        }
    }

    pub fn write(&self) -> WriteGuard<'_, T> {
        while let Err(s) =
            self.state
                .compare_exchange(0, WRITE_LOCK_STATE, Ordering::Acquire, Ordering::Relaxed)
        {
            let w = self.writer_wake_counter.load(Ordering::Acquire);
            if self.state.load(Ordering::Relaxed) != 0 {
                wait(&self.writer_wake_counter, w);
            }
        }
        WriteGuard { rwlock: self }
    }
}

unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

pub struct ReadGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        if self.rwlock.state.fetch_sub(1, Ordering::Release) == 1 {
            self.rwlock
                .writer_wake_counter
                .fetch_add(1, Ordering::Release);
            wake_one(&self.rwlock.writer_wake_counter);
        }
    }
}

impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.rwlock.value.get() }
    }
}

pub struct WriteGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        self.rwlock.state.store(0, Ordering::Release);
        self.rwlock
            .writer_wake_counter
            .fetch_add(1, Ordering::Release);
        wake_one(&self.rwlock.writer_wake_counter);
        wake_all(&self.rwlock.state);
    }
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.rwlock.value.get() }
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.rwlock.value.get() }
    }
}

fn main() {
    println!("lock_rw_002");
    let counter = RwLock::new(0);
    black_box(&counter);

    let start = Instant::now();
    std::thread::scope(|s| {
        let t1 = s.spawn({
            || {
                for _i in 0..1000_000_0 {
                    let mut c = counter.write();
                    black_box(&c);
                    *c += 1;
                }
            }
        });
        let t2 = s.spawn({
            || {
                for _i in 0..1000_000_0 {
                    let mut c = counter.write();
                    black_box(&c);
                    *c += 2;
                }
            }
        });
        t1.join().unwrap();
        t2.join().unwrap();
    });
    assert_eq!(*counter.write(), 30000000);
    println!("{} ms", start.elapsed().as_millis());

    let counter = RwLock::new(0);
    black_box(&counter);

    // 誰もいない場合
    let start = Instant::now();
    std::thread::scope(|s| {
        let t1 = s.spawn({
            || {
                for _i in 0..1000_000_0 {
                    let mut c = black_box(counter.write());
                    black_box(&c);
                    *c += 1;
                }
            }
        });
        t1.join().unwrap();
    });
    assert_eq!(*counter.read(), 10000000);
    println!("{} ms", start.elapsed().as_millis());
}
