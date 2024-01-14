use std::any::Any;

use winit::event_loop::EventLoopProxy;

use super::events::CommandEvent;
use crate::core::command_queue::*;

use async_trait::async_trait;

#[async_trait(?Send)]
/// The `App` trait represents the core functionality of an application.
pub trait App {
    /// Initializes the application with the provided initial commands.
    ///
    /// # Arguments
    ///
    /// * `init_commands` - A vector of `Command` objects representing the initial commands.
    ///
    fn init(&mut self, init_commands: Vec<Command>);

    /// Queues commands to be processed by the application duricng the current frame.
    ///
    /// # Returns
    ///
    /// A vector of `Command` objects representing the commands to be queued from the app.
    ///
    fn update(&mut self /*schedule: Schedule, */) -> Vec<Command>;

    /// Processes a single command.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The `Command` object to be processed (comes from the CLI App with only the argumets provided).
    ///
    fn process_command(&mut self, cmd: Command);

    /// Processes an event asynchronously.
    ///
    /// # Arguments
    ///
    /// * `event` - an event dispatched from the event loop.
    /// * `elp` - The event loop proxy (can be used to send other events when the current one has been handled).
    ///
    async fn process_event(
        &mut self,
        event: &winit::event::Event<CommandEvent>,
        elp: EventLoopProxy<CommandEvent>,
    );

    /// Strips the App object from all of its implementaions and returns it as an Any object.
    /// This object can be used to downcast the app to a concrete type (Ex: App -> Any -> Windower)
    fn as_any(&self) -> &dyn Any;

    /// Strips the App object from all of its implementaions and returns it as an Any object.
    /// This object can be used to downcast the app to a concrete type (Ex: App -> Any -> Windower)
    ///
    /// # Example
    ///
    /// ```
    /// match event {
    ///     CommandEvent::OpenWindow(props) => {
    ///         let mut state_lock = State::write();
    ///         let windower = state_lock
    ///             .apps
    ///             .get_mut("windower")
    ///             .unwrap()
    ///             .as_any_mut()
    ///             .downcast_mut::<Windower>()
    ///             .unwrap();
    ///
    ///         windower.create_window(props, elp, elwt);
    ///     }
    ///}
    /// ```
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
