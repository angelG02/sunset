use bevy_ecs::component::Component;

#[derive(Debug, Clone)]
pub struct PerspectiveProps {
    pub aspect: f32,
    pub fovy: f32,
}

#[derive(Debug, Clone)]
pub struct OrthogonalProps {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
}

#[derive(Debug, Clone)]
pub enum CamType {
    Perspective(PerspectiveProps),
    Orthogonal(OrthogonalProps),
}

#[derive(Debug, Clone, Component)]
pub struct CameraComponent {
    pub camera_type: CamType,

    // NOTE (A40): Should be replaced by Transfrom Comp
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub znear: f32,
    pub zfar: f32,

    // Temp
    pub uuid: uuid::Uuid,
}

impl CameraComponent {
    pub fn build_vp_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);

        match &self.camera_type {
            CamType::Perspective(props) => {
                let proj = cgmath::perspective(
                    cgmath::Deg(props.fovy),
                    props.aspect,
                    self.znear,
                    self.zfar,
                );
                return OPENGL_TO_WGPU_MATRIX * proj * view;
            }
            CamType::Orthogonal(props) => {
                let proj = cgmath::ortho(
                    props.left,
                    props.right,
                    props.bottom,
                    props.top,
                    self.znear,
                    self.zfar,
                );
                return OPENGL_TO_WGPU_MATRIX * proj * view;
            }
        }
    }

    pub fn layout_desc() -> wgpu::BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        }
    }
}

// impl Default for CameraComponent {
//     fn default() -> Self {
//         CameraComponent {
//             camera_type: CamType::Perspective(PerspectiveProps {
//                 aspect: (),
//                 fovy: (),
//             }),
//             eye: (),
//             target: (),
//             up: (),
//             znear: (),
//             zfar: (),
//         }
//     }
// }

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);
