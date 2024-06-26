use std::{collections::HashMap, sync::atomic::AtomicBool};

use async_std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use once_cell::sync::Lazy;
use tracing::{error, info, warn};
use winit::event_loop::EventLoopProxy;

use crate::prelude::windower::NewWindowProps;
#[allow(unused_imports)]
use crate::{
    assets::{asset_cmd::AssetCommand, asset_server::AssetServer},
    core::{
        app::App,
        cli::run_cli,
        command_queue::{Command, CommandQueue, CommandType},
        default_apps::default_apps,
        events::CommandEvent,
    },
    window::windower::Windower,
};

static mut GLOBAL_STATE: Lazy<RwLock<State>> = Lazy::new(Default::default);
static mut RUNNING: AtomicBool = AtomicBool::new(true);
pub static mut ENGINE_INIT: AtomicBool = AtomicBool::new(false);

pub fn initialized() -> bool {
    unsafe { ENGINE_INIT.load(std::sync::atomic::Ordering::Acquire) }
}

pub fn finish_init() {
    unsafe {
        ENGINE_INIT.store(true, std::sync::atomic::Ordering::Release);
    }
}

pub fn is_running() -> bool {
    unsafe { RUNNING.load(std::sync::atomic::Ordering::Relaxed) }
}

pub fn terminate() {
    unsafe {
        RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Default)]
pub struct State {
    pub apps: HashMap<String, Box<dyn App>>,
    pub command_queue: CommandQueue,
    pub event_loop_proxy: Option<winit::event_loop::EventLoopProxy<CommandEvent>>,
}

impl State {
    async fn init() -> winit::event_loop::EventLoop<CommandEvent> {
        init_trace();

        let event_loop = winit::event_loop::EventLoopBuilder::<CommandEvent>::with_user_event()
            .build()
            .unwrap();

        let event_loop_proxy = event_loop.create_proxy();

        // Scoped to make sure the lock is dropped
        {
            let mut state_lock = State::write().await;
            state_lock.event_loop_proxy = Some(event_loop_proxy.clone());
        }

        let default_apps = default_apps();

        for app in default_apps {
            State::insert_app(&app.0, app.1).await;
        }

        {
            let mut state_lock = State::write().await;
            let apps = &mut state_lock.apps;

            for app in apps.values_mut() {
                app.init(event_loop_proxy.clone())
            }
        }

        State::update(0.0).await;

        event_loop
    }

    async fn update(delta_time: f32) {
        let mut frame_commands: Vec<Option<Command>> = vec![];
        {
            let apps = &mut State::write().await.apps;

            for app in apps.values_mut() {
                let cmds = app.update(delta_time);

                for cmd in cmds {
                    frame_commands.push(Some(cmd));
                }
            }
        }

        {
            let mut state_lock = State::write().await;

            for command in &mut frame_commands {
                if !command.as_ref().unwrap().processed {
                    let cmd = command.take();
                    if let Some(app) = state_lock.apps.get_mut(&cmd.as_ref().unwrap().app) {
                        app.process_command(cmd.unwrap()).await;
                    } else {
                        error!(
                            "No app found with name: {} to process: \"{}\"",
                            &cmd.as_ref().unwrap().app,
                            &cmd.as_ref().unwrap().args.as_ref().unwrap()
                        );
                    }
                }
            }
        }

        {
            State::write()
                .await
                .command_queue
                .add_commands(frame_commands);
        }

        {
            let elp: EventLoopProxy<CommandEvent>;
            {
                elp = State::get_proxy().await;
            }
            let mut state = State::write().await;
            state.command_queue.execute(elp);
        }
    }

    pub async fn process_window_events(
        event: winit::event::WindowEvent,
        id: winit::window::WindowId,
        delta_time: f32,
    ) {
        let apps = &mut State::write().await.apps;

        for app in apps.values_mut() {
            app.process_window_event(&event, id, delta_time).await;
        }
    }

    pub async fn process_user_events(event: CommandEvent, delta_time: f32) {
        let apps = &mut State::write().await.apps;

        for app in apps.values_mut() {
            app.process_user_event(&event, delta_time).await;
        }
    }

    pub async fn process_device_events(
        event: winit::event::DeviceEvent,
        device_id: winit::event::DeviceId,
        delta_time: f32,
    ) {
        let apps = &mut State::write().await.apps;

        for app in apps.values_mut() {
            app.process_device_event(&event, device_id, delta_time)
                .await;
        }
    }

    pub async fn on_new_window_requested(props: NewWindowProps, window: winit::window::Window) {
        let mut state_lock = State::write().await;

        let windower = state_lock
            .apps
            .get_mut("windower")
            .unwrap()
            .as_any_mut()
            .downcast_mut::<Windower>()
            .unwrap();

        windower.create_window(props, window);
    }

    pub async fn get_proxy() -> winit::event_loop::EventLoopProxy<CommandEvent> {
        State::read().await.event_loop_proxy.clone().unwrap()
    }

    pub async fn read() -> RwLockReadGuard<'static, State> {
        unsafe { GLOBAL_STATE.read().await }
    }

    pub async fn write() -> RwLockWriteGuard<'static, State> {
        unsafe { GLOBAL_STATE.write().await }
    }

    pub async fn insert_app(app_name: &str, app: Box<dyn App>) {
        let app_name = app_name.to_ascii_lowercase();
        if !State::read().await.apps.contains_key(app_name.as_str()) {
            State::write().await.apps.insert(app_name.to_owned(), app);
        } else {
            warn!("State already contains app {app_name}!");
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub async fn run() {
    let event_loop = State::init().await;

    info!("Initialzied State!");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let builder = std::thread::Builder::new().name("CLI".into());

        let runtime_cli = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(8)
            .build()
            .unwrap();

        builder
            .spawn(move || {
                runtime_cli.block_on(run_cli());
            })
            .unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(16)
        .build()
        .unwrap();

    let mut current_time = web_time::Instant::now();

    event_loop
        .run(move |event, elwt: &winit::event_loop::EventLoopWindowTarget<CommandEvent>| {
            if !is_running() {
                elwt.exit()
            }

            // Calculate frame time (delta time)
            let new_time = web_time::Instant::now();
            let frame_time = (new_time - current_time).as_nanos();
            #[allow(unused_mut)]    
            let mut delta_time = frame_time as f32 * 0.000000001;
            current_time = new_time;

            #[cfg(target_arch = "wasm32")]
            {
                if delta_time < 0.0001 {
                    delta_time = 0.0001;
                }
            }

            //info!("{frame_time}ms");

            elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);

            match event {
                winit::event::Event::UserEvent(event) => {
                    cfg_if::cfg_if! {
                        if #[cfg(not(target_arch = "wasm32"))] {
                            runtime.block_on(State::process_user_events(event.clone(), delta_time));
                        }
                        else {
                            wasm_bindgen_futures::spawn_local(State::process_user_events(event.clone(), delta_time));
                        }
                    }
                    match event {
                        CommandEvent::RequestNewWindow(props) => {
                            let window = winit::window::WindowBuilder::new()
                                .with_inner_size(winit::dpi::Size::Physical(props.size))
                                .with_title(props.name.clone())
                                .build(elwt)
                                .expect("Could not create new window T-T");

                            cfg_if::cfg_if! {
                                if #[cfg(not(target_arch = "wasm32"))] {
                                    runtime.block_on(State::on_new_window_requested(props, window));
                                }
                                else {
                                    wasm_bindgen_futures::spawn_local(State::on_new_window_requested(props, window));
                                }
                            }
                        }
                        CommandEvent::Exit => {
                            terminate();
                        }
                        _ => {}
                    }
                }
                winit::event::Event::WindowEvent { window_id, event } => {
                    cfg_if::cfg_if! {
                        if #[cfg(not(target_arch = "wasm32"))] {
                            runtime.block_on(State::process_window_events(event.clone(), window_id, delta_time));
                        }
                        else {
                            wasm_bindgen_futures::spawn_local(State::process_window_events(event.clone(), window_id, delta_time));
                        }
                    }
                }
                winit::event::Event::DeviceEvent { device_id, event } => {
                    cfg_if::cfg_if! {
                        if #[cfg(not(target_arch = "wasm32"))] {
                            runtime.block_on(State::process_device_events(event.clone(), device_id, delta_time));
                        }
                        else {
                            wasm_bindgen_futures::spawn_local(State::process_device_events(event.clone(), device_id, delta_time));
                        }
                    }
                }
                _ => {}
            }
            cfg_if::cfg_if! {
                if #[cfg(not(target_arch = "wasm32"))] {
                    runtime.block_on(State::update(delta_time));
                }
                else {
                    wasm_bindgen_futures::spawn_local(State::update(delta_time));
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
        .without_time()
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Could not set default trace subscriver!");
}
