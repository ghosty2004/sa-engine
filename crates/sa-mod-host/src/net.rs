//! The mods' network sockets: one [`Network`] per platform (natively real UDP sockets, in the browser WebSockets to
//! the page's UDP proxy; `backend`), and each mod's open sockets with its per-frame send budget ([`Sockets`]).

use std::io;
use std::rc::Rc;

use sa_mod_api::net::{MAX_SEND_BYTES_PER_FRAME, MAX_SENDS_PER_FRAME, MAX_SOCKETS};
pub use sa_net::{DatagramTransport, LinkState};

/// Where the mods' sockets go.
pub trait Network {
    /// Opens a connected, non-blocking datagram socket to `target` (`HOST:PORT`).
    fn open(&self, target: &str) -> io::Result<Box<dyn DatagramTransport>>;
    /// What it is, for the log.
    fn describe(&self) -> String;
}

/// The network the host gives its mods.
#[derive(Clone)]
pub struct Net(pub Rc<dyn Network>);

impl std::fmt::Debug for Net {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Net({})", self.0.describe())
    }
}

/// One mod's sockets (a handle is the index of its slot) and what it sent in the current main-loop iteration.
#[derive(Default)]
pub struct Sockets {
    slots: Vec<Option<Box<dyn DatagramTransport>>>,
    sends: u32,
    bytes: usize,
}

impl std::fmt::Debug for Sockets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sockets({} open)", self.open())
    }
}

impl Sockets {
    /// The number of open sockets.
    pub fn open(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    /// Takes `socket` into a free slot: its handle, or `None` when the mod holds [`MAX_SOCKETS`].
    pub fn add(&mut self, socket: Box<dyn DatagramTransport>) -> Option<usize> {
        let i = match self.slots.iter().position(Option::is_none) {
            Some(i) => i,
            None if self.slots.len() < MAX_SOCKETS => {
                self.slots.push(None);
                self.slots.len() - 1
            }
            None => return None,
        };
        self.slots[i] = Some(socket);
        Some(i)
    }

    /// Whether a socket can be added.
    pub fn has_room(&self) -> bool {
        self.open() < MAX_SOCKETS
    }

    /// The socket of `handle`.
    pub fn get(&mut self, handle: i32) -> Option<&mut Box<dyn DatagramTransport>> {
        self.slots.get_mut(usize::try_from(handle).ok()?)?.as_mut()
    }

    /// Closes the socket of `handle`; false when there is none.
    pub fn close(&mut self, handle: i32) -> bool {
        usize::try_from(handle).ok().and_then(|i| self.slots.get_mut(i)).and_then(Option::take).is_some()
    }

    /// Counts a send of `len` bytes against the frame's budget; false (nothing counted) when it is used up.
    pub fn charge(&mut self, len: usize) -> bool {
        if self.sends >= MAX_SENDS_PER_FRAME || self.bytes + len > MAX_SEND_BYTES_PER_FRAME {
            return false;
        }
        self.sends += 1;
        self.bytes += len;
        true
    }

    /// A new main-loop iteration: a new send budget.
    pub fn new_frame(&mut self) {
        self.sends = 0;
        self.bytes = 0;
    }
}
