# sunset
Rust multi-platform application engine

# How to run: 
1. Make sure to have installed cargo and the latest version of rustc;
2. Create new project with cargo new <proj-name>;
3. Get latest version of sunset engine and place it next to your newly created project:
<p align="center">
 <img src="https://github.com/angelG02/sunset/assets/112871889/3d17bc11-59e3-42e8-96b3-76aad7012444)/>
</p>

5. Add Sunset engine to your project's dependencies (in Cargo.toml inside your newly created project):
```Toml
[dependencies]
sunset = { path = "../sunset" }
```

7. Configure your main application and run the sunset state:
```Rust
use state::State;
use sunset::prelude::*;

fn main() {
    let cli = cli::CLI {
        command_queue: command_queue::CommandQueue::default(),
        context: command_queue::Context,
    };

    State::insert_app("CLI", Box::new(cli));
    State::run();
}

```

9. To run your app open a terminal in your project's folder and type `cargo run`
