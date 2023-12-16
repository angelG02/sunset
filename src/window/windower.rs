use std::collections::HashMap;

use tracing::{error, info};
use winit::{dpi::PhysicalSize, event_loop::EventLoopWindowTarget};

use crate::core::{
    app::App,
    command_queue::{Command, Task},
    events::{CommandEvent, NewWindowProps},
};

pub struct WinID {
    pub name: String,
    pub id: winit::window::WindowId,
}

#[derive(Default)]
pub struct Windower {
    pub windows: HashMap<winit::window::WindowId, winit::window::Window>,
    pub window_ids: HashMap<String, winit::window::WindowId>,
    pub commands: Vec<Command>,
}

impl Windower {
    pub fn process_window_command(&mut self, mut cmd: Command) {
        let args = cmd.args.clone().unwrap();
        let vec_args: Vec<&str> = args.split(' ').collect();

        let task = match vec_args[0].to_ascii_lowercase().as_str() {
            "open" => self.open(vec_args[1..].join(" ")),
            "close" => self.close(vec_args[1..].join(" ")),
            "help" => self.help(),
            _ => self.unsupported(args.as_str()),
        };

        cmd.task = task;
        cmd.args = Some(args);

        self.commands.push(cmd);
    }

    pub fn open(&self, args: String) -> Option<Task<CommandEvent>> {
        let args_vec: Vec<&str> = args.split(' ').collect();
        if args_vec.len() != 3 {
            error!("Windower <open> takes 3 arguments!");
            error!("args: {:?}", args_vec);
            return None;
        }

        if self.window_ids.get(args_vec[0]).is_some() {
            error!("Window <{}> already exists!", args_vec[0]);
            return None;
        }

        let cmd = move || {
            let args_vec: Vec<&str> = args.split(' ').collect();
            let event = CommandEvent::OpenWindow(NewWindowProps {
                name: args_vec[0].to_string(),
                size: PhysicalSize {
                    width: args_vec[1].parse::<u32>().unwrap_or(1024),
                    height: args_vec[2].parse::<u32>().unwrap_or(1024),
                },
            });

            info!("{event:?}");

            event
        };

        Some(Box::new(cmd))
    }

    pub fn close(&mut self, args: String) -> Option<Task<CommandEvent>> {
        let args: Vec<&str> = args.split(' ').collect();
        if args.len() != 1 {
            error!("Windower <close> takes 1 argument!");
            error!("args: {:?}", args);
            return None;
        }

        if let Some(id) = self.window_ids.get(args[0]) {
            self.windows.remove(id);
            None
        } else {
            error!("Window <{}> not found!", args[0]);
            None
        }
    }

    pub fn help(&self) -> Option<Task<CommandEvent>> {
        info!(
            "open <Name> <Width> <Height> -> Opens a window with the specified name and dimensions"
        );
        info!("close <Name> -> closes the specified window");

        None
    }

    pub fn unsupported(&self, args: &str) -> Option<Task<CommandEvent>> {
        error!("Unsupported arguments {args}");
        info!("type help for supported commands");
        None
    }
}

impl App for Windower {
    fn init(&mut self, mut init_commands: Vec<Command>) {
        self.commands.append(&mut init_commands);
    }

    fn queue_commands(&mut self /*schedule: Schedule, */) -> Vec<Command> {
        self.commands.drain(..).collect()
    }

    fn process_command(&mut self, cmd: Command) {
        self.process_window_command(cmd);
    }

    fn process_event(&mut self, event: &CommandEvent, elwt: &EventLoopWindowTarget<CommandEvent>) {
        if let CommandEvent::OpenWindow(props) = event {
            let window = winit::window::WindowBuilder::new()
                .with_inner_size(winit::dpi::Size::Physical(props.size))
                .with_title(props.name.clone())
                .build(elwt)
                .expect("Could not create new window T-T");

            let win_id = window.id();

            self.windows.insert(window.id(), window);
            self.window_ids.insert(props.name.clone(), win_id);

            #[cfg(target_arch = "wasm32")]
            {
                let window = self.windows.get(&win_id).unwrap();
                append_canvas(window);
            }
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
fn append_canvas(window: &winit::window::Window) {
    use wasm_bindgen::prelude::*;
    use winit::platform::web::WindowExtWebSys;

    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let web_window = web_sys::window().expect("no global `window` exists");
    let document = web_window
        .document()
        .expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    let canvas_header = document.create_element("h2").unwrap();
    canvas_header.set_text_content(Some(format!("Canvas: {:?}", window.id()).as_str()));
    body.append_child(&canvas_header).unwrap();

    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let canvas = web_sys::Element::from(window.canvas().unwrap());
            body.append_child(&canvas).ok()?;
            Some(())
        })
        .expect("Couldn't append canvas to document body.");
}

// pub struct WindowCommand {
//     pub command_type: CommandType,
//     pub args: String,
//     pub task: Option<Task<CommandEvent>>,
// }

// impl IntoCommand for WindowCommand {
//     fn into_command(self) -> Command {
//         Command {
//             app: "Windower".into(),
//             command_type: self.command_type,
//             args: Some(self.args),
//             task: self.task,
//         }
//     }
// }
