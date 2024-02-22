use crate::{core::app::App, gallery::scene, renderer, window::windower};

pub fn default_apps() -> Vec<(String, Box<dyn App>)> {
    let windower = windower::Windower::default();

    let sun = renderer::sun::Sun::default();
    let scene = scene::Scene::default();

    // Asset server for http requests
    #[cfg(not(target_arch = "wasm32"))]
    let asset_server =
        crate::assets::asset_server::AssetServer::new("as-http.angel-sunset.app:8080".into());

    // Asset server for web requests
    #[cfg(target_arch = "wasm32")]
    let asset_server = crate::assets::asset_server::AssetServer::new("angel-sunset.app".into());

    vec![
        ("windower".into(), Box::new(windower)),
        ("sun".into(), Box::new(sun)),
        ("asset_server".into(), Box::new(asset_server)),
        ("default_scene".into(), Box::new(scene)),
    ]
}
