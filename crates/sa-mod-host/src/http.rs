//! The mods' HTTP requests: one [`HttpClient`] per platform (`backend`: natively a thread per request, in the browser
//! the page's `fetch`), and each mod's requests with their time and size limits ([`Requests`]).

use std::rc::Rc;
use std::time::Duration;

use sa_mod_api::http::{MAX_BODY, MAX_REQUESTS, TIMEOUT_MS};

/// Where a request stands.
#[derive(Debug, PartialEq, Eq)]
pub enum Progress {
    /// Still running.
    Running,
    /// The whole response is there.
    Done {
        /// The HTTP status.
        status: u16,
        /// The body.
        body: Vec<u8>,
    },
    /// It failed: why.
    Failed(String),
}

/// One GET request running in the background.
pub trait HttpRequest {
    /// Where it stands; once it returned `Done` or `Failed` it is not polled again.
    fn poll(&mut self) -> Progress;
}

/// Where the mods' requests go.
pub trait HttpClient {
    /// Starts a GET request of `url` (checked against the mod's `http` list), with at most
    /// [`MAX_BODY`] bytes of body and [`TIMEOUT_MS`] in all.
    fn get(&self, url: &str) -> Box<dyn HttpRequest>;
    /// What it is, for the log.
    fn describe(&self) -> String;
}

/// The HTTP client the host gives its mods.
#[derive(Clone)]
pub struct Http(pub Rc<dyn HttpClient>);

impl std::fmt::Debug for Http {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Http({})", self.0.describe())
    }
}

enum Slot {
    Running(Box<dyn HttpRequest>, web_time::Instant),
    Done(u16, Vec<u8>),
    Failed,
}

/// What [`Requests::poll`] found.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome<'a> {
    /// Still running.
    Running,
    /// The status and the body.
    Done(u16, &'a [u8]),
    /// It failed.
    Failed,
}

/// One mod's requests (a handle is the index of its slot).
#[derive(Default)]
pub struct Requests {
    slots: Vec<Option<Slot>>,
}

impl std::fmt::Debug for Requests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Requests({})", self.slots.iter().flatten().count())
    }
}

impl Requests {
    /// Whether a request can be added.
    pub fn has_room(&self) -> bool {
        self.slots.iter().flatten().count() < MAX_REQUESTS
    }

    /// Takes `request` into a free slot: its handle, or `None` when the mod holds [`MAX_REQUESTS`].
    pub fn add(&mut self, request: Box<dyn HttpRequest>) -> Option<usize> {
        let slot = Some(Slot::Running(request, web_time::Instant::now()));
        if let Some(i) = self.slots.iter().position(Option::is_none) {
            self.slots[i] = slot;
            return Some(i);
        }
        if self.slots.len() >= MAX_REQUESTS {
            return None;
        }
        self.slots.push(slot);
        Some(self.slots.len() - 1)
    }

    /// Where request `handle` stands (`None`: no such request), and why it failed when it failed in this poll.
    pub fn poll(&mut self, handle: i32) -> Option<(Outcome<'_>, Option<String>)> {
        let slot = self.slots.get_mut(usize::try_from(handle).ok()?)?.as_mut()?;
        let mut why = None;
        if let Slot::Running(r, start) = slot {
            let next = match r.poll() {
                Progress::Running if start.elapsed() > Duration::from_millis(TIMEOUT_MS) => {
                    Some(Err(format!("no answer within {} s", TIMEOUT_MS / 1000)))
                }
                Progress::Running => None,
                Progress::Done { body, .. } if body.len() > MAX_BODY => {
                    Some(Err(format!("the body is over {MAX_BODY} bytes")))
                }
                Progress::Done { status, body } => Some(Ok((status, body))),
                Progress::Failed(e) => Some(Err(e)),
            };
            match next {
                None => {}
                Some(Ok((status, body))) => *slot = Slot::Done(status, body),
                Some(Err(e)) => {
                    why = Some(e);
                    *slot = Slot::Failed;
                }
            }
        }
        let outcome = match slot {
            Slot::Running(..) => Outcome::Running,
            Slot::Done(status, body) => Outcome::Done(*status, body),
            Slot::Failed => Outcome::Failed,
        };
        Some((outcome, why))
    }

    /// Forgets request `handle`; false when there is none.
    pub fn close(&mut self, handle: i32) -> bool {
        usize::try_from(handle)
            .ok()
            .and_then(|i| self.slots.get_mut(i))
            .and_then(Option::take)
            .is_some()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A request that answers after `polls` polls.
    pub struct Canned {
        pub polls: u32,
        pub answer: Option<Progress>,
    }

    impl HttpRequest for Canned {
        fn poll(&mut self) -> Progress {
            if self.polls > 0 {
                self.polls -= 1;
                return Progress::Running;
            }
            self.answer
                .take()
                .unwrap_or(Progress::Failed("polled twice".into()))
        }
    }

    /// Answers every URL with its own bytes and status 200 after one poll, `.../fail` with a failure.
    pub struct Echo;

    impl HttpClient for Echo {
        fn get(&self, url: &str) -> Box<dyn HttpRequest> {
            let answer = if url.ends_with("/fail") {
                Progress::Failed("refused".into())
            } else {
                Progress::Done {
                    status: 200,
                    body: url.as_bytes().to_vec(),
                }
            };
            Box::new(Canned {
                polls: 1,
                answer: Some(answer),
            })
        }

        fn describe(&self) -> String {
            "echo".into()
        }
    }

    #[test]
    fn slots_limits_and_outcomes() {
        let mut r = Requests::default();
        let echo = Echo;
        let handles: Vec<i32> = (0..MAX_REQUESTS)
            .map(|i| r.add(echo.get(&format!("http://a/{i}"))).unwrap() as i32)
            .collect();
        assert!(!r.has_room() && r.add(echo.get("http://a/x")).is_none());
        assert_eq!(r.poll(0).unwrap().0, Outcome::Running);
        assert_eq!(r.poll(0).unwrap().0, Outcome::Done(200, b"http://a/0"));
        assert_eq!(r.poll(0).unwrap().0, Outcome::Done(200, b"http://a/0"));
        assert!(r.close(handles[1]) && !r.close(handles[1]) && r.poll(1).is_none());
        let h = r.add(echo.get("http://a/fail")).unwrap() as i32;
        assert_eq!(h, 1);
        r.poll(h);
        assert_eq!(r.poll(h), Some((Outcome::Failed, Some("refused".into()))));
        assert_eq!(r.poll(h), Some((Outcome::Failed, None)));
        let big = Progress::Done {
            status: 200,
            body: vec![0; MAX_BODY + 1],
        };
        r.close(h);
        let h = r
            .add(Box::new(Canned {
                polls: 0,
                answer: Some(big),
            }))
            .unwrap() as i32;
        assert!(matches!(r.poll(h), Some((Outcome::Failed, Some(_)))));
    }
}
