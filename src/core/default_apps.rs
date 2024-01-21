use crate::{core::app::App, gallery::scene, renderer, window::windower};

pub fn default_apps() -> Vec<(String, Box<dyn App>)> {
    let windower = windower::Windower::default();

    let sun = renderer::sun::Sun::default();
    let scene = scene::Scene::default();
    let asset_server = crate::assets::asset_server::AssetServer::new("127.0.0.1:7878".into());

    vec![
        ("windower".into(), Box::new(windower)),
        ("sun".into(), Box::new(sun)),
        ("asset_server".into(), Box::new(asset_server)),
        ("default_scene".into(), Box::new(scene)),
    ]
}
