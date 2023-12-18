use tracing::{error, info};
use winit::event_loop::EventLoopWindowTarget;

use crate::core::{app::*, command_queue::*, events::CommandEvent, state::State};

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

        cmd.task = task;
        cmd.args = Some(args);

        self.commands.push(cmd);
    }

    // TODO: load dyn lib
    fn load_dynamic(_path: &str) -> Option<Task<CommandEvent>> {
        None
    }

    fn exit(&self) -> Option<Task<CommandEvent>> {
        let cmd = move || {
            let event = CommandEvent::Exit;

            info!("{event:?}");

            event
        };

        Some(Box::new(cmd))
    }

    fn unsupported(args: &str) -> Option<Task<CommandEvent>> {
        error!("Unsupported arguments {args}");
        None
    }
}

impl App for CLI {
    fn init(&mut self, mut init_commands: Vec<Command>) {
        self.commands.append(&mut init_commands);
    }

    fn queue_commands(&mut self) -> Vec<Command> {
        self.commands.drain(0..self.commands.len()).collect()
    }

    fn process_command(&mut self, cmd: Command) {
        self.process_cli_command(cmd);
    }

    fn process_event(
        &mut self,
        event: &winit::event::Event<CommandEvent>,
        elwt: &EventLoopWindowTarget<CommandEvent>,
    ) {
        if let winit::event::Event::UserEvent(CommandEvent::Exit) = event {
            elwt.exit()
        }
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

    // match args[0].to_ascii_lowercase().as_str() {
    //     "assetserver" => match args[1].to_ascii_lowercase().as_str() {
    //         "get" => AssetCommand::new(CommandType::Get, args[2..].join(" "), elp).into_command(),
    //         _ => AssetCommand::new(CommandType::Other, args[2..].join(" "), elp).into_command(),
    //     },
    //     "window" => match args[1].to_ascii_lowercase().as_str() {
    //         "open" => WindowCommand::new(CommandType::Open, args[2..].join(" ")).into_command(),
    //         _ => WindowCommand::new(CommandType::Other, args[2..].join(" ")).into_command(),
    //     },
    //     _ => Command::from_args(args, elp),
    // }
    Command {
        app: args[0].to_owned(),
        command_type: CommandType::TBD,
        args: Some(args[1..].join(" ")),
        task: None,
    }
}

pub fn run_cli() {
    while State::read().running {
        let next_command = get_cli_command();
        info!("Command: {:?}", next_command);

        let mut state_lock = State::write();

        if let Some(app) = state_lock.apps.get_mut(&next_command.app) {
            app.process_command(next_command);
        }
    }
}
