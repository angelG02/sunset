use std::{
    collections::HashMap,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use once_cell::sync::Lazy;
use tracing::warn;

use crate::core::{
    app::App,
    cli::run_cli,
    command_queue::{Command, CommandQueue},
    events::CommandEvent,
};

static mut GLOBAL_STATE: Lazy<RwLock<State>> = Lazy::new(Default::default);

pub struct State {
    pub running: bool,
    pub apps: HashMap<String, Box<dyn App>>,
    pub command_queue: CommandQueue,
    pub windows: HashMap<winit::window::WindowId, winit::window::Window>,
    pub event_loop_proxy: Option<winit::event_loop::EventLoopProxy<CommandEvent>>,
}

impl State {
    fn init() -> winit::event_loop::EventLoop<CommandEvent> {
        init_trace();
        let event_loop = winit::event_loop::EventLoopBuilder::<CommandEvent>::with_user_event()
            .build()
            .unwrap();
        let event_loop_proxy = event_loop.create_proxy();

        unsafe {
            let mut state_lock = GLOBAL_STATE.write().unwrap();
            state_lock.event_loop_proxy = Some(event_loop_proxy);
        }

        event_loop
    }

    fn update() {
        let mut frame_commands: Vec<Command> = vec![];
        {
            let apps = &mut State::write().apps;

            for app in apps.values_mut() {
                frame_commands.append(&mut app.queue_commands());
            }
        }

        {
            State::write().command_queue.add_commands(frame_commands);
        }
        {
            State::write().command_queue.execute();
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
    pub fn run() {
        let _event_loop = State::init();

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            run_cli();
        });

        loop {
            State::update();
        }
    }

    pub fn get_proxy() -> winit::event_loop::EventLoopProxy<CommandEvent> {
        State::read().event_loop_proxy.clone().unwrap()
    }

    pub fn read() -> RwLockReadGuard<'static, State> {
        unsafe { GLOBAL_STATE.read().expect("Cry about it :P") }
    }

    pub fn write() -> RwLockWriteGuard<'static, State> {
        unsafe { GLOBAL_STATE.write().expect("Cry about it :P") }
    }

    pub fn insert_app(app_name: &str, app: Box<dyn App>) {
        let app_name = app_name.to_ascii_lowercase();
        if !State::read().apps.contains_key(app_name.as_str()) {
            State::write().apps.insert(app_name.to_owned(), app);
        } else {
            warn!("State already contains app {app_name}!");
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            running: true,
            apps: HashMap::new(),
            windows: HashMap::new(),
            event_loop_proxy: None,
            command_queue: CommandQueue::default(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
fn init_trace() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default_with_config(tracing_wasm::WASMLayerConfig::default());
}

#[cfg(not(target_arch = "wasm32"))]
fn init_trace() {
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    // Trace initialization
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_file(false)
        .with_line_number(true)
        .without_time()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Could not set default trace subscriver!");
}
