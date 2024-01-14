use tracing::info;

use crate::{
    assets::{Asset, AssetType},
    core::{
        command_queue::{Command, CommandType, IntoCommand, Task},
        events::CommandEvent,
    },
};

#[allow(dead_code)]
pub struct AssetCommand {
    pub command_type: CommandType,
    pub args: String,
    pub task: Option<Task<Vec<CommandEvent>>>,
}

impl AssetCommand {
    pub fn new(
        command_type: CommandType,
        args: String,
        elp: winit::event_loop::EventLoopProxy<CommandEvent>,
    ) -> Self {
        let task = match command_type {
            CommandType::Get => {
                let args: Vec<&str> = args.split(' ').collect();
                match args[0] {
                    "-h" => AssetCommand::display_help(),
                    "-from_server" => AssetCommand::get_from_server(args[1..].join(" "), elp),
                    "-local" => AssetCommand::get_local(args[1..].join(" ")),
                    _ => AssetCommand::display_help(),
                }
            }
            _ => AssetCommand::unsupported(args.clone()),
        };

        Self {
            command_type,
            args,
            task,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_from_server(
        args: String,
        _elp: winit::event_loop::EventLoopProxy<CommandEvent>,
    ) -> Option<Task<Vec<CommandEvent>>> {
        use std::{
            io::{BufRead, BufReader, Write},
            net::TcpStream,
        };

        // 127.0.0.1 shader.wgsl shader
        let cmd = move || {
            let args: Vec<&str> = args.split(' ').collect();
            info!(
                "Get Asset {} of type {} from server {}",
                args[1], args[2], args[0]
            );

            let asset_path = args[1].to_owned();
            let asset_name = args[1].split('/').last().unwrap().to_owned();

            info!("path: {}, name: {}", asset_path, asset_name);

            match TcpStream::connect(args[0]) {
                Ok(mut stream) => {
                    info!("Successfully connected to server {}", args[0]);

                    let request = format!("get {}\r\n\r\n", args[1].to_owned());

                    stream.write_all(request.as_bytes()).unwrap();

                    let buf_reader = BufReader::new(&mut stream);
                    let response: Vec<_> = buf_reader
                        .lines()
                        .map(|result| {
                            let mut r = result.unwrap();
                            r.push_str("\r\n");
                            r
                        })
                        .take_while(|line| !line.contains("END OF FILE"))
                        .collect();

                    let asset_type = match args[2].to_ascii_lowercase().as_str() {
                        "shader" => AssetType::Shader,
                        "string" => AssetType::String,
                        _ => AssetType::Unknown,
                    };

                    vec![CommandEvent::Asset(Asset {
                        asset_type,
                        data: response.concat(),
                        name: asset_name,
                        path: asset_path,
                    })]
                }
                Err(e) => {
                    println!("Failed to connect: {}", e);
                    vec![]
                }
            }
        };

        Some(Box::new(cmd))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_from_server(
        args: String,
        elp: winit::event_loop::EventLoopProxy<CommandEvent>,
    ) -> Option<Task<Vec<CommandEvent>>> {
        let cmd = move || {
            use wasm_bindgen::prelude::*;

            // Needs a clone here so the function can become FnMut (otherwise elp will be moved out of the closure by the forgotten server closure below)
            let elp = elp.clone();

            let args: Vec<&str> = args.split(' ').collect();
            info!("Get Asset {} from server {}", args[1], args[0]);

            let addr = args[0];
            let file_path = args[1];

            let asset_path = args[1].to_owned();
            let asset_name = args[1].split('/').last().unwrap().to_owned();
            let asset_type = match args[2].to_ascii_lowercase().as_str() {
                "shader" => AssetType::Shader,
                "string" => AssetType::String,
                _ => AssetType::Unknown,
            };

            let url = format!("{}{}", "ws://", addr);

            let ws = web_sys::WebSocket::new(url.as_str()).unwrap();
            let cloned_ws = ws.clone();

            let file_path_copy = file_path.clone().to_owned();
            let onopen_callback = Closure::<dyn FnMut()>::new(move || {
                info!("socket opened");
                match cloned_ws.send_with_str(format!("{}{}", "get ", file_path_copy).as_str()) {
                    Ok(_) => info!("message successfully sent"),
                    Err(err) => info!("error sending message: {:?}", err),
                }
            });
            ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();

            let file_path_copy = file_path.clone();
            let onmessage_callback =
                Closure::<dyn FnMut(_)>::new(move |e: web_sys::MessageEvent| {
                    if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                        //info!("message event, received Text: {:?}", txt);
                        let str_out: String = txt.into();

                        elp.send_event(CommandEvent::Asset(Asset {
                            asset_type: asset_type.clone(),
                            data: str_out,
                            name: asset_name.clone(),
                            path: asset_path.clone(),
                        }))
                        .expect("Could not send event T-T");
                    } else {
                        info!("message event, received Unknown: {:?}", e.data());
                    }
                });
            // set message event handler on WebSocket
            ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            // forget the callback to keep it alive
            onmessage_callback.forget();

            //CommandEvent::FilePending(file_path.to_owned())
            vec![]
        };

        Some(Box::new(cmd))
    }

    fn get_local(args: String) -> Option<Task<Vec<CommandEvent>>> {
        let cmd = move || {
            let args: Vec<&str> = args.split(' ').collect();
            info!("Get Asset {}", args[0]);

            let asset_path = args[1].to_owned();
            let asset_name = args[1].split('/').last().unwrap().to_owned();

            let asset_type = match args[1].to_ascii_lowercase().as_str() {
                "shader" => AssetType::Shader,
                "string" => AssetType::String,
                _ => AssetType::Unknown,
            };

            vec![CommandEvent::Asset(Asset {
                asset_type,
                data: args[0].to_owned(),
                name: asset_name,
                path: asset_path,
            })]
        };

        Some(Box::new(cmd))
    }

    fn unsupported(_args: String) -> Option<Task<Vec<CommandEvent>>> {
        //let args: Vec<&str> = args.split(' ').collect();

        info!("Unsupported Asset Command!");
        None
    }

    // TODO: ALL COMMANDS NEED THIS
    fn display_help() -> Option<Task<Vec<CommandEvent>>> {
        info!("-from_server <port> <file_path>: Query the asset server for the specified asset");
        info!("-local <file_path>: Get the asset from the local asset dir (ONLY WORKS ON not(target = wasm32)");
        None
    }
}

impl IntoCommand for AssetCommand {
    fn into_command(self) -> Command {
        Command {
            app: "AssetServer".into(),
            command_type: self.command_type,
            args: Some(self.args),
            task: self.task,
        }
    }
}
