//! **J**oin **O**n **D**rop thread (`jod_thread`) is a thin wrapper around `std::thread`,
//! which makes sure that by default all threads are joined.

use std::fmt;

/// Like `thread::JoinHandle`, but joins the thread on drop and propagates
/// panics by default.
pub struct JoinHandle<T = ()>(Option<std::thread::JoinHandle<T>>);

impl<T> fmt::Debug for JoinHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("JoinHandle { .. }")
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        if let Some(inner) = self.0.take() {
            let res = inner.join();
            if res.is_err() && !std::thread::panicking() {
                res.unwrap();
            }
        }
    }
}

impl<T> JoinHandle<T> {
    pub fn thread(&self) -> &std::thread::Thread {
        self.0.as_ref().unwrap().thread()
    }
    pub fn join(mut self) -> T {
        let inner = self.0.take().unwrap();
        inner.join().unwrap()
    }
    pub fn detach(mut self) -> std::thread::JoinHandle<T> {
        let inner = self.0.take().unwrap();
        inner
    }
}

impl<T> From<std::thread::JoinHandle<T>> for JoinHandle<T> {
    fn from(inner: std::thread::JoinHandle<T>) -> JoinHandle<T> {
        JoinHandle(Some(inner))
    }
}

#[derive(Debug)]
pub struct Builder(std::thread::Builder);

impl Builder {
    pub fn new() -> Builder {
        Builder(std::thread::Builder::new())
    }

    pub fn name(self, name: String) -> Builder {
        Builder(self.0.name(name))
    }

    pub fn stack_size(self, size: usize) -> Builder {
        Builder(self.0.stack_size(size))
    }

    pub fn spawn<F, T>(self, f: F) -> std::io::Result<JoinHandle<T>>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        self.0.spawn(f).map(JoinHandle::from)
    }
}

pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    Builder::new().spawn(f).expect("failed to spawn thread")
}

#[test]
fn smoke() {
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    drop(spawn(|| COUNTER.fetch_add(1, Ordering::SeqCst)));
    assert_eq!(COUNTER.load(Ordering::SeqCst), 1);

    let res = std::panic::catch_unwind(|| {
        let _handle = Builder::new()
            .name("panicky".to_string())
            .spawn(|| COUNTER.fetch_add(1, Ordering::SeqCst))
            .unwrap();
        panic!("boom")
    });
    assert!(res.is_err());

    assert_eq!(COUNTER.load(Ordering::SeqCst), 2);

    let res = std::panic::catch_unwind(|| {
        let handle = spawn(|| panic!("boom"));
        let () = handle.join();
    });
    assert!(res.is_err());

    let res = std::panic::catch_unwind(|| {
        let _handle = spawn(|| panic!("boom"));
    });
    assert!(res.is_err());
}
