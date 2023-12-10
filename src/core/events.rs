use winit::dpi::PhysicalSize;

#[derive(Default, Debug, Clone)]
pub struct NewWindowProps {
    pub size: PhysicalSize<u32>,
    pub name: String,
    // Option<Decorations...icon...etc, etc>
}

#[derive(Debug, Clone)]
pub enum CommandEvent {
    OpenWindow(NewWindowProps),
    CloseWindow(winit::window::WindowId),
    Exit,
    File(String),
    FileNotFound,
    FilePending(String),
    None,
}
