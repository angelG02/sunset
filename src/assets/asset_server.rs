use async_trait::async_trait;
use tracing::info;
use winit::event_loop::EventLoopProxy;

use crate::core::{
    app::App,
    command_queue::{Command, Task},
    events::CommandEvent,
};

use super::asset_cmd::AssetCommand;

pub struct AssetServer {
    pub server_addr: String,
    pub commands: Vec<Command>,
}

impl AssetServer {
    pub fn new(addr: String) -> Self {
        AssetServer {
            server_addr: addr,
            commands: vec![],
        }
    }

    pub fn process_asset_command(&mut self, mut cmd: Command, elp: EventLoopProxy<CommandEvent>) {
        let args = cmd.args.clone().unwrap();
        let vec_args: Vec<&str> = args.split(' ').collect();

        let task = match vec_args[0].to_ascii_lowercase().as_str() {
            "get" => self.get(&vec_args[1..].join(" "), elp),
            //"put" => self.put(args),
            _ => AssetServer::unsupported(args.as_str()),
        };

        cmd.processed = true;
        cmd.task = task;
        cmd.args = Some(args);

        self.commands.push(cmd);
    }

    pub fn get(
        &self,
        args: &str,
        elp: EventLoopProxy<CommandEvent>,
    ) -> Option<Task<Vec<CommandEvent>>> {
        let vec_args: Vec<&str> = args.split(' ').collect();
        let args = format!("{} {} {}", self.server_addr, vec_args[0], vec_args[1]);

        info!("{}", args);

        AssetCommand::get_from_server(args, elp)
    }
}

#[async_trait(?Send)]
impl App for AssetServer {
    fn init(&mut self, _elp: EventLoopProxy<CommandEvent>) {}

    async fn process_command(&mut self, cmd: Command, elp: EventLoopProxy<CommandEvent>) {
        self.process_asset_command(cmd, elp)
    }

    async fn process_event(
        &mut self,
        _event: &winit::event::Event<crate::core::events::CommandEvent>,
        _elp: EventLoopProxy<CommandEvent>,
    ) {
    }

    fn update(&mut self /*schedule: Schedule, */) -> Vec<Command> {
        self.commands.drain(..).collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
