use std::collections::HashMap;

use bevy_ecs::{entity::Entity, world::World};

use crate::prelude::{
    transform_component::TransformComponent, ui_component::UIComponent, ChangeComponentState,
};

#[derive(Default)]
pub struct UIHandler {
    pub id_map: HashMap<String, Entity>,
}

impl UIHandler {
    pub fn new() -> Self {
        Self {
            id_map: HashMap::new(),
        }
    }

    pub fn add_handle(&mut self, ui_string_id: String, entity: Entity) {
        self.id_map.insert(ui_string_id, entity);
    }

    pub fn on_change_component_state(
        &mut self,
        change_state: &ChangeComponentState,
        world: &mut World,
    ) {
        match change_state {
            // Replace/Add Text
            crate::prelude::ChangeComponentState::UI((changed_ui, changed_transform)) => {
                let id = &changed_ui.string_id;
                if let Some(entity) = self.id_map.get(id) {
                    // Replace or spawn component of the requested type on the provided entity
                    if let Some(mut ui) = world.get_mut::<UIComponent>(*entity) {
                        *ui = UIComponent {
                            id: ui.id,
                            string_id: ui.string_id.clone(),
                            ui_type: changed_ui.ui_type.clone(),
                        };
                    } else {
                        world.entity_mut(*entity).insert(changed_ui.clone());
                    }

                    if changed_transform.is_some() {
                        if let Some(mut transform) = world.get_mut::<TransformComponent>(*entity) {
                            *transform = changed_transform.clone().unwrap();
                        } else {
                            world
                                .entity_mut(*entity)
                                .insert(changed_transform.clone().unwrap());
                        }
                    }
                } else {
                    let mut entity = world.spawn_empty();

                    entity.insert(changed_ui.clone());

                    if let Some(trans) = changed_transform {
                        entity.insert(trans.clone());
                    }
                    self.id_map.insert(id.clone(), entity.id());
                }
            }
            ChangeComponentState::Window(_) => todo!(),
        }
    }
}
