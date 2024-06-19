use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tracing::{error, info};
use winit::{event::WindowEvent, event_loop::EventLoopProxy, window::Window};

use crate::{
    core::{app::App, command_queue::Command, events::CommandEvent},
    prelude::{
        camera_component::{CameraComponent, ModelUniform},
        command_queue::CommandType,
        state,
        text_component::TextDesc,
        transform_component::TransformComponent,
        ui_component::{RenderUIDesc, UIComponent, UIType},
        window_component::WindowContainer,
        Asset, AssetStatus, AssetType, ChangeComponentState,
    },
};

pub type ResourceID = uuid::Uuid;
pub type PrimitiveID = uuid::Uuid;

use super::{
    buffer::SunBuffer,
    pipeline::{PipelineDesc, SunPipeline},
    primitive::{Render2D, VertexExt},
    resources::{
        font::SunFont,
        model::{DrawModel, RenderModelDesc, SunModel},
        texture::SunTexture,
    },
};

#[derive(Debug, Clone)]
pub struct RenderFrameDesc {
    pub model_desc: RenderModelDesc,
    pub ui_desc: RenderUIDesc,
    pub window_id: winit::window::WindowId,
}

pub struct Sun {
    instance: Option<wgpu::Instance>,
    adapter: Option<wgpu::Adapter>,
    pub device: Option<wgpu::Device>,
    pub queue: Option<wgpu::Queue>,

    pub viewports: HashMap<winit::window::WindowId, Viewport>,
    pub pipelines: HashMap<String, SunPipeline>,
    pub shaders: HashMap<String, Asset>,

    pub mvp_buffer: Option<SunBuffer>,
    pub mvp_bindgroup: Option<wgpu::BindGroup>,

    pub models: HashMap<String, SunModel>,
    pub fonts: HashMap<String, SunFont>,

    pub quad_instance_buffer: Option<SunBuffer>,

    pub vertex_buffers: HashMap<uuid::Uuid, Vec<(SunBuffer, u16)>>,
    pub index_buffers: HashMap<uuid::Uuid, SunBuffer>,
    pub bind_groups: HashMap<String, wgpu::BindGroup>,

    commands: Vec<Command>,

    pub proxy: Option<EventLoopProxy<CommandEvent>>,

    frame_time: web_time::Instant,
    time: web_time::Instant,
}

impl Sun {
    pub async fn create_adapter(&mut self, surface: &wgpu::Surface<'static>) {
        let adapter = self
            .instance
            .as_ref()
            .unwrap()
            .request_adapter(&wgpu::RequestAdapterOptions {
                // Request an adapter which can render to our surface
                compatible_surface: Some(surface),
                ..Default::default()
            })
            .await
            .expect("Failed to find an appropriate adapter");
        self.adapter = Some(adapter);
    }

    pub async fn create_device(&mut self) {
        // Create the logical device and command queue
        let (device, queue) = self
            .adapter
            .as_ref()
            .unwrap()
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::DEPTH32FLOAT_STENCIL8,
                    required_limits: self.adapter.as_ref().unwrap().limits(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        self.device = Some(device);
        self.queue = Some(queue);
    }

    pub async fn create_viewport(&mut self, window: Arc<Window>) {
        let vp_desc = ViewportDesc::new(
            Arc::clone(&window),
            wgpu::Color {
                r: 224.0 / 255.0,
                g: 188.0 / 255.0,
                b: 223.0 / 255.0,
                a: 0.0,
            },
            self.instance.as_ref().unwrap(),
        );

        if self.adapter.is_none() {
            self.create_adapter(&vp_desc.surface).await;
        }
        if self.device.is_none() {
            self.create_device().await;
        }

        let vp = vp_desc.build(
            self.adapter.as_ref().unwrap(),
            self.device.as_ref().unwrap(),
        );

        self.viewports.insert(window.id(), vp);
    }

    pub async fn create_pipeline(
        &mut self,
        win_id: winit::window::WindowId,
        name: String,
        shader_src: impl AsRef<str>,
        vertex_entry_fn_name: impl AsRef<str>,
        fragment_entry_fn_name: impl AsRef<str>,
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'static>],
        bind_group_layout_descs: Vec<wgpu::BindGroupLayoutDescriptor<'static>>,
        topology: wgpu::PrimitiveTopology,
        depth_stencil_desc: Option<wgpu::DepthStencilState>,
    ) {
        let pipeline = SunPipeline::new(
            self.device.as_ref().unwrap(),
            self.viewports.get(&win_id).unwrap(),
            name.clone(),
            shader_src,
            vertex_entry_fn_name,
            fragment_entry_fn_name,
            vertex_buffer_layouts,
            bind_group_layout_descs,
            topology,
            depth_stencil_desc,
        );

        self.pipelines.insert(name, pipeline);

        if !state::initialized() {
            state::finish_init();
            info!("Initialized Render Engine!")
        }
    }

    pub async fn regenerate_buffers(&mut self, render_desc: &RenderFrameDesc) {
        if self.mvp_buffer.is_none() {
            let dummy_cam = CameraComponent::default();
            let dummy_transform = TransformComponent::zero();
            let dummy_cam_uniform =
                ModelUniform::from_camera_and_model_transform(&dummy_cam, &dummy_transform);

            self.mvp_buffer = Some(SunBuffer::new_with_data(
                "Camera Uniform Buffer",
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                bytemuck::cast_slice(&[dummy_cam_uniform]),
                self.device.as_ref().unwrap(),
            ));

            self.mvp_bindgroup = Some(
                self.device
                    .as_ref()
                    .unwrap()
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Camera Uniform Bind Group"),
                        layout: self
                            .pipelines
                            .get("basic_shader.wgsl")
                            .unwrap()
                            .bind_group_layouts
                            .get(0)
                            .unwrap(),
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self
                                .mvp_buffer
                                .as_ref()
                                .unwrap()
                                .get_buffer()
                                .as_entire_binding(),
                        }],
                    }),
            )
        }

        // We get the viewport so we can calculate Normalized device coordinates for the position of the character in 2D
        if let Some(vp) = self.viewports.get_mut(&render_desc.window_id) {
            // (Re)Generate text buffers only if the text has changed since last gen
            let indices = &render_desc.ui_desc.geometry.1;
            let vertex_data = &render_desc.ui_desc.geometry.0;
            for ui_data in vertex_data {
                let mut vertex_buffers = vec![];
                let mut index_buffer = None;

                let ui_data_array = ui_data.vertices.clone();

                if ui_data.changed || vp.changed {
                    for (data, z_index) in ui_data_array {
                        let vb = SunBuffer::new_with_data(
                            "quad_vb",
                            wgpu::BufferUsages::VERTEX,
                            bytemuck::cast_slice(&data),
                            self.device.as_ref().unwrap(),
                        );

                        vertex_buffers.push((vb, z_index));
                    }
                    // Create buffers with the quad vertex data
                    let ib = SunBuffer::new_with_data(
                        "quad_ib",
                        wgpu::BufferUsages::INDEX,
                        bytemuck::cast_slice(indices),
                        self.device.as_ref().unwrap(),
                    );
                    index_buffer = Some(ib);
                }

                // Reset vieport state
                if vp.changed {
                    vp.changed = false;
                }
                if !vertex_buffers.is_empty() {
                    vertex_buffers.sort_unstable_by(|a, b| a.1.cmp(&b.1));
                    self.vertex_buffers.insert(ui_data.id, vertex_buffers);
                }
                if index_buffer.is_some() {
                    self.index_buffers.insert(ui_data.id, index_buffer.unwrap());
                }
            }
        }
    }

    pub async fn redraw(&mut self, render_desc: RenderFrameDesc) {
        if !state::initialized() {
            return;
        }
        self.regenerate_buffers(&render_desc).await;

        let mut model_desc = render_desc.model_desc;
        model_desc
            .models
            .sort_by(|a, b| a.1.translation.z.total_cmp(&b.1.translation.z));

        let ui_desc = render_desc.ui_desc;

        // Get the viewport for the requested window
        if let Some(vp) = self.viewports.get_mut(&render_desc.window_id) {
            // Get the texture of the window to render to
            let Ok(frame) = vp.get_current_texture() else {
                error!(
                    "Could not get viewport texture of [{:?}]",
                    render_desc.window_id
                );
                return;
            };

            // Get the texture view of the surface
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Render models
            if let Some(mvp_bg) = &self.mvp_bindgroup {
                // Create command encoder for model render commands
                let mut encoder = self.device.as_ref().unwrap().create_command_encoder(
                    &wgpu::CommandEncoderDescriptor {
                        label: Some("model_render_commands"),
                    },
                );

                // TODO (@A40): Get pipeline from material!
                let model_pipeline = self.pipelines.get("basic_shader.wgsl");

                if let Some(pipe) = model_pipeline {
                    {
                        // Model Render Pass
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(vp.desc.background),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: &pipe.depth_texture.view,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(1.0),
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(0),
                                        store: wgpu::StoreOp::Store,
                                    }),
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                        rpass.set_pipeline(&pipe.pipeline);
                        for (model, transform) in model_desc.models.iter().rev() {
                            let mvp = ModelUniform::from_camera_and_model_transform(
                                &model_desc.active_camera,
                                &transform,
                            );

                            self.queue.as_ref().unwrap().write_buffer(
                                self.mvp_buffer.as_ref().unwrap().get_buffer(),
                                0,
                                bytemuck::cast_slice(&[mvp]),
                            );

                            // Draw model through the active camera
                            if let Some(model) = self.models.get(&model.model_path) {
                                rpass.draw_model(&model, mvp_bg);
                            }
                        }
                    }
                }
                // Submit all render commands to the command queue
                self.queue.as_ref().unwrap().submit(Some(encoder.finish()));
            }
            // Render UI
            // Create command encoder for model render commands
            let mut encoder = self.device.as_ref().unwrap().create_command_encoder(
                &wgpu::CommandEncoderDescriptor {
                    label: Some("text_render_commands"),
                },
            );
            {
                // TODO (@A40): Get pipeline from material!
                let text_pipeline = self.pipelines.get("text_shader.wgsl");
                let quad_pipeline = self.pipelines.get("quad_shader.wgsl");

                // UI Render Pass

                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                for ui_data in &ui_desc.geometry.0 {
                    let vertex_buffers = self.vertex_buffers.get(&ui_data.id);
                    let index_buffer = self.index_buffers.get(&ui_data.id);
                    match &ui_data.ui_type {
                        UIType::Container(_) => {
                            if let Some(pipe) = quad_pipeline {
                                rpass.set_pipeline(&pipe.pipeline);

                                if vertex_buffers.is_some() && index_buffer.is_some() {
                                    for (buf, _) in vertex_buffers.unwrap() {
                                        // TODO: Check for texture in quad data and set pipeline accordingly
                                        rpass.draw_colored_quad(buf, index_buffer.unwrap());
                                    }
                                }
                            }
                        }
                        UIType::Text(text) => {
                            if let Some(pipe) = text_pipeline {
                                rpass.set_pipeline(&pipe.pipeline);
                                let texture_bind_group = self.bind_groups.get(&text.font);

                                if vertex_buffers.is_some()
                                    && index_buffer.is_some()
                                    && texture_bind_group.is_some()
                                {
                                    for (buf, _) in vertex_buffers.unwrap() {
                                        rpass.draw_textured_quad(
                                            buf,
                                            index_buffer.unwrap(),
                                            texture_bind_group.unwrap(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Submit all render commands to the command queue
            self.queue.as_ref().unwrap().submit(Some(encoder.finish()));

            // Present to the texture executing the submitted commands?
            frame.present();
        }
    }
}

impl Default for Sun {
    fn default() -> Self {
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            dx12_shader_compiler: Default::default(),
            ..Default::default()
        });

        Self {
            instance: Some(instance),
            adapter: None,
            device: None,
            queue: None,

            viewports: HashMap::new(),
            pipelines: HashMap::new(),
            shaders: HashMap::new(),

            mvp_buffer: None,
            mvp_bindgroup: None,

            models: HashMap::new(),
            fonts: HashMap::new(),

            quad_instance_buffer: None,

            vertex_buffers: HashMap::new(),
            index_buffers: HashMap::new(),
            bind_groups: HashMap::new(),

            commands: vec![],

            proxy: None,

            frame_time: web_time::Instant::now(),
            time: web_time::Instant::now(),
        }
    }
}

#[async_trait(?Send)]
impl App for Sun {
    fn get_name(&self) -> String {
        "Renderer".into()
    }
    fn init(&mut self, elp: EventLoopProxy<CommandEvent>) {
        self.proxy = Some(elp.clone());

        let load_basic_shader = Command::new(
            "asset_server",
            CommandType::Get,
            Some("get shaders/basic_shader.wgsl shader".into()),
            None,
        );

        self.commands.append(&mut vec![load_basic_shader]);
    }

    async fn process_command(&mut self, _cmd: Command) {}

    async fn process_user_event(
        &mut self,
        event: &crate::core::events::CommandEvent,
        _delta_time: f32,
    ) {
        match event {
            CommandEvent::OnWindowCreated(window) => {
                self.create_viewport(Arc::clone(window)).await;
            }
            CommandEvent::OnWindowClosed((id, _)) => {
                self.viewports.remove(id);
            }

            CommandEvent::RenderFrame(render_desc) => {
                self.redraw(render_desc.clone()).await;
            }

            CommandEvent::RequestPipeline(pipe_desc) => {
                let pipe_desc = pipe_desc.clone();

                self.create_pipeline(
                    pipe_desc.win_id,
                    pipe_desc.name,
                    pipe_desc.shader_src,
                    pipe_desc.vertex_entry_fn_name,
                    pipe_desc.fragment_entry_fn_name,
                    &pipe_desc.vertex_buffer_layouts,
                    pipe_desc.bind_group_layout_desc,
                    pipe_desc.topology,
                    pipe_desc.depth_stencil_desc,
                )
                .await;
            }

            CommandEvent::Asset(asset) => {
                if asset.status == AssetStatus::NotFound {
                    return;
                }
                match asset.asset_type {
                    AssetType::Font => {
                        let font = SunFont::from_font_bytes(&asset.name, &asset.data).await;
                        match font {
                            Ok(font) => {
                                info!("Successfully created font: {}", font.font_file);

                                let font_copy = font.clone();
                                let task = Box::new(move || {
                                    vec![CommandEvent::SignalChange(
                                        ChangeComponentState::FontAtlas(font_copy.clone()),
                                    )]
                                });

                                let cmd = Command::new("sun", CommandType::Other, None, Some(task));
                                self.commands.push(cmd);

                                let Ok(font_atlas_texture) = SunTexture::from_image(
                                    &font.font_file,
                                    self.device.as_ref().unwrap(),
                                    self.queue.as_ref().unwrap(),
                                    font.atlas.image.clone(),
                                ) else {
                                    error!(
                                        "Failed to create texture from font image: {}",
                                        font.font_file
                                    );
                                    return;
                                };

                                let font_atlas_texture_bg_layout = &self
                                    .pipelines
                                    .get("text_shader.wgsl")
                                    .unwrap()
                                    .bind_group_layouts[0];

                                let font_bind_group =
                                    self.device.as_ref().unwrap().create_bind_group(
                                        &wgpu::BindGroupDescriptor {
                                            label: None,
                                            layout: font_atlas_texture_bg_layout,
                                            entries: &[
                                                wgpu::BindGroupEntry {
                                                    binding: 0,
                                                    resource: wgpu::BindingResource::TextureView(
                                                        &font_atlas_texture.view,
                                                    ),
                                                },
                                                wgpu::BindGroupEntry {
                                                    binding: 1,
                                                    resource: wgpu::BindingResource::Sampler(
                                                        &font_atlas_texture.sampler,
                                                    ),
                                                },
                                            ],
                                        },
                                    );
                                self.bind_groups
                                    .insert(font.font_file.clone(), font_bind_group);
                                self.fonts.insert(font.font_file.clone(), font);
                            }
                            Err(err) => {
                                error!("{err}");
                            }
                        }
                    }
                    AssetType::Shader => {
                        self.shaders.insert(asset.name.clone(), asset.clone());
                    }
                    AssetType::Model => {
                        let diffuse_texture_bg_layout = &self
                            .pipelines
                            .get("basic_shader.wgsl")
                            .unwrap()
                            .bind_group_layouts[1];

                        let model = SunModel::from_glb(
                            asset,
                            &diffuse_texture_bg_layout,
                            self.queue.as_ref().unwrap(),
                            self.device.as_ref().unwrap(),
                        );

                        match model {
                            Ok(model) => {
                                info!("Created Model: {:?}", asset.path);
                                self.models.insert(asset.path.clone(), model);
                            }
                            Err(err) => {
                                error!(
                                    "Failed to create model from Glb: {} with error: {}",
                                    asset.path, err
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    async fn process_window_event(
        &mut self,
        event: &winit::event::WindowEvent,
        window_id: winit::window::WindowId,
        _delta_time: f32,
    ) {
        match event {
            WindowEvent::Resized(new_size) => {
                // Recreate the swap chain with the new size
                if let Some(viewport) = self.viewports.get_mut(&window_id) {
                    {
                        viewport.resize(self.device.as_ref().unwrap(), new_size);
                        for pipeline in self.pipelines.values_mut() {
                            pipeline.depth_texture = SunTexture::create_depth_texture(
                                self.device.as_ref().unwrap(),
                                &viewport.config,
                                "depth_texture",
                            );
                        }
                    }
                    // On macos the window needs to be redrawn manually after resizing
                    viewport.desc.window.request_redraw();

                    let width = new_size.width as f32;
                    let height = new_size.height as f32;
                    let task = Box::new(move || {
                        vec![CommandEvent::SignalChange(ChangeComponentState::Window(
                            WindowContainer { width, height },
                        ))]
                    });

                    let cmd = Command::new("sun", CommandType::Other, None, Some(task));
                    self.commands.push(cmd);
                }
            }
            WindowEvent::CloseRequested => {
                self.viewports.remove(&window_id);
            }

            WindowEvent::RedrawRequested => {
                for (name, shader) in &self.shaders {
                    // Create model and text pipeline from a received shader
                    if !self.pipelines.contains_key(name) {
                        let basic_tex_bg_layout_desc = wgpu::BindGroupLayoutDescriptor {
                            label: Some("Diffuse Texture Bind Group"),
                            entries: &[
                                wgpu::BindGroupLayoutEntry {
                                    binding: 0,
                                    visibility: wgpu::ShaderStages::FRAGMENT,
                                    ty: wgpu::BindingType::Texture {
                                        sample_type: wgpu::TextureSampleType::Float {
                                            filterable: true,
                                        },
                                        view_dimension: wgpu::TextureViewDimension::D2,
                                        multisampled: false,
                                    },
                                    count: None,
                                },
                                wgpu::BindGroupLayoutEntry {
                                    binding: 1,
                                    visibility: wgpu::ShaderStages::FRAGMENT,
                                    // This should match the filterable field of the
                                    // corresponding Texture entry above.
                                    ty: wgpu::BindingType::Sampler(
                                        wgpu::SamplerBindingType::Filtering,
                                    ),
                                    count: None,
                                },
                            ],
                        };

                        let camera_bg_layout_desc = wgpu::BindGroupLayoutDescriptor {
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
                            label: Some("Camera Bind Group Layout"),
                        };

                        let stencil_state_front = wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Greater,
                            fail_op: wgpu::StencilOperation::IncrementClamp,
                            depth_fail_op: wgpu::StencilOperation::DecrementClamp,
                            pass_op: wgpu::StencilOperation::DecrementClamp,
                        };

                        let stencil_state_back = wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Always,
                            fail_op: wgpu::StencilOperation::DecrementClamp,
                            depth_fail_op: wgpu::StencilOperation::IncrementClamp,
                            pass_op: wgpu::StencilOperation::DecrementClamp,
                        };

                        let depth_stencil_desc = Some(wgpu::DepthStencilState {
                            format: wgpu::TextureFormat::Depth32FloatStencil8,
                            depth_write_enabled: true,
                            depth_compare: wgpu::CompareFunction::Less,
                            stencil: wgpu::StencilState {
                                front: stencil_state_front,
                                back: stencil_state_back,
                                read_mask: 0xff,
                                write_mask: 0xff,
                            },
                            bias: wgpu::DepthBiasState::default(),
                        });

                        // Model pipeline
                        let pipe_desc = PipelineDesc {
                            name: name.clone(),
                            win_id: window_id,
                            shader_src: String::from_utf8(shader.data.clone()).unwrap(),
                            vertex_entry_fn_name: "vs_main".to_string(),
                            fragment_entry_fn_name: "fs_main".to_string(),
                            vertex_buffer_layouts: vec![super::primitive::ModelVertex::desc()],
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            bind_group_layout_desc: vec![
                                camera_bg_layout_desc.clone(),
                                basic_tex_bg_layout_desc.clone(),
                            ],
                            bind_group_layout_name: vec!["camera".into(), "diffuse".into()],
                            depth_stencil_desc,
                        };

                        self.proxy
                            .as_ref()
                            .unwrap()
                            .send_event(CommandEvent::RequestPipeline(pipe_desc))
                            .unwrap();

                        // Text pipeline
                        let pipe_desc = PipelineDesc {
                            name: "text_shader.wgsl".to_string(),
                            win_id: window_id,
                            shader_src: String::from_utf8(shader.data.clone()).unwrap(),
                            vertex_entry_fn_name: "vs_text".to_string(),
                            fragment_entry_fn_name: "fs_text".to_string(),
                            vertex_buffer_layouts: vec![super::primitive::Quad2DVertex::desc()],
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            bind_group_layout_desc: vec![basic_tex_bg_layout_desc.clone()],
                            bind_group_layout_name: vec!["diffuse".into()],
                            depth_stencil_desc: None,
                        };

                        self.proxy
                            .as_ref()
                            .unwrap()
                            .send_event(CommandEvent::RequestPipeline(pipe_desc))
                            .unwrap();

                        // Colored Quad pipeline
                        let pipe_desc = PipelineDesc {
                            name: "quad_shader.wgsl".to_string(),
                            win_id: window_id,
                            shader_src: String::from_utf8(shader.data.clone()).unwrap(),
                            vertex_entry_fn_name: "vs_quad".to_string(),
                            fragment_entry_fn_name: "fs_colored_quad".to_string(),
                            vertex_buffer_layouts: vec![super::primitive::Quad2DVertex::desc()],
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            bind_group_layout_desc: vec![],
                            bind_group_layout_name: vec![],
                            depth_stencil_desc: None,
                        };

                        self.proxy
                            .as_ref()
                            .unwrap()
                            .send_event(CommandEvent::RequestPipeline(pipe_desc))
                            .unwrap();
                    }
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, _delta_time: f32) -> Vec<Command> {
        let renderer_time = self.time.elapsed().as_secs_f32();

        let frame_time = self.frame_time.elapsed().as_nanos() as f32 / 1_000_000.0;
        self.frame_time = web_time::Instant::now();

        if renderer_time > 0.01 {
            self.time = web_time::Instant::now();
            let text_changed = TextDesc {
                changed: true,
                text: format!("Render Time: {}ms", frame_time),
                ..Default::default()
            };

            let ui_changed = UIComponent {
                id: uuid::Uuid::new_v4(),
                string_id: "stats".to_string(),
                ui_type: UIType::Text(text_changed),
                visible: true,
            };

            let mut ui_changed_trans = TransformComponent::zero();
            ui_changed_trans.scale.x += 150.0;
            ui_changed_trans.scale.y += 150.0;

            let task = Box::new(move || {
                vec![CommandEvent::SignalChange(ChangeComponentState::UI((
                    ui_changed.clone(),
                    Some(ui_changed_trans),
                )))]
            });

            let cmd = Command::new("sun", CommandType::Other, None, Some(task));
            self.commands.push(cmd);
        }

        self.commands.drain(..).collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
pub struct ViewportDesc {
    window: Arc<Window>,
    background: wgpu::Color,
    surface: wgpu::Surface<'static>,
}

#[derive(Debug)]
pub struct Viewport {
    desc: ViewportDesc,
    pub config: wgpu::SurfaceConfiguration,
    changed: bool,
}

impl ViewportDesc {
    fn new(window: Arc<Window>, background: wgpu::Color, instance: &wgpu::Instance) -> Self {
        let surface = unsafe {
            instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_window(window.clone().as_ref()).unwrap(),
                )
                .unwrap()
        };
        Self {
            window,
            background,
            surface,
        }
    }

    fn build(self, adapter: &wgpu::Adapter, device: &wgpu::Device) -> Viewport {
        let size = self.window.inner_size();

        let caps = self.surface.get_capabilities(adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 3,
        };

        self.surface.configure(device, &config);

        Viewport {
            desc: self,
            config,
            changed: true,
        }
    }

    pub fn window(&self) -> Arc<Window> {
        self.window.clone()
    }
}

impl Viewport {
    fn resize(&mut self, device: &wgpu::Device, size: &winit::dpi::PhysicalSize<u32>) {
        if size.height != 0 && size.width != 0 {
            self.config.width = size.width;
            self.config.height = size.height;
            self.desc.surface.configure(device, &self.config);
            self.changed = true;
        }
    }
    fn get_current_texture(&mut self) -> anyhow::Result<wgpu::SurfaceTexture> {
        let surface_tex = self.desc.surface.get_current_texture()?;

        Ok(surface_tex)
    }

    pub fn get_surface(&self) -> &wgpu::Surface {
        &self.desc.surface
    }

    pub fn get_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub fn get_description(&self) -> &ViewportDesc {
        &self.desc
    }
}
