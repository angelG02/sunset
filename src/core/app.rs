use std::any::Any;

use winit::event_loop::EventLoopWindowTarget;

use crate::core::command_queue::*;

use super::events::CommandEvent;

pub trait App {
    fn build(&self) {}
    fn init(&mut self, init_commands: Vec<Command>);
    fn queue_commands(&mut self /*schedule: Schedule, */) -> Vec<Command>;
    fn process_command(&mut self, cmd: Command);
    fn process_event(&mut self, event: &CommandEvent, elwt: &EventLoopWindowTarget<CommandEvent>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
