use async_trait::async_trait;
use winit::event_loop::EventLoopProxy;

use crate::core::{app::App, command_queue::Command, events::CommandEvent};

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
}

#[async_trait(?Send)]
impl App for AssetServer {
    fn init(&mut self, mut init_commands: Vec<crate::core::command_queue::Command>) {
        self.commands.append(&mut init_commands);
    }

    fn process_command(&mut self, _cmd: Command) {}

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
