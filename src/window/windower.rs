use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tracing::{debug, error, info};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

use crate::{
    core::{
        app::App,
        command_queue::{Command, Task},
        events::CommandEvent,
    },
    prelude::{
        command_queue::CommandType, window_component::WindowContainer, ChangeComponentState,
    },
};

#[derive(Default, Debug, Clone)]
pub struct NewWindowProps {
    pub size: PhysicalSize<u32>,
    pub name: String,
    pub element_id: String,
    // Option<Decorations...icon...etc, etc>
}

pub struct WinID {
    pub name: String,
    pub id: winit::window::WindowId,
}

#[derive(Default)]
pub struct Windower {
    pub windows: HashMap<winit::window::WindowId, Arc<winit::window::Window>>,
    pub window_names: HashMap<winit::window::WindowId, String>,
    pub commands: Vec<Command>,

    pub proxy: Option<EventLoopProxy<CommandEvent>>,
}

impl Windower {
    pub fn process_window_command(&mut self, mut cmd: Command) {
        let args = cmd.args.clone().unwrap();
        let vec_args: Vec<&str> = args.split(' ').collect();

        let task = match vec_args[0].to_ascii_lowercase().as_str() {
            "open" => self.open(vec_args[1..].join(" ")),
            "close" => self.close(vec_args[1..].join(" ")),
            "help" => self.help(),
            _ => Windower::unsupported(args.as_str()),
        };

        cmd.task = task;
        cmd.args = Some(args);
        cmd.processed = true;

        self.commands.push(cmd);
    }

    pub fn open(&self, args: String) -> Option<Task<Vec<CommandEvent>>> {
        let args_vec: Vec<&str> = args.split(' ').collect();
        if args_vec.len() != 3 {
            error!("Windower <open> takes 3 arguments!");
            error!("args: {:?}", args_vec);
            return None;
        }

        let cmd = move || {
            let args_vec: Vec<&str> = args.split(' ').collect();
            let event = CommandEvent::RequestNewWindow(NewWindowProps {
                name: args_vec[0].to_string(),
                size: PhysicalSize {
                    width: args_vec[1].parse::<u32>().unwrap_or(1024),
                    height: args_vec[2].parse::<u32>().unwrap_or(1024),
                },
                element_id: args_vec[0].to_string(),
            });

            let event1 =
                CommandEvent::SignalChange(ChangeComponentState::Window(WindowContainer {
                    width: args_vec[1].parse::<u32>().unwrap_or(1024) as f32,
                    height: args_vec[2].parse::<u32>().unwrap_or(1024) as f32,
                }));

            vec![event, event1]
        };

        Some(Box::new(cmd))
    }

    pub fn close(&mut self, args: String) -> Option<Task<Vec<CommandEvent>>> {
        let args: Vec<&str> = args.split(' ').collect();
        let name = args[0].to_owned();
        if args.len() != 1 {
            error!("Windower <close> takes 1 argument!");
            error!("args: {:?}", args);
            return None;
        }

        let mut windows: Vec<winit::window::WindowId> = vec![];

        for (id, window_name) in &self.window_names {
            if name == *window_name {
                windows.push(*id);
            }
        }

        let mut events: Vec<CommandEvent> = vec![];

        for window in &windows {
            events.push(CommandEvent::OnWindowClosed((*window, name.clone())));
        }

        if !events.is_empty() {
            return Some(Box::new(move || events.clone()));
        }

        if windows.is_empty() {
            error!("Window <{}> not found!", args[0]);
        }
        None
    }

    pub fn help(&self) -> Option<Task<Vec<CommandEvent>>> {
        info!(
            "open <Name> <Width> <Height> -> Opens a window with the specified name and dimensions"
        );
        info!("close <Name> -> closes the specified window");

        None
    }

    pub fn create_window(&mut self, props: NewWindowProps, window: winit::window::Window) {
        let win_id = window.id();

        self.windows.insert(window.id(), Arc::new(window));
        self.window_names.insert(win_id, props.name.clone());

        debug!("Created window {}: {:?}", props.name.clone(), win_id);
        let window = self.windows.get(&win_id).unwrap();

        self.proxy
            .as_ref()
            .unwrap()
            .send_event(CommandEvent::OnWindowCreated(Arc::clone(window)))
            .expect("Failed to send event!");

        #[cfg(target_arch = "wasm32")]
        {
            append_canvas(window, props.size, &props.element_id);
        }
    }
}

#[async_trait(?Send)]
impl App for Windower {
    fn get_name(&self) -> String {
        "Windower".into()
    }
    fn init(&mut self, elp: EventLoopProxy<CommandEvent>) {
        self.proxy = Some(elp.clone());

        let task = self.open("Sandbox 1920 1080".into());

        let cmd = Command {
            processed: true,
            app: "Windower".into(),
            args: None,
            command_type: CommandType::Open,
            task,
        };

        self.commands.push(cmd);
    }

    fn update(&mut self, _delta_time: f32) -> Vec<Command> {
        for (_window_id, window) in &self.windows {
            if window.has_focus() {
                window.request_redraw();
            }

            #[cfg(target_arch = "wasm32")]
            {
                use winit::platform::web::WindowExtWebSys;
                let web_window = web_sys::window().expect("no global `window` exists");
                let document = web_window
                    .document()
                    .expect("should have a document on window");

                let element_id = self.window_names.get(_window_id).unwrap();

                if let Some(outer_el) = document.get_element_by_id(element_id) {
                    if let Some(inner_el) =
                        document.get_element_by_id(format!("{}_canvas", element_id).as_str())
                    {
                        let desired_width = outer_el.client_width() as u32;
                        let desired_height = outer_el.client_height() as u32;

                        if window.inner_size().width != desired_width
                            || window.inner_size().height != desired_height
                        {
                            let canvas_css = format!(
                                "width: {}px; height: {}px; position: absolute",
                                desired_width, desired_height
                            );

                            let js_val = <web_sys::Element as AsRef<wasm_bindgen::JsValue>>::as_ref(
                                &inner_el,
                            );

                            <web_sys::HtmlElement as From<wasm_bindgen::JsValue>>::from(
                                js_val.clone(),
                            )
                            .style()
                            .set_css_text(canvas_css.as_str());

                            window.set_min_inner_size(Some(PhysicalSize {
                                width: desired_width,
                                height: desired_height,
                            }));
                            window.request_redraw();
                        }
                    }
                }
            }
        }
        self.commands.drain(..).collect()
    }

    async fn process_command(&mut self, cmd: Command) {
        self.process_window_command(cmd);
    }

    async fn process_window_event(
        &mut self,
        event: &winit::event::WindowEvent,
        window_id: winit::window::WindowId,
        _delta_time: f32,
    ) {
        if let winit::event::WindowEvent::CloseRequested = event {
            self.windows.remove(&window_id);
            self.window_names.remove(&window_id);
        }
    }

    async fn process_user_event(&mut self, event: &CommandEvent, _delta_time: f32) {
        if let CommandEvent::OnWindowClosed((id, _)) = event {
            self.windows.remove(id);
            self.window_names.remove(id);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(target_arch = "wasm32")]
fn append_canvas(window: &winit::window::Window, size: PhysicalSize<u32>, element_id: &str) {
    use winit::platform::web::WindowExtWebSys;

    window.set_min_inner_size(Some(size));

    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let web_window = web_sys::window().expect("no global `window` exists");
    let document = web_window
        .document()
        .expect("should have a document on window");

    let body = document.body().expect("document should have a body");

    if let Some(canvas_el) = document.get_element_by_id(element_id) {
        let canvas_css = format!(
            "width: {}px; height: {}px; position: absolute",
            canvas_el.client_width(),
            canvas_el.client_height()
        );

        let canvas = window.canvas().unwrap();
        canvas.style().set_css_text(canvas_css.as_str());
        canvas.set_id(format!("{}_canvas", element_id).as_str());
        canvas_el.append_child(&canvas).unwrap();
    } else {
        let canvas_header = document.create_element("h2").unwrap();
        canvas_header.set_text_content(Some(
            format!(
                "Canvas: {:?}! No element with ID: {}",
                window.id(),
                element_id
            )
            .as_str(),
        ));
        body.append_child(&canvas_header).unwrap();

        let canvas_css = format!(
            "width: {}px; height: {}px; position: relative; z-index: 9999",
            size.width, size.height
        );

        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|_doc| {
                let canvas = window.canvas().unwrap();
                canvas.style().set_css_text(canvas_css.as_str());
                body.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }
}
