//! The kernel32 objects the ported code uses: semaphores, events, threads, `LocalAlloc` memory.
//!
//! `CdStream` (sa-stream) is the main user: it creates one semaphore per stream and one for the
//! request queue, a suspended streaming thread (`CreateThread(…, CdStreamThread, …,
//! CREATE_SUSPENDED)` + `ResumeThread`), `LocalAlloc`s its queue, and waits with
//! `WaitForSingleObject(Ex)`. [`Kernel`] keeps those objects behind integer handles, thread-safe
//! (the streaming thread and the game thread both call it). Thread entry points are code addresses
//! in the original; the caller resolves them to Rust functions ([`Kernel::create_thread`] takes the
//! resolved entry).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

/// `WaitForSingleObject` results.
pub const WAIT_OBJECT_0: u32 = 0;
pub const WAIT_TIMEOUT: u32 = 0x102;
pub const WAIT_FAILED: u32 = 0xFFFF_FFFF;
/// `INFINITE`.
pub const INFINITE: u32 = 0xFFFF_FFFF;

/// A counting semaphore (also used for auto-reset events: max 1).
#[derive(Debug)]
pub struct Semaphore {
    count: Mutex<i32>,
    max: i32,
    cv: Condvar,
}

impl Semaphore {
    pub fn new(initial: i32, max: i32) -> Semaphore {
        Semaphore {
            count: Mutex::new(initial),
            max,
            cv: Condvar::new(),
        }
    }

    /// `ReleaseSemaphore`: the previous count, or `None` if the count would exceed the maximum.
    pub fn release(&self, n: i32) -> Option<i32> {
        let mut c = self.count.lock().unwrap_or_else(|e| e.into_inner());
        if n <= 0 || *c + n > self.max {
            return None;
        }
        let prev = *c;
        *c += n;
        self.cv.notify_all();
        Some(prev)
    }

    /// Waits up to `ms` milliseconds ([`INFINITE`] = forever) for a count > 0 and takes one.
    pub fn wait(&self, ms: u32) -> u32 {
        let mut c = self.count.lock().unwrap_or_else(|e| e.into_inner());
        // wasm32 (one thread, no blocking): nobody else can release while we wait, so an unsignalled semaphore
        // times out at once, whatever the timeout
        #[cfg(target_arch = "wasm32")]
        #[allow(clippy::needless_return)] // the native code below is cfg'd out on wasm32
        {
            let _ = ms;
            if *c <= 0 {
                return WAIT_TIMEOUT;
            }
            *c -= 1;
            return WAIT_OBJECT_0;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if ms == INFINITE {
            while *c <= 0 {
                c = self.cv.wait(c).unwrap_or_else(|e| e.into_inner());
            }
        } else {
            let deadline = std::time::Instant::now() + Duration::from_millis(ms as u64);
            while *c <= 0 {
                let now = std::time::Instant::now();
                if now >= deadline {
                    return WAIT_TIMEOUT;
                }
                c = self
                    .cv
                    .wait_timeout(c, deadline - now)
                    .unwrap_or_else(|e| e.into_inner())
                    .0;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            *c -= 1;
            WAIT_OBJECT_0
        }
    }
}

/// A thread created suspended; [`Kernel::resume_thread`] starts it.
pub struct ThreadObj {
    entry: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    pub id: u32,
    pub priority: AtomicU32,
}

enum Obj {
    Semaphore(Arc<Semaphore>),
    Thread(Arc<ThreadObj>),
    /// A handle that is always signalled (files waited on with `WaitForSingleObjectEx`: every
    /// read completes synchronously).
    Signalled,
}

/// The kernel object table.
pub struct Kernel {
    objects: Mutex<HashMap<u32, Obj>>,
    memory: Mutex<HashMap<usize, Box<[u8]>>>,
    next: AtomicU32,
}

impl Default for Kernel {
    fn default() -> Self {
        Kernel {
            objects: Mutex::new(HashMap::new()),
            memory: Mutex::new(HashMap::new()),
            next: AtomicU32::new(0x0010_0000),
        }
    }
}

impl Kernel {
    /// Handles are multiples of 4 from 0x100000 (disjoint from [`crate::vfs::FileTable`]'s).
    pub const FIRST_HANDLE: u32 = 0x0010_0000;

    fn insert(&self, o: Obj) -> u32 {
        let h = self.next.fetch_add(4, Ordering::Relaxed);
        self.objects
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(h, o);
        h
    }

    pub fn is_kernel_handle(h: u32) -> bool {
        (Self::FIRST_HANDLE..0x4000_0000).contains(&h)
    }

    /// `CreateSemaphoreA(NULL, initial, max, name)` (names are ignored: one process).
    pub fn create_semaphore(&self, initial: i32, max: i32) -> u32 {
        self.insert(Obj::Semaphore(Arc::new(Semaphore::new(initial, max))))
    }

    /// An always-signalled pseudo object (a named event that exists once, a file wait).
    pub fn create_signalled(&self) -> u32 {
        self.insert(Obj::Signalled)
    }

    fn semaphore(&self, h: u32) -> Option<Arc<Semaphore>> {
        match self.objects.lock().unwrap_or_else(|e| e.into_inner()).get(&h)? {
            Obj::Semaphore(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// `ReleaseSemaphore`: the previous count, or `None` (FALSE).
    pub fn release_semaphore(&self, h: u32, n: i32) -> Option<i32> {
        self.semaphore(h)?.release(n)
    }

    /// `WaitForSingleObject(Ex)`. Non-kernel handles (files) count as signalled.
    pub fn wait(&self, h: u32, ms: u32) -> u32 {
        if !Self::is_kernel_handle(h) {
            return WAIT_OBJECT_0;
        }
        let o = match self.objects.lock().unwrap_or_else(|e| e.into_inner()).get(&h) {
            Some(Obj::Semaphore(s)) => Some(s.clone()),
            Some(_) => None,
            None => return WAIT_FAILED,
        };
        match o {
            Some(s) => s.wait(ms),
            // a thread handle: the game never waits for its threads to end
            None => WAIT_OBJECT_0,
        }
    }

    /// `CreateThread(…, entry, …, CREATE_SUSPENDED)`: a handle; `entry` runs on
    /// [`Kernel::resume_thread`].
    pub fn create_thread(&self, entry: Box<dyn FnOnce() + Send>) -> (u32, u32) {
        let id = self.next.fetch_add(4, Ordering::Relaxed);
        let t = Arc::new(ThreadObj {
            entry: Mutex::new(Some(entry)),
            id,
            priority: AtomicU32::new(0),
        });
        (self.insert(Obj::Thread(t)), id)
    }

    fn thread(&self, h: u32) -> Option<Arc<ThreadObj>> {
        match self.objects.lock().unwrap_or_else(|e| e.into_inner()).get(&h)? {
            Obj::Thread(t) => Some(t.clone()),
            _ => None,
        }
    }

    /// `ResumeThread`: starts a suspended thread; the previous suspend count (1), or
    /// `0xFFFFFFFF` for a bad handle. Resuming a running thread returns 0.
    pub fn resume_thread(&self, h: u32) -> u32 {
        let Some(t) = self.thread(h) else {
            return 0xFFFF_FFFF;
        };
        let Some(f) = t.entry.lock().unwrap_or_else(|e| e.into_inner()).take() else {
            return 0;
        };
        match std::thread::Builder::new()
            .name(format!("win32-thread-{:x}", t.id))
            .spawn(f)
        {
            Ok(_) => 1,
            Err(_) => 0xFFFF_FFFF,
        }
    }

    /// `SetThreadPriority` (recorded only: the host scheduler decides).
    pub fn set_thread_priority(&self, h: u32, p: i32) -> bool {
        self.thread(h)
            .map(|t| t.priority.store(p as u32, Ordering::Relaxed))
            .is_some()
    }

    pub fn thread_priority(&self, h: u32) -> i32 {
        self.thread(h)
            .map_or(0, |t| t.priority.load(Ordering::Relaxed) as i32)
    }

    /// `CloseHandle` of a kernel object.
    pub fn close(&self, h: u32) -> bool {
        self.objects
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&h)
            .is_some()
    }

    /// `LocalAlloc(LMEM_ZEROINIT, n)`: the address of `n` zeroed bytes (kept until
    /// [`Kernel::local_free`]).
    pub fn local_alloc(&self, n: usize) -> usize {
        let b = vec![0u8; n.max(1)].into_boxed_slice();
        let a = b.as_ptr() as usize;
        self.memory.lock().unwrap_or_else(|e| e.into_inner()).insert(a, b);
        a
    }

    /// `LocalFree`: true if `a` came from [`Kernel::local_alloc`].
    pub fn local_free(&self, a: usize) -> bool {
        self.memory
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&a)
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semaphores() {
        let k = Kernel::default();
        let s = k.create_semaphore(0, 2);
        assert_eq!(k.wait(s, 0), WAIT_TIMEOUT);
        assert_eq!(k.release_semaphore(s, 2), Some(0));
        assert_eq!(k.release_semaphore(s, 1), None);
        assert_eq!(k.wait(s, INFINITE), WAIT_OBJECT_0);
        assert_eq!(k.wait(s, 10), WAIT_OBJECT_0);
        assert_eq!(k.wait(s, 1), WAIT_TIMEOUT);
        assert_eq!(k.wait(0x2000, 0), WAIT_OBJECT_0, "file handles are signalled");
        assert!(k.close(s) && !k.close(s));
    }

    #[test]
    fn threads_start_on_resume() {
        let k = Arc::new(Kernel::default());
        let done = k.create_semaphore(0, 1);
        let k2 = k.clone();
        let (t, _) = k.create_thread(Box::new(move || {
            k2.release_semaphore(done, 1);
        }));
        assert_eq!(k.wait(done, 50), WAIT_TIMEOUT, "suspended");
        assert_eq!(k.resume_thread(t), 1);
        assert_eq!(k.wait(done, INFINITE), WAIT_OBJECT_0);
        assert_eq!(k.resume_thread(t), 0);
    }

    #[test]
    fn local_memory() {
        let k = Kernel::default();
        let a = k.local_alloc(16);
        assert!(a != 0 && k.local_free(a) && !k.local_free(a));
    }
}
