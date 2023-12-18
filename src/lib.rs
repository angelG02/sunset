pub mod core;
pub mod renderer;
pub mod window;

pub mod prelude {
    pub use crate::core::*;
    pub use crate::renderer::*;
    pub use crate::window::*;
}
