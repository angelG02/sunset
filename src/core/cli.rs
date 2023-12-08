use tracing::{error, info};

use crate::core::{app::*, command_queue::*, state::State};

pub struct CLI {
    pub command_queue: CommandQueue,
    pub context: Context,
}

impl CLI {
    pub fn process_cli_command(&mut self, mut cmd: Command) {
        let args = cmd.args.clone().unwrap();
        let vec_args: Vec<&str> = args.split(' ').collect();

        let task = match args.to_ascii_lowercase().as_str() {
            "exit" => CLI::exit(),
            "load" => CLI::load_dynamic(vec_args[1]),
            _ => CLI::unsupported(args.as_str()),
        };

        cmd.task = task;
        cmd.args = Some(args);

        self.command_queue.add_command(cmd);
    }

    // TODO: load dyn lib
    fn load_dynamic(_path: &str) -> Option<Task<CommandEvent>> {
        None
    }

    fn exit() -> Option<Task<CommandEvent>> {
        info!("Exiting...");
        //State::write().running = false;
        None
    }

    fn unsupported(args: &str) -> Option<Task<CommandEvent>> {
        error!("Unsupported arguments {args}");
        None
    }
}

impl App for CLI {
    fn init(&mut self, init_commands: Vec<Command>) {
        self.command_queue.add_commands(init_commands);
    }
    fn add_command(&mut self, cmd: Command) {
        self.command_queue.add_command(cmd);
    }
    fn add_commands(&mut self, commands: Vec<Command>) {
        self.command_queue.add_commands(commands);
    }
    fn update(&mut self) {
        self.command_queue.execute(&mut self.context);
    }
    fn process_command(&mut self, cmd: Command) {
        self.process_cli_command(cmd);
    }
}

pub fn get_cli_command() -> Command {
    let mut buffer = String::new();

    buffer.clear();

    info!("Please enter command! (type 'help' for list of commands)");
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
