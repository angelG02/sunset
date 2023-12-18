use std::{
    collections::HashMap,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use once_cell::sync::Lazy;
use tracing::{info, warn};
use winit::event_loop::EventLoopProxy;

use crate::{
    core::{
        app::App,
        cli::run_cli,
        command_queue::{Command, CommandQueue},
        default_apps::default_apps,
        events::CommandEvent,
    },
    window::windower::Windower,
};

static mut GLOBAL_STATE: Lazy<RwLock<State>> = Lazy::new(Default::default);

pub struct State {
    pub running: bool,
    pub apps: HashMap<String, Box<dyn App>>,
    pub command_queue: CommandQueue,
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

        let default_apps = default_apps();

        for app in default_apps {
            State::insert_app(&app.0, app.1);
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
            let elp: EventLoopProxy<CommandEvent>;
            {
                elp = State::get_proxy();
            }
            let mut state = State::write();
            state.command_queue.execute(elp);
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
            event_loop_proxy: None,
            command_queue: CommandQueue::default(),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    let event_loop = State::init();

    info!("Initialzied State!");

    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
        run_cli();
    });

    event_loop
        .run(move |event, elwt| {
            if !State::read().running {
                elwt.exit()
            }
            {
                State::update();
            }
            elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);

            let elp: EventLoopProxy<CommandEvent>;
            {
                elp = State::get_proxy();
            }
            {
                let apps = &mut State::write().apps;

                for app in apps.values_mut() {
                    pollster::block_on(app.process_event(&event, elp.clone()));
                }
            }
            if let winit::event::Event::UserEvent(event) = event {
                match event {
                    CommandEvent::OpenWindow(props) => {
                        let mut state_lock = State::write();

                        let windower = state_lock
                            .apps
                            .get_mut("windower")
                            .unwrap()
                            .as_any_mut()
                            .downcast_mut::<Windower>()
                            .unwrap();

                        windower.create_window(props, elp, elwt);
                    }
                    CommandEvent::Exit => elwt.exit(),
                    _ => {}
                }
            }
        })
        .unwrap();
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
        .with_thread_ids(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Could not set default trace subscriver!");
}
