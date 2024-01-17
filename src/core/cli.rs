use async_trait::async_trait;
use tracing::{error, info};
use winit::event_loop::EventLoopProxy;

use crate::core::{
    app::*,
    command_queue::*,
    events::CommandEvent,
    state::{is_running, State},
};

pub struct CLIContext;

pub struct CLI {
    pub commands: Vec<Command>,
    pub context: CLIContext,
}

impl CLI {
    pub fn process_cli_command(&mut self, mut cmd: Command) {
        let args = cmd.args.clone().unwrap();
        let vec_args: Vec<&str> = args.split(' ').collect();

        let task = match args.to_ascii_lowercase().as_str() {
            "exit" => CLI::exit(self),
            "load" => CLI::load_dynamic(vec_args[1]),
            _ => CLI::unsupported(args.as_str()),
        };

        cmd.processed = true;
        cmd.task = task;
        cmd.args = Some(args);

        self.commands.push(cmd);
    }

    // TODO: load dyn lib
    fn load_dynamic(_path: &str) -> Option<Task<Vec<CommandEvent>>> {
        None
    }

    fn exit(&self) -> Option<Task<Vec<CommandEvent>>> {
        let cmd = move || {
            let event = CommandEvent::Exit;

            vec![event]
        };

        Some(Box::new(cmd))
    }
}

#[async_trait(?Send)]
impl App for CLI {
    fn init(&mut self, _elp: EventLoopProxy<CommandEvent>) {}

    fn update(&mut self) -> Vec<Command> {
        self.commands.drain(0..self.commands.len()).collect()
    }

    fn process_command(&mut self, cmd: Command, _elp: EventLoopProxy<CommandEvent>) {
        self.process_cli_command(cmd);
    }

    async fn process_event(
        &mut self,
        _event: &winit::event::Event<CommandEvent>,
        _elp: EventLoopProxy<CommandEvent>,
    ) {
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn get_cli_command() -> Command {
    let mut buffer = String::new();

    buffer.clear();

    std::io::stdin()
        .read_line(&mut buffer)
        .expect("Could not read provided command!");

    let command = buffer.trim_end().to_string();

    info!("Command: {}", command);

    let args: Vec<&str> = command.split(' ').collect();

    Command {
        processed: false,
        app: args[0].to_owned(),
        command_type: CommandType::TBD,
        args: Some(args[1..].join(" ")),
        task: None,
    }
}

pub async fn run_cli() {
    while is_running() {
        let next_command = get_cli_command();
        info!("Command: {:?}", next_command);

        let mut state_lock = State::write().await;
        let elp = state_lock.event_loop_proxy.clone().unwrap();

        if let Some(app) = state_lock.apps.get_mut(&next_command.app) {
            app.process_command(next_command, elp);
        } else {
            error!("No app found with name: {}", next_command.app);
        }
    }
}
