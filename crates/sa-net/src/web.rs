//! Browser WebSocket transport (wasm32): `web_sys::WebSocket` to `sa-udp-proxy`, one binary frame per datagram.
//!
//! The browser delivers WebSocket events on the thread that created the socket (main thread or a worker),
//! between two turns of its event loop. The callbacks only push into a shared queue; RakNet's update polls
//! [`DatagramTransport::recv`] from its frame tick, so it must run on the same thread and return to the event
//! loop between ticks (no blocking update thread on wasm; samp-rs's `threaded = false` path).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{self, ErrorKind};
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{BinaryType, CloseEvent, Event, MessageEvent, WebSocket};

use crate::{
    Connector, DatagramTransport, INBOX_LIMIT, LinkState, PENDING_LIMIT, ProxyReply, connect_frame,
    parse_reply,
};

#[derive(Debug)]
struct Shared {
    state: LinkState,
    /// Datagrams sent before the proxy said `connected`.
    pending: Vec<Vec<u8>>,
    inbox: VecDeque<Vec<u8>>,
}

type Handler<T> = Closure<dyn FnMut(T)>;

/// A browser WebSocket to the proxy, relaying to one server.
pub struct WebSocketTransport {
    ws: WebSocket,
    shared: Rc<RefCell<Shared>>,
    _on_open: Handler<Event>,
    _on_message: Handler<MessageEvent>,
    _on_close: Handler<CloseEvent>,
    _on_error: Handler<Event>,
}

fn js_err(e: wasm_bindgen::JsValue) -> io::Error {
    io::Error::other(format!("{e:?}"))
}

fn close_with(shared: &RefCell<Shared>, why: String) {
    let mut s = shared.borrow_mut();
    if !matches!(s.state, LinkState::Closed(_)) {
        s.state = LinkState::Closed(why);
    }
}

impl WebSocketTransport {
    /// Starts opening `proxy_url` (`ws://` or `wss://`) for `server` (`host:port`). Returns at once in
    /// [`LinkState::Connecting`]; datagrams sent meanwhile are queued (up to [`PENDING_LIMIT`]).
    pub fn open(proxy_url: &str, server: &str) -> io::Result<Self> {
        let frame = connect_frame(server)?;
        let ws = WebSocket::new(proxy_url).map_err(js_err)?;
        ws.set_binary_type(BinaryType::Arraybuffer);
        let shared = Rc::new(RefCell::new(Shared {
            state: LinkState::Connecting,
            pending: Vec::new(),
            inbox: VecDeque::new(),
        }));

        let on_open: Handler<Event> = {
            let (ws, shared) = (ws.clone(), Rc::clone(&shared));
            Closure::new(move |_: Event| {
                if let Err(e) = ws.send_with_str(&frame) {
                    close_with(&shared, format!("send connect: {e:?}"));
                }
            })
        };
        let on_message: Handler<MessageEvent> = {
            let (ws, shared) = (ws.clone(), Rc::clone(&shared));
            Closure::new(move |ev: MessageEvent| {
                let data = ev.data();
                if let Some(buf) = data.dyn_ref::<js_sys::ArrayBuffer>() {
                    let mut s = shared.borrow_mut();
                    if s.state == LinkState::Open && s.inbox.len() < INBOX_LIMIT {
                        s.inbox.push_back(js_sys::Uint8Array::new(buf).to_vec());
                    }
                    return;
                }
                let text = data.as_string().unwrap_or_default();
                let connecting = shared.borrow().state == LinkState::Connecting;
                match parse_reply(&text) {
                    ProxyReply::Connected if connecting => {
                        let pending = {
                            let mut s = shared.borrow_mut();
                            s.state = LinkState::Open;
                            std::mem::take(&mut s.pending)
                        };
                        for d in pending {
                            if let Err(e) = ws.send_with_u8_array(&d) {
                                close_with(&shared, format!("send: {e:?}"));
                                break;
                            }
                        }
                    }
                    ProxyReply::Connected => {}
                    ProxyReply::Error(e) => {
                        close_with(&shared, format!("proxy: {e}"));
                        let _ = ws.close();
                    }
                }
            })
        };
        let on_close: Handler<CloseEvent> = {
            let shared = Rc::clone(&shared);
            Closure::new(move |ev: CloseEvent| {
                close_with(&shared, format!("closed ({} {})", ev.code(), ev.reason()));
            })
        };
        let on_error: Handler<Event> = {
            let shared = Rc::clone(&shared);
            Closure::new(move |_: Event| close_with(&shared, "websocket error".into()))
        };
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        Ok(Self {
            ws,
            shared,
            _on_open: on_open,
            _on_message: on_message,
            _on_close: on_close,
            _on_error: on_error,
        })
    }

    /// Bytes the browser has queued but not yet sent (back-pressure hint).
    pub fn buffered_amount(&self) -> u32 {
        self.ws.buffered_amount()
    }
}

impl DatagramTransport for WebSocketTransport {
    fn send(&mut self, datagram: &[u8]) -> io::Result<usize> {
        let mut s = self.shared.borrow_mut();
        match &s.state {
            LinkState::Connecting => {
                if s.pending.len() < PENDING_LIMIT {
                    s.pending.push(datagram.to_vec());
                }
                Ok(datagram.len())
            }
            LinkState::Open => {
                drop(s);
                self.ws.send_with_u8_array(datagram).map_err(|e| {
                    close_with(&self.shared, format!("send: {e:?}"));
                    io::Error::new(ErrorKind::NotConnected, "websocket send failed")
                })?;
                Ok(datagram.len())
            }
            LinkState::Closed(why) => Err(io::Error::new(ErrorKind::NotConnected, why.clone())),
        }
    }

    fn recv(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Some(d) = self.shared.borrow_mut().inbox.pop_front() else {
            return Err(ErrorKind::WouldBlock.into());
        };
        let n = d.len().min(buf.len());
        buf[..n].copy_from_slice(&d[..n]);
        Ok(n)
    }

    fn state(&self) -> LinkState {
        self.shared.borrow().state.clone()
    }
}

impl Drop for WebSocketTransport {
    fn drop(&mut self) {
        self.ws.set_onopen(None);
        self.ws.set_onmessage(None);
        self.ws.set_onclose(None);
        self.ws.set_onerror(None);
        let _ = self.ws.close();
    }
}

/// Opens [`WebSocketTransport`]s through one proxy.
#[derive(Clone, Debug)]
pub struct WebSocketConnector {
    /// `ws://` / `wss://` URL of the proxy.
    pub proxy_url: String,
}

impl Connector for WebSocketConnector {
    type Transport = WebSocketTransport;

    fn connect(&self, server: &str) -> io::Result<WebSocketTransport> {
        WebSocketTransport::open(&self.proxy_url, server)
    }
}
