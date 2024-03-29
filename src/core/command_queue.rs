use crate::core::events::CommandEvent;
use std::{collections::VecDeque, fmt::Debug};

// TODO: Context needs to be defined in the app itself
pub struct Context;

#[cfg(not(target_arch = "wasm32"))]
pub type Task<T> = Box<dyn FnMut() -> T + Send + Sync>;

#[cfg(target_arch = "wasm32")]
pub type Task<T> = Box<dyn FnMut() -> T + Send>;

//#[derive(Reflect)]
#[derive(PartialEq, Eq, Debug)]
pub enum CommandType {
    Exit,
    Help,
    Get,
    Put,
    Querry,
    Open,
    Close,
    Other,
    TBD,
}

// "assetserver get -from_server 127.0.0.1:7878 shaders/shader_challenge.vert"
// "window open NewWindow 1080 720" -> Creates a window and an empty hall and inserts them into Renderer
// "Gallery open NewGallery shader.vert shader.frag" -> Creates a gallery and sets it as the current gallery of the hall
//
// Command has:
// context: the API behind it? How would this be used
// type
// function
// args: Vec<String>
#[allow(dead_code)]
pub struct Command {
    pub app: String,
    pub command_type: CommandType,
    pub processed: bool,

    pub args: Option<String>,
    pub task: Option<Task<Vec<CommandEvent>>>,
}

impl Command {
    pub fn new(
        app: &str,
        command_type: CommandType,
        args: Option<String>,
        task: Option<Task<Vec<CommandEvent>>>,
    ) -> Command {
        Command {
            processed: task.is_some(),
            app: app.to_owned(),
            command_type,
            args,
            task,
        }
    }

    pub fn from_args(
        args: Vec<&str>,
        elp: winit::event_loop::EventLoopProxy<CommandEvent>,
    ) -> Command {
        match args[0] {
            "close" | "exit" => Command::exit(elp),
            _ => Command {
                processed: false,
                app: args[0].to_owned(),
                command_type: CommandType::Other,
                args: Some(args[1..].join(" ")),
                task: None,
            },
        }
    }

    pub fn exit(elp: winit::event_loop::EventLoopProxy<CommandEvent>) -> Command {
        elp.send_event(CommandEvent::Exit).unwrap();
        Command {
            processed: true,
            app: "Main".to_owned(),
            command_type: CommandType::Close,
            args: None,
            task: None,
        }
    }
}

impl Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("App", &self.app)
            .field("Type", &self.command_type)
            .field("args", &self.args)
            .finish()
    }
}

pub trait IntoCommand {
    fn into_command(self) -> Command;
}

impl IntoCommand for Command {
    fn into_command(self) -> Command {
        self
    }
}

#[derive(Default)]
pub struct CommandQueue {
    commands: VecDeque<Command>,
}

impl CommandQueue {
    pub fn new(startup_commands: Vec<impl IntoCommand>) -> CommandQueue {
        let mut commands: Vec<Command> = Vec::with_capacity(startup_commands.len());

        for cmd in startup_commands {
            commands.push(cmd.into_command());
        }

        CommandQueue {
            commands: commands.into(),
        }
    }

    // TODO: Pass an RwLockGuard to all tasks?
    pub fn execute(&mut self, elp: winit::event_loop::EventLoopProxy<CommandEvent>) {
        for _ in 0..self.commands.len() {
            let command = self.commands.pop_front();
            if let Some(command) = command {
                if let Some(mut task) = command.task {
                    let events = task();
                    for event in events {
                        elp.send_event(event).expect("Could not send event T-T");
                    }
                }
            }
        }
    }

    pub fn add_command(&mut self, command: impl IntoCommand) {
        self.commands.push_back(command.into_command());
    }

    pub fn add_commands(&mut self, commands: Vec<Option<Command>>) {
        #[allow(clippy::manual_flatten)]
        for command in commands {
            if let Some(cmd) = command {
                self.commands.push_back(cmd.into_command());
            }
        }
    }
}
