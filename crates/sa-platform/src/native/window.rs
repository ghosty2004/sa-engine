//! winit window, event loop and input translation.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use crate::input::{InputEvent, InputState, Key, MouseButton};

/// The application driven by [`run_windowed`].
pub trait WindowApp {
    /// The window exists (called once).
    fn init(&mut self, window: Arc<Window>);
    /// The drawable size changed (physical pixels).
    fn resized(&mut self, width: u32, height: u32);
    /// One frame. Returns `false` to quit.
    fn frame(&mut self, input: &InputState) -> bool;
}

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// Left click captures the pointer and Escape releases it (the viewer's free camera). The
    /// game manages the pointer itself (DirectInput exclusive mode), so `sa-game` turns this off
    /// and calls [`set_pointer_capture`].
    pub click_to_capture: bool,
}

impl WindowConfig {
    pub fn new(title: &str, width: u32, height: u32) -> WindowConfig {
        WindowConfig {
            title: title.to_owned(),
            width,
            height,
            click_to_capture: true,
        }
    }
}

struct Runner<'a, A: WindowApp> {
    cfg: WindowConfig,
    app: &'a mut A,
    window: Option<Arc<Window>>,
    input: InputState,
    #[cfg(feature = "gamepad")]
    pads: Option<super::gamepad::Pads>,
}

/// winit's physical key → [`Key`].
pub fn map_key(k: KeyCode) -> Option<Key> {
    use KeyCode as C;
    Some(match k {
        C::KeyA => Key::A,
        C::KeyB => Key::B,
        C::KeyC => Key::C,
        C::KeyD => Key::D,
        C::KeyE => Key::E,
        C::KeyF => Key::F,
        C::KeyG => Key::G,
        C::KeyH => Key::H,
        C::KeyI => Key::I,
        C::KeyJ => Key::J,
        C::KeyK => Key::K,
        C::KeyL => Key::L,
        C::KeyM => Key::M,
        C::KeyN => Key::N,
        C::KeyO => Key::O,
        C::KeyP => Key::P,
        C::KeyQ => Key::Q,
        C::KeyR => Key::R,
        C::KeyS => Key::S,
        C::KeyT => Key::T,
        C::KeyU => Key::U,
        C::KeyV => Key::V,
        C::KeyW => Key::W,
        C::KeyX => Key::X,
        C::KeyY => Key::Y,
        C::KeyZ => Key::Z,
        C::Digit0 => Key::Digit(0),
        C::Digit1 => Key::Digit(1),
        C::Digit2 => Key::Digit(2),
        C::Digit3 => Key::Digit(3),
        C::Digit4 => Key::Digit(4),
        C::Digit5 => Key::Digit(5),
        C::Digit6 => Key::Digit(6),
        C::Digit7 => Key::Digit(7),
        C::Digit8 => Key::Digit(8),
        C::Digit9 => Key::Digit(9),
        C::Numpad0 => Key::Numpad(0),
        C::Numpad1 => Key::Numpad(1),
        C::Numpad2 => Key::Numpad(2),
        C::Numpad3 => Key::Numpad(3),
        C::Numpad4 => Key::Numpad(4),
        C::Numpad5 => Key::Numpad(5),
        C::Numpad6 => Key::Numpad(6),
        C::Numpad7 => Key::Numpad(7),
        C::Numpad8 => Key::Numpad(8),
        C::Numpad9 => Key::Numpad(9),
        C::NumpadDivide => Key::NumpadDivide,
        C::NumpadMultiply => Key::NumpadMultiply,
        C::NumpadSubtract => Key::NumpadSubtract,
        C::NumpadAdd => Key::NumpadAdd,
        C::NumpadDecimal => Key::NumpadDecimal,
        C::NumpadEnter => Key::NumpadEnter,
        C::NumLock => Key::NumLock,
        C::F1 => Key::Fn(1),
        C::F2 => Key::Fn(2),
        C::F3 => Key::Fn(3),
        C::F4 => Key::Fn(4),
        C::F5 => Key::Fn(5),
        C::F6 => Key::Fn(6),
        C::F7 => Key::Fn(7),
        C::F8 => Key::Fn(8),
        C::F9 => Key::Fn(9),
        C::F10 => Key::Fn(10),
        C::F11 => Key::Fn(11),
        C::F12 => Key::Fn(12),
        C::Space => Key::Space,
        C::ShiftLeft => Key::ShiftLeft,
        C::ShiftRight => Key::ShiftRight,
        C::ControlLeft => Key::ControlLeft,
        C::ControlRight => Key::ControlRight,
        C::AltLeft => Key::AltLeft,
        C::AltRight => Key::AltRight,
        C::SuperLeft => Key::MetaLeft,
        C::SuperRight => Key::MetaRight,
        C::ContextMenu => Key::ContextMenu,
        C::CapsLock => Key::CapsLock,
        C::ArrowUp => Key::Up,
        C::ArrowDown => Key::Down,
        C::ArrowLeft => Key::Left,
        C::ArrowRight => Key::Right,
        C::Insert => Key::Insert,
        C::Delete => Key::Delete,
        C::Home => Key::Home,
        C::End => Key::End,
        C::PageUp => Key::PageUp,
        C::PageDown => Key::PageDown,
        C::PrintScreen => Key::PrintScreen,
        C::ScrollLock => Key::ScrollLock,
        C::Pause => Key::Pause,
        C::BracketLeft => Key::BracketLeft,
        C::BracketRight => Key::BracketRight,
        C::Backslash => Key::Backslash,
        C::Semicolon => Key::Semicolon,
        C::Quote => Key::Quote,
        C::Backquote => Key::Backquote,
        C::Comma => Key::Comma,
        C::Period => Key::Period,
        C::Slash => Key::Slash,
        C::Minus => Key::Minus,
        C::Equal => Key::Equal,
        C::IntlBackslash => Key::IntlBackslash,
        C::Escape => Key::Escape,
        C::Tab => Key::Tab,
        C::Enter => Key::Enter,
        C::Backspace => Key::Backspace,
        _ => return None,
    })
}

/// Captures (relative mode, hidden cursor) or releases the pointer of `w`; returns whether it is
/// captured.
pub fn set_pointer_capture(w: &Window, on: bool) -> bool {
    if on {
        let ok = w
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| w.set_cursor_grab(CursorGrabMode::Confined))
            .is_ok();
        w.set_cursor_visible(!ok);
        ok
    } else {
        let _ = w.set_cursor_grab(CursorGrabMode::None);
        w.set_cursor_visible(true);
        false
    }
}

impl<A: WindowApp> Runner<'_, A> {
    fn set_capture(&mut self, on: bool) {
        let Some(w) = &self.window else { return };
        self.input.pointer_captured = set_pointer_capture(w, on);
    }
}

impl<A: WindowApp> ApplicationHandler for Runner<'_, A> {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(self.cfg.title.clone())
            .with_inner_size(PhysicalSize::new(self.cfg.width, self.cfg.height));
        match el.create_window(attrs) {
            Ok(w) => {
                let w = Arc::new(w);
                self.window = Some(w.clone());
                self.app.init(w.clone());
                let s = w.inner_size();
                self.app.resized(s.width, s.height);
                w.request_redraw();
            }
            Err(e) => {
                eprintln!("sa-platform: cannot create window: {e}");
                el.exit();
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.input.events.push(InputEvent::Close);
                el.exit();
            }
            WindowEvent::Resized(s) => {
                self.input.events.push(InputEvent::Resized {
                    width: s.width,
                    height: s.height,
                });
                self.app.resized(s.width, s.height);
            }
            WindowEvent::Focused(on) => {
                if !on {
                    self.input.release_all();
                    self.set_capture(false);
                }
                self.input.events.push(InputEvent::Focus(on));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let Some(k) = map_key(code) else { return };
                match event.state {
                    ElementState::Pressed
                        if self.cfg.click_to_capture && k == Key::Escape && self.input.pointer_captured =>
                    {
                        self.set_capture(false)
                    }
                    ElementState::Pressed if event.repeat => self.input.key_repeat(k),
                    ElementState::Pressed => self.input.key_down(k),
                    ElementState::Released => self.input.key_up(k),
                }
                if event.state == ElementState::Pressed
                    && let Some(t) = &event.text
                {
                    for c in t.chars() {
                        self.input.events.push(InputEvent::Char(c));
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.events.push(InputEvent::MouseMove {
                    x: position.x as i32,
                    y: position.y as i32,
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let b = match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    winit::event::MouseButton::Back => MouseButton::X1,
                    winit::event::MouseButton::Forward => MouseButton::X2,
                    _ => return,
                };
                match state {
                    ElementState::Pressed => {
                        if self.cfg.click_to_capture && b == MouseButton::Left && !self.input.pointer_captured
                        {
                            self.set_capture(true);
                        }
                        self.input.mouse_down(b);
                    }
                    ElementState::Released => self.input.mouse_up(b),
                }
            }
            WindowEvent::MouseWheel { delta, .. } => self.input.wheel_lines(match delta {
                MouseScrollDelta::LineDelta(_, y) => y,
                MouseScrollDelta::PixelDelta(p) => (p.y / 40.0) as f32,
            }),
            WindowEvent::RedrawRequested => {
                #[cfg(feature = "gamepad")]
                if let Some(p) = &mut self.pads {
                    self.input.pads = p.poll();
                }
                if !self.app.frame(&self.input) {
                    el.exit();
                }
                self.input.end_frame();
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: winit::event::DeviceId, event: DeviceEvent) {
        // DirectInput reports raw relative motion whether or not the cursor is captured; the
        // viewer only wants it while captured
        if let DeviceEvent::MouseMotion { delta } = event
            && (self.input.pointer_captured || !self.cfg.click_to_capture)
        {
            self.input.mouse_delta.0 += delta.0;
            self.input.mouse_delta.1 += delta.1;
        }
    }
}

/// Opens a window and drives `app` until it returns `false` or the window closes.
pub fn run_windowed<A: WindowApp>(cfg: WindowConfig, app: &mut A) -> Result<(), String> {
    let el = EventLoop::new().map_err(|e| e.to_string())?;
    el.set_control_flow(ControlFlow::Poll);
    let mut r = Runner {
        cfg,
        app,
        window: None,
        input: InputState::default(),
        #[cfg(feature = "gamepad")]
        pads: super::gamepad::Pads::new().ok(),
    };
    el.run_app(&mut r).map_err(|e| e.to_string())
}
