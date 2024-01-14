use crate::{
    core::{
        app::App,
        command_queue::{Command, CommandType},
    },
    gallery::scene,
    renderer,
    window::windower,
};

pub fn default_apps() -> Vec<(String, Box<dyn App>)> {
    let mut windower = windower::Windower::default();
    let task = windower.open("Sandbox 1920 1080".into());
    windower.init(vec![Command {
        app: "Windower".into(),
        args: None,
        command_type: CommandType::Open,
        task,
    }]);

    let sun = renderer::sun::Sun::default();
    let scene = scene::Scene::default();

    vec![
        ("windower".into(), Box::new(windower)),
        ("sun".into(), Box::new(sun)),
        ("defautl_scene".into(), Box::new(scene)),
    ]
}
