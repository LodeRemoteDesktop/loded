use std::{io::ErrorKind, sync::Mutex};

use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    AttributeSet, EventType, InputEvent, Key, RelativeAxisType,
};

use log::{debug, info, warn};

use tokio::sync::{
    broadcast,
    mpsc::{channel, Receiver, Sender},
};

use crate::Result;

#[derive(thiserror::Error, Debug)]
pub enum InputManagerError {
    #[error("An unknown key {0} was encountered")]
    UnknownKey(String),
    #[error("An internal mutex was poisoned")]
    PoisonedMutex,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum KeyDirection {
    Up = 0,
    Down = 1,
    RepeatingDown = 2,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub key: Key,
    pub direction: KeyDirection,
}

impl From<KeyEvent> for InputEvent {
    fn from(val: KeyEvent) -> Self {
        InputEvent::new(EventType::KEY, val.key.code(), val.direction as i32)
    }
}

impl KeyEvent {
    pub fn from_js_key_name_with_direction(key: &str, direction: KeyDirection) -> Result<Self> {
        let key = match key {
            "KeyA" => Key::KEY_A,
            "KeyB" => Key::KEY_B,
            "KeyC" => Key::KEY_C,
            "KeyD" => Key::KEY_D,
            "KeyE" => Key::KEY_E,
            "KeyF" => Key::KEY_F,
            "KeyG" => Key::KEY_G,
            "KeyH" => Key::KEY_H,
            "KeyI" => Key::KEY_I,
            "KeyJ" => Key::KEY_J,
            "KeyK" => Key::KEY_K,
            "KeyL" => Key::KEY_L,
            "KeyM" => Key::KEY_M,
            "KeyN" => Key::KEY_N,
            "KeyO" => Key::KEY_O,
            "KeyP" => Key::KEY_P,
            "KeyQ" => Key::KEY_Q,
            "KeyR" => Key::KEY_R,
            "KeyS" => Key::KEY_S,
            "KeyT" => Key::KEY_T,
            "KeyU" => Key::KEY_U,
            "KeyV" => Key::KEY_V,
            "KeyW" => Key::KEY_W,
            "KeyX" => Key::KEY_X,
            "KeyY" => Key::KEY_Y,
            "KeyZ" => Key::KEY_Z,
            "CapsLock" => Key::KEY_CAPSLOCK,
            "Escape" => Key::KEY_ESC,
            "Backquote" => Key::KEY_GRAVE,
            "KEY0" => Key::KEY_0,
            "KEY1" => Key::KEY_1,
            "KEY2" => Key::KEY_2,
            "KEY3" => Key::KEY_3,
            "KEY4" => Key::KEY_4,
            "KEY5" => Key::KEY_5,
            "KEY6" => Key::KEY_6,
            "KEY7" => Key::KEY_7,
            "KEY8" => Key::KEY_8,
            "KEY9" => Key::KEY_9,
            "Minus" => Key::KEY_MINUS,
            "Equal" => Key::KEY_EQUAL,
            "Backspace" => Key::KEY_BACKSPACE,
            "BracketLeft" => Key::KEY_LEFTBRACE,
            "BracketRight" => Key::KEY_RIGHTBRACE,
            "Backslash" => Key::KEY_BACKSLASH,
            "Tab" => Key::KEY_TAB,
            "ShiftLeft" => Key::KEY_LEFTSHIFT,
            "ControlLeft" => Key::KEY_LEFTCTRL,
            "AltLeft" => Key::KEY_LEFTALT,
            "Space" => Key::KEY_SPACE,
            "AltRight" => Key::KEY_RIGHTALT,
            "ControlRight" => Key::KEY_RIGHTCTRL,
            "ShiftRight" => Key::KEY_RIGHTSHIFT,
            "Enter" => Key::KEY_ENTER,
            k => return Err(InputManagerError::UnknownKey(k.to_string()).into()),
        };
        Ok(Self { key, direction })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MouseMoveEvent {
    pub x: i32,
    pub y: i32,
    pub wheel: i32,
    _padding: i32,
}

impl MouseMoveEvent {
    pub fn get_input_events(&self) -> Vec<InputEvent> {
        let mut out = Vec::new();
        if self.x != 0 {
            out.push(InputEvent::new(
                EventType::RELATIVE,
                RelativeAxisType::REL_X.0,
                self.x,
            ));
        }

        if self.y != 0 {
            out.push(InputEvent::new(
                EventType::RELATIVE,
                RelativeAxisType::REL_Y.0,
                self.y,
            ));
        }

        if self.wheel != 0 {
            out.push(InputEvent::new(
                EventType::RELATIVE,
                RelativeAxisType::REL_HWHEEL_HI_RES.0,
                self.wheel,
            ))
        }

        out
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MouseButtonEvent {
    pub key: Key,
    pub direction: KeyDirection,
}

impl From<MouseButtonEvent> for InputEvent {
    fn from(value: MouseButtonEvent) -> Self {
        InputEvent::new(EventType::KEY, value.key.code(), value.direction as i32)
    }
}

#[derive(Debug)]
pub enum InputManagerEvent {
    Keyboard(Vec<KeyEvent>),
    Mouse(Option<Vec<MouseMoveEvent>>, Option<Vec<MouseButtonEvent>>),
}

/// Struct that receives virtual key events and forwards them to the operating system
pub struct InputManager {
    keyboard: Mutex<VirtualDevice>,
    mouse: Mutex<VirtualDevice>,
    rx: Mutex<Option<Receiver<InputManagerEvent>>>,
    running: Mutex<Option<broadcast::Receiver<()>>>,
}

impl InputManager {
    pub fn new(die_handle: broadcast::Receiver<()>) -> Result<(Self, Sender<InputManagerEvent>)> {
        let mut keys = AttributeSet::<Key>::new();
        keys.insert(Key::KEY_A);
        keys.insert(Key::KEY_B);
        keys.insert(Key::KEY_C);
        keys.insert(Key::KEY_D);
        keys.insert(Key::KEY_E);
        keys.insert(Key::KEY_F);
        keys.insert(Key::KEY_G);
        keys.insert(Key::KEY_H);
        keys.insert(Key::KEY_I);
        keys.insert(Key::KEY_J);
        keys.insert(Key::KEY_K);
        keys.insert(Key::KEY_L);
        keys.insert(Key::KEY_M);
        keys.insert(Key::KEY_N);
        keys.insert(Key::KEY_O);
        keys.insert(Key::KEY_P);
        keys.insert(Key::KEY_Q);
        keys.insert(Key::KEY_R);
        keys.insert(Key::KEY_S);
        keys.insert(Key::KEY_T);
        keys.insert(Key::KEY_U);
        keys.insert(Key::KEY_V);
        keys.insert(Key::KEY_W);
        keys.insert(Key::KEY_X);
        keys.insert(Key::KEY_Y);
        keys.insert(Key::KEY_Z);
        keys.insert(Key::KEY_CAPSLOCK);
        keys.insert(Key::KEY_ESC);
        keys.insert(Key::KEY_GRAVE);
        keys.insert(Key::KEY_0);
        keys.insert(Key::KEY_1);
        keys.insert(Key::KEY_2);
        keys.insert(Key::KEY_3);
        keys.insert(Key::KEY_4);
        keys.insert(Key::KEY_5);
        keys.insert(Key::KEY_6);
        keys.insert(Key::KEY_7);
        keys.insert(Key::KEY_8);
        keys.insert(Key::KEY_9);
        keys.insert(Key::KEY_MINUS);
        keys.insert(Key::KEY_EQUAL);
        keys.insert(Key::KEY_BACKSPACE);
        keys.insert(Key::KEY_LEFTBRACE);
        keys.insert(Key::KEY_RIGHTBRACE);
        keys.insert(Key::KEY_BACKSLASH);
        keys.insert(Key::KEY_TAB);
        keys.insert(Key::KEY_LEFTSHIFT);
        keys.insert(Key::KEY_LEFTCTRL);
        keys.insert(Key::KEY_LEFTALT);
        keys.insert(Key::KEY_SPACE);
        keys.insert(Key::KEY_RIGHTALT);
        keys.insert(Key::KEY_RIGHTCTRL);
        keys.insert(Key::KEY_RIGHTSHIFT);
        keys.insert(Key::KEY_ENTER);

        debug!("Made keys: {:#?}", keys);

        let keyboard = VirtualDeviceBuilder::new()?
            .name("rdesktopd Virtal Keyboard")
            .with_keys(&keys)?
            .build()?;

        debug!("Made keyboard");

        let mut axis = AttributeSet::<RelativeAxisType>::new();
        axis.insert(RelativeAxisType::REL_X);
        axis.insert(RelativeAxisType::REL_Y);
        // axis.insert(RelativeAxisType::REL_Z);
        axis.insert(RelativeAxisType::REL_WHEEL);

        debug!("Made axis: {:#?}", axis);

        let mut buttons = AttributeSet::<Key>::new();
        buttons.insert(Key::BTN_LEFT);
        buttons.insert(Key::BTN_RIGHT);
        buttons.insert(Key::BTN_MIDDLE);

        debug!("Made buttons: {:#?}", buttons);

        let mouse = VirtualDeviceBuilder::new()?
            .name("rdesktopd Virtual Mouse")
            .with_relative_axes(&axis)?
            .with_keys(&buttons)?
            .build()?;

        debug!("Made mouse");

        let (tx, rx) = channel(100);

        info!("Intialized InputManager");

        Ok((
            Self {
                keyboard: Mutex::new(keyboard),
                mouse: Mutex::new(mouse),
                rx: Mutex::new(Some(rx)),
                running: Mutex::new(Some(die_handle)),
            },
            tx,
        ))
    }

    pub async fn listen(&self) -> Result<()> {
        let mut rx = self
            .rx
            .lock()
            .map_err(|_| InputManagerError::PoisonedMutex)?
            .take()
            .expect("Listen must not be called more than once");
        let mut ds = self
            .running
            .lock()
            .map_err(|_| InputManagerError::PoisonedMutex)?
            .take()
            .expect("Listen must not be called more than once");

        loop {
            if ds.try_recv().is_ok() {
                break;
            }

            if let Some(msg) = rx.recv().await {
                match msg {
                    InputManagerEvent::Keyboard(key_evt) => {
                        match self.send_keyboard_events(key_evt.as_slice()) {
                            Ok(_) => {}
                            Err(e) => warn!("Failed to write keyboard events: {e}"),
                        }
                    }
                    InputManagerEvent::Mouse(move_evt, button_evt) => {
                        let move_evts = if let Some(events) = move_evt.as_ref() {
                            events.as_slice()
                        } else {
                            &[]
                        };

                        let btn_evts = if let Some(events) = button_evt.as_ref() {
                            events.as_slice()
                        } else {
                            &[]
                        };

                        match self.send_mouse_events(move_evts, btn_evts) {
                            Ok(_) => {}
                            Err(e) => warn!("Failed to write mouse events: {e}"),
                        }
                    }
                };
            } else {
                break;
            }
        }

        Ok(())
    }

    pub fn send_keyboard_events(&self, key_event: &[KeyEvent]) -> std::io::Result<()> {
        let events = key_event
            .iter()
            .copied()
            .map(|i| i.into())
            .collect::<Vec<InputEvent>>();

        let mut keyboard = self
            .keyboard
            .lock()
            .map_err(|_| std::io::Error::new(ErrorKind::Other, InputManagerError::PoisonedMutex))?;
        keyboard.emit(&events)
    }

    pub fn send_mouse_events(
        &self,
        move_event: &[MouseMoveEvent],
        click_events: &[MouseButtonEvent],
    ) -> std::io::Result<()> {
        let mut events = move_event
            .iter()
            .flat_map(|mme| mme.get_input_events())
            .collect::<Vec<InputEvent>>();
        events.extend(click_events.iter().copied().map(InputEvent::from));

        let mut mouse = self
            .mouse
            .lock()
            .map_err(|_| std::io::Error::new(ErrorKind::Other, InputManagerError::PoisonedMutex))?;
        mouse.emit(&events)
    }
}
