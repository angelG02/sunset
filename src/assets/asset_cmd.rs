use tracing::{debug, error, info};

use crate::{
    assets::{Asset, AssetType},
    core::{
        command_queue::{Command, CommandType, IntoCommand, Task},
        events::CommandEvent,
    },
};

use super::AssetStatus;

#[allow(dead_code)]
pub struct AssetCommand {
    pub processed: bool,
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
            _ => None,
        };

        Self {
            processed: task.is_some(),
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
            io::{Read, Write},
            net::TcpStream,
        };

        use tracing::warn;

        // 127.0.0.1 shader.wgsl shader
        let cmd = move || {
            let mut events = vec![];
            let args: Vec<&str> = args.split(' ').collect();

            if args.contains(&"changed") {
                match TcpStream::connect(args[0]) {
                    Ok(mut stream) => {
                        debug!("Successfully connected to server {}", args[0]);

                        let request = args[1..].join(" ");

                        stream
                            .write_all(&request.len().to_ne_bytes().to_vec())
                            .unwrap_or_else(|err| {
                                error!("Could not write data: {err}");
                                ()
                            });

                        info!("Request size: {}", request.len());

                        stream.write_all(request.as_bytes()).unwrap();

                        let mut data = Vec::new();

                        stream.read_to_end(&mut data).unwrap_or(1);

                        let try_convert_string = std::str::from_utf8(&data);

                        if let Ok(res) = try_convert_string {
                            let paths: Vec<String> = res
                                .split(' ')
                                .into_iter()
                                .map(|path| path.to_string())
                                .collect();
                            events.push(CommandEvent::ChangedAssets(paths));
                        }
                        events
                    }
                    Err(e) => {
                        println!("Failed to connect: {}", e);
                        events
                    }
                }
            } else {
                debug!(
                    "Get Asset {} of type {} from server {}",
                    args[1], args[2], args[0]
                );

                let asset_type = args[2].to_ascii_lowercase().to_owned();
                let asset_path = args[1].to_owned();
                let asset_name = args[1].split('/').last().unwrap().to_owned();

                debug!("path: {}, name: {}", asset_path, asset_name);

                match TcpStream::connect(args[0]) {
                    Ok(mut stream) => {
                        debug!("Successfully connected to server {}", args[0]);

                        let request = format!("get {} {}", asset_path, asset_type);

                        stream
                            .write_all(&request.len().to_ne_bytes().to_vec())
                            .unwrap_or_else(|err| {
                                error!("Could not write data: {err}");
                                ()
                            });

                        info!("Request size: {}", request.len());

                        stream.write_all(request.as_bytes()).unwrap_or_else(|err| {
                            error!("Could not write data: {err}");
                            ()
                        });

                        let mut data = Vec::new();

                        stream.read_to_end(&mut data).unwrap_or_else(|err| {
                            error!("Could not read data: {err}");
                            0
                        });

                        let try_convert_string = std::str::from_utf8(&data);

                        if let Ok(res) = try_convert_string {
                            if res.contains("File not found") {
                                error!("File not found: {}", asset_path);
                                return vec![];
                            }
                        }

                        let asset_type = match asset_type.as_str() {
                            "shader" => AssetType::Shader,
                            "string" => AssetType::String,
                            "texture" => AssetType::Texture,
                            "model" => AssetType::Model,
                            "mesh" => AssetType::Mesh,
                            "material" => AssetType::Material,
                            _ => {
                                warn!("Unkown asset type requested: {asset_type:?}");
                                AssetType::Unknown
                            }
                        };

                        vec![CommandEvent::Asset(Asset {
                            asset_type,
                            status: AssetStatus::Ready,
                            data,
                            name: asset_name,
                            path: asset_path,
                        })]
                    }
                    Err(e) => {
                        println!("Failed to connect: {}", e);
                        vec![]
                    }
                }
            }
        };

        Some(Box::new(cmd))
    }

    pub fn get_local(args: String) -> Option<Task<Vec<CommandEvent>>> {
        let cmd = move || {
            let args: Vec<&str> = args.split(' ').collect();
            let asset_type = args[2].to_ascii_lowercase().to_owned();
            let asset_path = args[1].to_owned();
            let asset_name = args[1].split('/').last().unwrap().to_owned();

            let asset_type = match asset_type.as_str() {
                "shader" => AssetType::Shader,
                "string" => AssetType::String,
                "texture" => AssetType::Texture,
                "model" => AssetType::Model,
                "mesh" => AssetType::Mesh,
                "material" => AssetType::Material,
                _ => AssetType::Unknown,
            };

            let asset_folder_path = std::env::var("ASSETS_PATH").unwrap();
            let full_path = format!("{asset_folder_path}{asset_path}");

            let data = std::fs::read(full_path);

            if data.is_ok() {
                return vec![CommandEvent::Asset(Asset {
                    asset_type,
                    status: AssetStatus::Ready,
                    data: data.unwrap(),
                    name: asset_name,
                    path: asset_path,
                })];
            }

            vec![CommandEvent::Asset(Asset {
                asset_type,
                status: AssetStatus::NotFound,
                data: vec![],
                name: asset_name,
                path: asset_path,
            })]
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
            let elp_1 = elp.clone();
            let elp_2 = elp.clone();

            let args: Vec<&str> = args.split(' ').collect();
            let addr = args[0];

            if args.contains(&"changed") {
                let url = format!("{}{}", "wss://", addr);

                let ws = web_sys::WebSocket::new(url.as_str()).unwrap();
                let cloned_ws = ws.clone();

                let onopen_callback = Closure::<dyn FnMut()>::new(move || {
                    //debug!("socket opened");
                    match cloned_ws.send_with_str("get changed") {
                        Ok(_) => {
                            //debug!("message successfully sent");
                        }
                        Err(err) => error!("error sending message: {:?}", err),
                    }
                });
                ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
                onopen_callback.forget();

                let onmessage_callback =
                    Closure::<dyn FnMut(_)>::new(move |e: web_sys::MessageEvent| {
                        if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                            debug!("message event, received blob: {:?}", blob);
                        } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                            let res: String = txt.into();
                            let paths: Vec<String> = res
                                .split(' ')
                                .into_iter()
                                .map(|path| path.to_string())
                                .collect();

                            elp_1
                                .clone()
                                .send_event(CommandEvent::ChangedAssets(paths))
                                .unwrap();
                        } else {
                            debug!("message event, received Unknown: {:?}", e.data());
                        }
                    });
                // set message event handler on WebSocket
                ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
                // forget the callback to keep it alive
                onmessage_callback.forget();

                //CommandEvent::FilePending(file_path.to_owned())
                vec![]
            } else {
                debug!("Get Asset {} from server {}", args[1], args[0]);

                let file_path = args[1];

                let request_type = args[2].to_ascii_lowercase().to_owned();
                let asset_path = args[1].to_owned();
                let asset_name = args[1].split('/').last().unwrap().to_owned();
                let asset_type = match args[2].to_ascii_lowercase().as_str() {
                    "shader" => AssetType::Shader,
                    "string" => AssetType::String,
                    "texture" => AssetType::Texture,
                    "model" => AssetType::Model,
                    "mesh" => AssetType::Mesh,
                    "material" => AssetType::Material,
                    _ => AssetType::Unknown,
                };

                let url = format!("{}{}", "wss://", addr);

                let ws = web_sys::WebSocket::new(url.as_str()).unwrap();
                let cloned_ws = ws.clone();

                let file_path_copy = file_path.to_owned();
                let onopen_callback = Closure::<dyn FnMut()>::new(move || {
                    //debug!("socket opened");
                    match cloned_ws.send_with_str(
                        format!("{}{} {}", "get ", file_path_copy, request_type).as_str(),
                    ) {
                        Ok(_) => debug!("message successfully sent"),
                        Err(err) => debug!("error sending message: {:?}", err),
                    }
                });
                ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
                onopen_callback.forget();

                let onmessage_callback =
                    Closure::<dyn FnMut(_)>::new(move |e: web_sys::MessageEvent| {
                        if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                            let elp_c1 = elp_1.clone();
                            let asset_type_clone = asset_type.clone();
                            let asset_name_clone = asset_name.clone();
                            let asset_path_clone = asset_path.clone();
                            debug!("message event, received blob: {:?}", blob);
                            // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
                            let fr = web_sys::FileReader::new().unwrap();
                            let fr_c = fr.clone();
                            // create onLoadEnd callback
                            let onloadend_cb =
                                Closure::<dyn FnMut(_)>::new(move |_e: web_sys::ProgressEvent| {
                                    let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
                                    let _len = array.byte_length() as usize;
                                    // here you can for example use the received image/png data

                                    elp_c1
                                        .clone()
                                        .send_event(CommandEvent::Asset(Asset {
                                            asset_type: asset_type_clone.clone(),
                                            data: array.to_vec(),
                                            status: AssetStatus::Ready,
                                            name: asset_name_clone.clone(),
                                            path: asset_path_clone.clone(),
                                        }))
                                        .unwrap();
                                });
                            fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                            fr.read_as_array_buffer(&blob).expect("blob not readable");
                            onloadend_cb.forget();
                        } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                            //info!("message event, received Text: {:?}", txt);
                            let data: String = txt.into();

                            elp_2
                                .send_event(CommandEvent::Asset(Asset {
                                    asset_type: asset_type.clone(),
                                    data: data.into(),
                                    status: AssetStatus::Ready,
                                    name: asset_name.clone(),
                                    path: asset_path.clone(),
                                }))
                                .expect("Could not send event T-T");
                        } else {
                            debug!("message event, received Unknown: {:?}", e.data());
                        }
                    });
                // set message event handler on WebSocket
                ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
                // forget the callback to keep it alive
                onmessage_callback.forget();

                //CommandEvent::FilePending(file_path.to_owned())
                vec![]
            }
        };

        Some(Box::new(cmd))
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
            processed: self.task.is_some(),
            app: "AssetServer".into(),
            command_type: self.command_type,
            args: Some(self.args),
            task: self.task,
        }
    }
}
