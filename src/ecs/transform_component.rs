use bevy_ecs::component::Component;
use cgmath::{Matrix3, Matrix4, SquareMatrix, Zero};

#[repr(C)]
#[derive(Debug, Clone, Copy, Component)]
pub struct TransformComponent {
    pub translation: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub scale: cgmath::Vector3<f32>,

    pub model_matrix: cgmath::Matrix4<f32>,

    pub dirty: bool,
}

impl TransformComponent {
    pub fn zero() -> Self {
        Self {
            translation: cgmath::Vector3::zero(),
            rotation: cgmath::Quaternion::zero(),
            scale: cgmath::Vector3::<f32>::new(1.0, 1.0, 1.0),
            model_matrix: cgmath::Matrix4::<f32>::from_value(1.0),
            dirty: true,
        }
    }

    pub fn recalculate(&mut self) {
        if self.dirty {
            let trans = cgmath::Matrix4::<f32>::from_translation(self.translation);
            let scale = cgmath::Matrix4::<f32>::from_nonuniform_scale(
                self.scale.x,
                self.scale.y,
                self.scale.z,
            );
            let rot: Matrix3<f32> = self.rotation.into();
            let rot: Matrix4<f32> = rot.into();

            self.model_matrix = trans * rot * scale;

            self.dirty = false;
        }
    }
}

unsafe impl bytemuck::Zeroable for TransformComponent {}
unsafe impl bytemuck::Pod for TransformComponent {}
