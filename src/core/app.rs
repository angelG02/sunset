use crate::core::command_queue::*;

pub trait App {
    fn build(&self) {}
    fn init(&mut self, init_commands: Vec<Command>);
    fn add_command(&mut self, cmd: Command);
    fn add_commands(&mut self, commands: Vec<Command>);
    fn update(&mut self);
    fn process_command(&mut self, cmd: Command);
}

// pub struct App {
//     pub cmd_queue: CommandQueue,
//     pub ctx: Context,
// }

// impl App {
//     pub fn init(&mut self, init_commands: Vec<Command>) {
//         self.cmd_queue.add_commands(init_commands);
//     }

//     pub fn add_command(&mut self, cmd: Command) {
//         self.cmd_queue.add_command(cmd);
//     }

//     pub fn add_commands(&mut self, commands: Vec<Command>) {
//         self.cmd_queue.add_commands(commands);
//     }

//     pub fn update(&mut self) {
//         self.cmd_queue.execute(&mut self.ctx);
//     }
// }
