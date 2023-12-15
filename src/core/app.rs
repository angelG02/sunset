use crate::core::command_queue::*;

pub trait App {
    fn build(&self) {}
    fn init(&mut self, init_commands: Vec<Command>);
    fn queue_commands(&mut self /*schedule: Schedule, */) -> Vec<Command>;
    fn process_command(&mut self, cmd: Command);
}
