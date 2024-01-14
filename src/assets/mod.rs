pub mod asset_cmd;
pub mod asset_server;

#[derive(Debug, Clone)]
pub struct Asset {
    pub asset_type: AssetType,
    pub data: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssetType {
    String,
    Shader,
    Unknown,
    //...model, texture, audio...
}
