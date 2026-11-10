//! Background loader (viewer code, not the game's `CStreaming`/`CdStream`): reads DFF/TXD
//! entries from the IMG archives and builds the `sa-rw` objects on worker threads. GPU uploads
//! happen on the main thread.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use sa_platform::fs::{GameFs, RandomRead};

use crate::world::EntryRef;

pub enum Request {
    Model { model: usize, entry: EntryRef },
    Txd { txd: usize, entry: EntryRef },
}

pub enum Loaded {
    Model { model: usize, result: Result<sa_rw::Clump, String> },
    Txd { txd: usize, result: Result<sa_rw::TexDictionary, String> },
}

pub struct Loader {
    tx: Sender<Request>,
    rx: Receiver<Loaded>,
    pub in_flight: usize,
    pub bytes_read: u64,
}

fn read(files: &mut [Box<dyn RandomRead>], e: EntryRef) -> Result<Vec<u8>, String> {
    let f = files.get_mut(e.img).ok_or("bad archive index")?;
    let len = e.len.min(f.len().saturating_sub(e.offset) as usize);
    let mut buf = vec![0u8; len];
    f.read_at(e.offset, &mut buf).map_err(|err| err.to_string())?;
    Ok(buf)
}

impl Loader {
    pub fn new(fs: Arc<dyn GameFs>, archives: Arc<Vec<String>>, workers: usize) -> Loader {
        let (tx, req_rx) = channel::<Request>();
        let (res_tx, rx) = channel::<Loaded>();
        let req_rx = Arc::new(Mutex::new(req_rx));
        for n in 0..workers.max(1) {
            let (fs, archives, req_rx, res_tx) = (fs.clone(), archives.clone(), req_rx.clone(), res_tx.clone());
            sa_platform::spawn_worker(&format!("sa-loader-{n}"), move || {
                let mut files: Vec<Box<dyn RandomRead>> = Vec::new();
                for p in archives.iter() {
                    match fs.open(p) {
                        Ok(f) => files.push(f),
                        Err(e) => {
                            eprintln!("loader: cannot open {p}: {e}");
                            return;
                        }
                    }
                }
                loop {
                    let req = match req_rx.lock() {
                        Ok(r) => r.recv(),
                        Err(_) => return,
                    };
                    let Ok(req) = req else { return };
                    let out = match req {
                        Request::Model { model, entry } => Loaded::Model {
                            model,
                            result: read(&mut files, entry).and_then(|b| sa_rw::read_clump(&b).map_err(|e| e.to_string())),
                        },
                        Request::Txd { txd, entry } => Loaded::Txd {
                            txd,
                            result: read(&mut files, entry)
                                .and_then(|b| sa_rw::read_tex_dictionary(&b).map_err(|e| e.to_string())),
                        },
                    };
                    if res_tx.send(out).is_err() {
                        return;
                    }
                }
            });
        }
        Loader { tx, rx, in_flight: 0, bytes_read: 0 }
    }

    pub fn request(&mut self, r: Request) {
        self.bytes_read += match &r {
            Request::Model { entry, .. } | Request::Txd { entry, .. } => entry.len as u64,
        };
        if self.tx.send(r).is_ok() {
            self.in_flight += 1;
        }
    }

    /// A finished load, if any (`block` waits for one when something is in flight).
    pub fn poll(&mut self, block: bool) -> Option<Loaded> {
        let r = if block && self.in_flight > 0 { self.rx.recv().ok() } else { self.rx.try_recv().ok() };
        if r.is_some() {
            self.in_flight -= 1;
        }
        r
    }
}
