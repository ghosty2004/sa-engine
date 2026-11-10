//! winit window, event loop and input translation.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use crate::input::{InputState, Key, MouseButton};

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
}

struct Runner<'a, A: WindowApp> {
    cfg: WindowConfig,
    app: &'a mut A,
    window: Option<Arc<Window>>,
    input: InputState,
}

fn map_key(k: KeyCode) -> Option<Key> {
    Some(match k {
        KeyCode::KeyW => Key::W,
        KeyCode::KeyA => Key::A,
        KeyCode::KeyS => Key::S,
        KeyCode::KeyD => Key::D,
        KeyCode::KeyQ => Key::Q,
        KeyCode::KeyE => Key::E,
        KeyCode::KeyR => Key::R,
        KeyCode::KeyF => Key::F,
        KeyCode::KeyT => Key::T,
        KeyCode::KeyG => Key::G,
        KeyCode::Space => Key::Space,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Key::ShiftLeft,
        KeyCode::ControlLeft | KeyCode::ControlRight => Key::ControlLeft,
        KeyCode::AltLeft | KeyCode::AltRight => Key::AltLeft,
        KeyCode::ArrowUp => Key::Up,
        KeyCode::ArrowDown => Key::Down,
        KeyCode::ArrowLeft => Key::Left,
        KeyCode::ArrowRight => Key::Right,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::BracketLeft => Key::BracketLeft,
        KeyCode::BracketRight => Key::BracketRight,
        KeyCode::Comma => Key::Comma,
        KeyCode::Period => Key::Period,
        KeyCode::Minus | KeyCode::NumpadSubtract => Key::Minus,
        KeyCode::Equal | KeyCode::NumpadAdd => Key::Equal,
        KeyCode::Digit0 => Key::Digit(0),
        KeyCode::Digit1 => Key::Digit(1),
        KeyCode::Digit2 => Key::Digit(2),
        KeyCode::Digit3 => Key::Digit(3),
        KeyCode::Digit4 => Key::Digit(4),
        KeyCode::Digit5 => Key::Digit(5),
        KeyCode::Digit6 => Key::Digit(6),
        KeyCode::Digit7 => Key::Digit(7),
        KeyCode::Digit8 => Key::Digit(8),
        KeyCode::Digit9 => Key::Digit(9),
        KeyCode::F1 => Key::Fn(1),
        KeyCode::F2 => Key::Fn(2),
        KeyCode::F3 => Key::Fn(3),
        KeyCode::F4 => Key::Fn(4),
        KeyCode::Escape => Key::Escape,
        KeyCode::Tab => Key::Tab,
        KeyCode::Enter => Key::Enter,
        _ => return None,
    })
}

impl<A: WindowApp> Runner<'_, A> {
    fn set_capture(&mut self, on: bool) {
        let Some(w) = &self.window else { return };
        if on {
            let ok = w
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| w.set_cursor_grab(CursorGrabMode::Confined))
                .is_ok();
            w.set_cursor_visible(!ok);
            self.input.pointer_captured = ok;
        } else {
            let _ = w.set_cursor_grab(CursorGrabMode::None);
            w.set_cursor_visible(true);
            self.input.pointer_captured = false;
        }
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
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => self.app.resized(s.width, s.height),
            WindowEvent::Focused(false) => {
                self.input.held.clear();
                self.set_capture(false);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                let Some(k) = map_key(code) else { return };
                match event.state {
                    ElementState::Pressed if k == Key::Escape && self.input.pointer_captured => self.set_capture(false),
                    ElementState::Pressed if !event.repeat => self.input.key_down(k),
                    ElementState::Pressed => {}
                    ElementState::Released => self.input.key_up(k),
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let b = match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    _ => return,
                };
                match state {
                    ElementState::Pressed => {
                        if b == MouseButton::Left && !self.input.pointer_captured {
                            self.set_capture(true);
                        }
                        self.input.mouse_held.insert(b);
                    }
                    ElementState::Released => {
                        self.input.mouse_held.remove(&b);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.wheel += match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y / 40.0) as f32,
                };
            }
            WindowEvent::RedrawRequested => {
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
        if let DeviceEvent::MouseMotion { delta } = event
            && self.input.pointer_captured
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
    let mut r = Runner { cfg, app, window: None, input: InputState::default() };
    el.run_app(&mut r).map_err(|e| e.to_string())
}
