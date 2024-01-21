pub mod asset_cmd;
pub mod asset_server;

#[derive(Debug, Clone)]
pub struct Asset {
    pub asset_type: AssetType,
    pub data: Vec<u8>,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssetType {
    String,
    Shader,
    Texture,
    Unknown,
    //...model, texture, audio...
}
