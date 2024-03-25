use std::collections::HashMap;

use async_trait::async_trait;
use tracing::{error, warn};
use winit::event_loop::EventLoopProxy;

use crate::{
    core::{
        app::App,
        command_queue::{Command, Task},
        events::CommandEvent,
    },
    prelude::command_queue::CommandType,
};

use super::{asset_cmd::AssetCommand, Asset, AssetStatus};

pub struct AssetServer {
    pub server_addr: String,
    pub commands: Vec<Command>,

    pub cached_assets: HashMap<String, Asset>,

    pub proxy: Option<EventLoopProxy<CommandEvent>>,
}

impl AssetServer {
    pub fn new(addr: String) -> Self {
        AssetServer {
            server_addr: addr,
            commands: vec![],

            cached_assets: HashMap::new(),

            proxy: None,
        }
    }

    pub fn process_asset_command(&mut self, mut cmd: Command) {
        let args = cmd.args.clone().unwrap();
        let vec_args: Vec<&str> = args.split(' ').collect();

        let task = match vec_args[0].to_ascii_lowercase().as_str() {
            "get" => self.get(&vec_args[1..].join(" ")),
            //"put" => self.put(args),
            _ => AssetServer::unsupported(args.as_str()),
        };

        cmd.processed = true;
        cmd.task = task;
        cmd.args = Some(args);

        self.commands.push(cmd);
    }

    pub fn get(&self, args: &str) -> Option<Task<Vec<CommandEvent>>> {
        let vec_args: Vec<&str> = args.split(' ').collect();

        if vec_args.len() < 2 {
            error!("Expected 2 arguments to command <get>!");
            return None;
        }

        let args = format!("{} {} {}", self.server_addr, vec_args[0], vec_args[1]);

        let cmd_args: Vec<String> = std::env::args().collect();

        if cmd_args.contains(&"local".to_string()) {
            return AssetCommand::get_local(args);
        }

        AssetCommand::get_from_server(args, self.proxy.clone().unwrap())
    }
}

#[async_trait(?Send)]
impl App for AssetServer {
    fn init(&mut self, elp: EventLoopProxy<CommandEvent>) {
        self.proxy = Some(elp.clone())
    }

    async fn process_command(&mut self, cmd: Command) {
        self.process_asset_command(cmd)
    }

    async fn process_user_event(
        &mut self,
        event: &crate::core::events::CommandEvent,
        _delta_time: f32,
    ) {
        match event {
            CommandEvent::Asset(asset) => {
                if asset.status == AssetStatus::NotFound {
                    warn!("File <{}> not found!", asset.path);
                } else {
                    self.cached_assets.insert(asset.path.clone(), asset.clone());
                }
            }
            CommandEvent::RequestCreateModel(model_comp) => {
                if self.cached_assets.contains_key(&model_comp.model_path) {
                    return;
                }

                let task = self.get(&format!("{} model", model_comp.model_path));

                let cmd = Command {
                    app: "asset_server".into(),
                    command_type: CommandType::Get,
                    processed: true,
                    args: None,
                    task,
                };

                self.commands.push(cmd);
            }
            _ => {}
        }
    }

    fn update(&mut self, _delta_time: f32) -> Vec<Command> {
        self.commands.drain(..).collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
