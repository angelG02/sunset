use bevy_ecs::component::Component;
use tracing::{error, info};

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

    // NOTE (A40): all 3 should be replaced by Transfrom Comp
    pub eye: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub forward: cgmath::Vector3<f32>,

    pub znear: f32,
    pub zfar: f32,
}

impl CameraComponent {
    pub fn from_args(args: Vec<&str>) -> Option<Self> {
        if !args.contains(&"2D") && !args.contains(&"3D") {
            error!("Expected at least 2 arguments with <camera type> being the first (either '2D' or '3D'");
            return None;
        }

        match args[0] {
            "2D" => {
                if args.len() < 10 {
                    error!("Expected 10 arguments. Add 'help' to see usage.");
                    return None;
                }

                if args.contains(&"help") {
                    info!(
                        "\n<orthogonal camera props> (left: f32, right: f32, bottom: f32, top: f32)
                        \n<camera pos> (x: f32, y: f32, z: f32)
                        \n<znear and zfar> (f32, f32)"
                    );
                }

                let left: f32 = args[1].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'left': {}", args[1]);
                    0.0
                });
                let right: f32 = args[2].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'right': {}", args[2]);
                    0.0
                });
                let bottom: f32 = args[3].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'bottom': {}", args[3]);
                    0.0
                });
                let top: f32 = args[4].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'top': {}", args[4]);
                    0.0
                });

                let x: f32 = args[5].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'camera_pos:x': {}", args[5]);
                    0.0
                });
                let y: f32 = args[6].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'camera_pos:y': {}", args[6]);
                    0.0
                });
                let z: f32 = args[7].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'camera_pos:z': {}", args[7]);
                    0.0
                });

                let camera_pos: cgmath::Point3<f32> = cgmath::Point3 { x, y, z };
                let znear: f32 = args[8].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'znear': {}", args[8]);
                    0.0
                });
                let zfar: f32 = args[9].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'zfar': {}", args[9]);
                    0.0
                });

                let props = OrthogonalProps {
                    left,
                    right,
                    bottom,
                    top,
                };

                Some(CameraComponent {
                    camera_type: CamType::Orthogonal(props),
                    eye: camera_pos,
                    up: cgmath::Vector3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                    forward: cgmath::Vector3 {
                        x: 0.0,
                        y: 0.0,
                        z: -1.0,
                    },
                    znear,
                    zfar,
                })
            }
            "3D" => {
                if args.len() < 7 {
                    error!(
                        "Expected at least 7 arguments. Add 'help' to argument list to see usage."
                    );
                    return None;
                }

                if args.contains(&"help") {
                    info!(
                        "\n<perspective camera props> (aspcet ratio: f32, fovy: f32)
                        \n<camera pos> (x: f32, y: f32, z: f32)
                        \n<znear and zfar> (f32, f32) <- false info"
                    );
                }

                let aspect: f32 = args[1].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'aspect ratio': {}", args[0]);
                    0.0
                });
                let fovy: f32 = args[2].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'fovy': {}", args[1]);
                    0.0
                });

                let x: f32 = args[3].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'camera_pos:x': {}", args[2]);
                    0.0
                });
                let y: f32 = args[4].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'camera_pos:y': {}", args[3]);
                    0.0
                });
                let z: f32 = args[5].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'camera_pos:z': {}", args[4]);
                    0.0
                });

                let camera_pos: cgmath::Point3<f32> = cgmath::Point3 { x, y, z };
                let znear: f32 = args[6].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'znear': {}", args[5]);
                    0.0
                });
                let zfar: f32 = args[7].parse().unwrap_or_else(|_| {
                    error!("Could not parse value for 'zfar': {}", args[6]);
                    0.0
                });

                let props = PerspectiveProps { aspect, fovy };

                //info!("{:?}", props);

                Some(CameraComponent {
                    camera_type: CamType::Perspective(props),
                    eye: camera_pos,
                    up: cgmath::Vector3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                    forward: cgmath::Vector3 {
                        x: 0.0,
                        y: 0.0,
                        z: -1.0,
                    },
                    znear,
                    zfar,
                })
            }
            _ => {
                error!("Expected first argument to be one of <2D, 3D>");
                None
            }
        }
    }

    pub fn build_vp_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_to_rh(self.eye, self.forward, self.up);

        match &self.camera_type {
            CamType::Perspective(props) => {
                let proj = cgmath::perspective(
                    cgmath::Deg(props.fovy),
                    props.aspect,
                    self.znear,
                    self.zfar,
                );
                OPENGL_TO_WGPU_MATRIX * proj * view
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
                OPENGL_TO_WGPU_MATRIX * proj * view
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

impl Default for CameraComponent {
    fn default() -> Self {
        Self {
            camera_type: CamType::Orthogonal(OrthogonalProps {
                left: -1.0,
                right: 1.0,
                bottom: -1.0,
                top: 1.0,
            }),
            eye: cgmath::Point3::new(0.0, 0.0, 0.0),
            forward: cgmath::Vector3::new(0.0, 0.0, -1.0),
            up: cgmath::Vector3::new(0.0, 1.0, 0.0),
            znear: 0.1,
            zfar: 100.0,
        }
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CameraUniform {
    vp: [[f32; 4]; 4],
}

unsafe impl bytemuck::Zeroable for CameraUniform {}
unsafe impl bytemuck::Pod for CameraUniform {}

impl CameraUniform {
    pub fn from_camera(cam: &CameraComponent) -> Self {
        Self {
            vp: cam.build_vp_matrix().into(),
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct ActiveCameraComponent {}
