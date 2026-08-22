extern crate self as parking_lot;

pub struct Mutex<T>(std::sync::Mutex<T>);

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self(std::sync::Mutex::new(value))
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, T> {
        self.0.lock().expect("ASR Miri harness mutex poisoned")
    }
}

pub mod asr;
