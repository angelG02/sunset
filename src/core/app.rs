use std::any::Any;

use winit::event_loop::EventLoopProxy;

use super::events::CommandEvent;
use crate::core::command_queue::*;

use async_trait::async_trait;

#[async_trait]
pub trait App {
    fn build(&self) {}
    fn init(&mut self, init_commands: Vec<Command>);
    fn queue_commands(&mut self /*schedule: Schedule, */) -> Vec<Command>;
    fn process_command(&mut self, cmd: Command);
    async fn process_event(
        &mut self,
        event: &winit::event::Event<CommandEvent>,
        elp: EventLoopProxy<CommandEvent>,
    );
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
