use crate::Scene;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{XrReferenceSpaceType, XrSession, XrSessionMode, XrWebGlLayer};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GridUniform {
    view_proj: [[f32; 4]; 4],
    camera_world_pos: [f32; 3],
    grid_size: f32,
    grid_min_pixels: f32,
    grid_cell_size: f32,
    orthographic_scale: f32,
    is_orthographic: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyUniform {
    proj: [[f32; 4]; 4],
    proj_inv: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    cam_pos: [f32; 4],
}

struct WebXrContext {
    session: XrSession,
    reference_space: web_sys::XrReferenceSpace,
    gl_layer: XrWebGlLayer,
    gl_context: web_sys::WebGl2RenderingContext,
    device: wgpu::Device,
    queue: wgpu::Queue,
    scene: Scene,
    cube_vertex_buffer: wgpu::Buffer,
    cube_index_buffer: wgpu::Buffer,
    green_cube_vertex_buffer: wgpu::Buffer,
    grid_uniform_buffer: wgpu::Buffer,
    grid_bind_group: wgpu::BindGroup,
    grid_pipeline: wgpu::RenderPipeline,
    sky_uniform_buffer: wgpu::Buffer,
    sky_bind_group: wgpu::BindGroup,
    sky_pipeline: wgpu::RenderPipeline,
    player_position: nalgebra_glm::Vec3,
    left_controller_position: Option<nalgebra_glm::Vec3>,
    right_controller_position: Option<nalgebra_glm::Vec3>,
    left_trigger_value: f32,
    right_trigger_value: f32,
    cached_render_texture: Option<(u32, u32, wgpu::Texture)>,
    cached_depth_texture: Option<(u32, u32, wgpu::Texture)>,
}

impl WebXrContext {
    async fn new(
        session: XrSession,
        reference_space: web_sys::XrReferenceSpace,
        gl_layer: XrWebGlLayer,
        gl_context: web_sys::WebGl2RenderingContext,
    ) -> Result<Self, JsValue> {
        log::info!("Creating wgpu instance with GL backend...");
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        log::info!("Requesting wgpu adapter...");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| {
                log::error!("Failed to get wgpu adapter: {:?}", e);
                JsValue::from_str(&format!("Failed to get GPU adapter: {:?}", e))
            })?;

        log::info!("wgpu adapter acquired");

        log::info!("Requesting wgpu device...");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("WebXR Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| {
                log::error!("Device request failed: {:?}", e);
                JsValue::from_str(&format!("GPU device creation failed: {:?}", e))
            })?;

        log::info!("wgpu device acquired, creating scene...");
        let scene = Scene::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
        log::info!("Scene created");

        let cube_vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Cube Vertex Buffer"),
                contents: bytemuck::cast_slice(&crate::CUBE_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let cube_index_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Cube Index Buffer"),
                contents: bytemuck::cast_slice(&crate::CUBE_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            },
        );

        let green_cube_vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Green Cube Vertex Buffer"),
                contents: bytemuck::cast_slice(&crate::GREEN_CUBE_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let grid_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Grid Uniform Buffer"),
            size: std::mem::size_of::<GridUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let grid_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Grid Bind Group Layout"),
            });

        let grid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &grid_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: grid_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Grid Bind Group"),
        });

        let grid_shader = device.create_shader_module(wgpu::include_wgsl!("grid.wgsl"));

        let grid_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[&grid_bind_group_layout],
            push_constant_ranges: &[],
        });

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Grid Pipeline"),
            layout: Some(&grid_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &grid_shader,
                entry_point: Some("vertex_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &grid_shader,
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sky_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sky Uniform Buffer"),
            size: std::mem::size_of::<SkyUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sky_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Sky Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let sky_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &sky_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sky_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Sky Bind Group"),
        });

        let sky_shader = device.create_shader_module(wgpu::include_wgsl!("sky.wgsl"));

        let sky_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sky Pipeline Layout"),
            bind_group_layouts: &[&sky_bind_group_layout],
            push_constant_ranges: &[],
        });

        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sky Pipeline"),
            layout: Some(&sky_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sky_shader,
                entry_point: Some("vs_sky"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sky_shader,
                entry_point: Some("fs_sky"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        log::info!("WebXR context initialized successfully");

        Ok(Self {
            session,
            reference_space,
            gl_layer,
            gl_context,
            device,
            queue,
            scene,
            cube_vertex_buffer,
            cube_index_buffer,
            green_cube_vertex_buffer,
            grid_uniform_buffer,
            grid_bind_group,
            grid_pipeline,
            sky_uniform_buffer,
            sky_bind_group,
            sky_pipeline,
            player_position: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            left_controller_position: None,
            right_controller_position: None,
            left_trigger_value: 0.0,
            right_trigger_value: 0.0,
            cached_render_texture: None,
            cached_depth_texture: None,
        })
    }

    fn render_frame(&mut self, frame: &web_sys::XrFrame) -> Result<(), JsValue> {
        let pose = frame
            .get_viewer_pose(&self.reference_space)
            .ok_or_else(|| JsValue::from_str("No viewer pose"))?;

        let views = pose.views();

        self.update_input_sources(frame)?;

        self.scene.model = nalgebra_glm::rotate(
            &self.scene.model,
            0.5_f32.to_radians(),
            &nalgebra_glm::Vec3::y(),
        );

        for view_index in 0..views.length() {
            let view_js = views.get(view_index);
            let view: web_sys::XrView = view_js.dyn_into()?;

            let projection_matrix = self.get_projection_matrix(&view);
            let view_matrix = self.get_view_matrix(&view);

            let viewport = self
                .gl_layer
                .get_viewport(&view)
                .ok_or_else(|| JsValue::from_str("No viewport"))?;

            self.render_view(&viewport, &projection_matrix, &view_matrix)?;
        }

        Ok(())
    }

    fn get_projection_matrix(&self, view: &web_sys::XrView) -> nalgebra_glm::Mat4 {
        let proj_array = view.projection_matrix();
        let mut values = [0.0f32; 16];

        for index in 0..16_usize {
            values[index] = proj_array[index];
        }

        nalgebra_glm::Mat4::from_column_slice(&values)
    }

    fn get_view_matrix(&self, view: &web_sys::XrView) -> nalgebra_glm::Mat4 {
        let transform = view.transform();
        let position = transform.position();
        let orientation = transform.orientation();

        let rotation = nalgebra_glm::quat(
            orientation.w() as f32,
            orientation.x() as f32,
            orientation.y() as f32,
            orientation.z() as f32,
        );

        let translation = nalgebra_glm::vec3(
            -position.x() as f32,
            position.y() as f32,
            -position.z() as f32,
        );

        let eye = translation + self.player_position;
        let target =
            eye + nalgebra_glm::quat_rotate_vec3(&rotation, &nalgebra_glm::vec3(0.0, 0.0, 1.0));
        let up = nalgebra_glm::quat_rotate_vec3(&rotation, &nalgebra_glm::vec3(0.0, 1.0, 0.0));

        nalgebra_glm::look_at_rh(&eye, &target, &up)
    }

    fn render_view(
        &mut self,
        viewport: &web_sys::XrViewport,
        projection_matrix: &nalgebra_glm::Mat4,
        view_matrix: &nalgebra_glm::Mat4,
    ) -> Result<(), JsValue> {
        let width = viewport.width() as u32;
        let height = viewport.height() as u32;
        let viewport_x = viewport.x() as i32;
        let viewport_y = viewport.y() as i32;

        let framebuffer = self.gl_layer.framebuffer();
        if framebuffer.is_none() {
            return Err(JsValue::from_str("No WebXR framebuffer available"));
        }

        let gl = &self.gl_context;

        gl.viewport(viewport_x, viewport_y, width as i32, height as i32);
        gl.scissor(viewport_x, viewport_y, width as i32, height as i32);
        gl.enable(web_sys::WebGl2RenderingContext::SCISSOR_TEST);

        let needs_new_render_texture = self
            .cached_render_texture
            .as_ref()
            .map_or(true, |(w, h, _)| *w != width || *h != height);

        if needs_new_render_texture {
            let new_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("WebXR Render Target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            self.cached_render_texture = Some((width, height, new_texture));
        }

        let view_texture = self
            .cached_render_texture
            .as_ref()
            .unwrap()
            .2
            .create_view(&wgpu::TextureViewDescriptor::default());

        let needs_new_depth_texture = self
            .cached_depth_texture
            .as_ref()
            .map_or(true, |(w, h, _)| *w != width || *h != height);

        if needs_new_depth_texture {
            let new_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("WebXR Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.cached_depth_texture = Some((width, height, new_texture));
        }

        let depth_view = self
            .cached_depth_texture
            .as_ref()
            .unwrap()
            .2
            .create_view(&wgpu::TextureViewDescriptor::default());

        let camera_position = nalgebra_glm::vec3(0.0, 1.7, 0.0) + self.player_position;

        let sky_uniform = SkyUniform {
            proj: (*projection_matrix).into(),
            proj_inv: nalgebra_glm::inverse(projection_matrix).into(),
            view: (*view_matrix).into(),
            cam_pos: [camera_position.x, camera_position.y, camera_position.z, 1.0],
        };
        self.queue.write_buffer(
            &self.sky_uniform_buffer,
            0,
            bytemuck::cast_slice(&[sky_uniform]),
        );

        let mut sky_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Sky Render Encoder"),
            });

        {
            let mut render_pass = sky_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sky Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.sky_pipeline);
            render_pass.set_bind_group(0, &self.sky_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(sky_encoder.finish()));

        let model_translation = nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 1.5, 2.0));
        let model = model_translation * self.scene.model;
        let triangle_mvp = projection_matrix * view_matrix * model;
        self.scene.uniform.update_buffer(
            &self.queue,
            0,
            crate::UniformBuffer { mvp: triangle_mvp },
        );

        let mut triangle_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Triangle Render Encoder"),
                });

        {
            let mut render_pass = triangle_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Triangle Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.scene.render(&mut render_pass);
        }

        self.queue
            .submit(std::iter::once(triangle_encoder.finish()));

        let grid_uniform = GridUniform {
            view_proj: (projection_matrix * view_matrix).into(),
            camera_world_pos: [camera_position.x, camera_position.y, camera_position.z],
            grid_size: 100.0,
            grid_min_pixels: 2.0,
            grid_cell_size: 0.025,
            orthographic_scale: 1.0,
            is_orthographic: 0.0,
        };
        self.queue.write_buffer(
            &self.grid_uniform_buffer,
            0,
            bytemuck::cast_slice(&[grid_uniform]),
        );

        let mut grid_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Grid Render Encoder"),
                });

        {
            let mut render_pass = grid_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Grid Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.grid_pipeline);
            render_pass.set_bind_group(0, &self.grid_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(grid_encoder.finish()));

        if let Some(left_pos) = self.left_controller_position {
            self.render_controller_cube(
                &view_texture,
                &depth_view,
                projection_matrix,
                view_matrix,
                &left_pos,
                self.left_trigger_value > 0.5,
            );
        }

        if let Some(right_pos) = self.right_controller_position {
            self.render_controller_cube(
                &view_texture,
                &depth_view,
                projection_matrix,
                view_matrix,
                &right_pos,
                self.right_trigger_value > 0.5,
            );
        }

        self.queue.submit(std::iter::empty());

        let gl = &self.gl_context;
        let xr_framebuffer = self.gl_layer.framebuffer();

        if let Some(fb) = xr_framebuffer.as_ref() {
            gl.bind_framebuffer(web_sys::WebGl2RenderingContext::DRAW_FRAMEBUFFER, Some(fb));

            let texture_ref = &self.cached_render_texture.as_ref().unwrap().2;
            let hal_texture = unsafe {
                texture_ref
                    .as_hal::<wgpu_hal::gles::Api>()
                    .ok_or_else(|| JsValue::from_str("Failed to get HAL texture"))?
            };

            let gl_texture = match &(*hal_texture).inner {
                wgpu_hal::gles::TextureInner::Texture { raw, .. } => raw,
                wgpu_hal::gles::TextureInner::Renderbuffer { .. } => {
                    return Err(JsValue::from_str("Cannot blit from renderbuffer"));
                }
                wgpu_hal::gles::TextureInner::DefaultRenderbuffer => {
                    return Err(JsValue::from_str("Cannot blit from default renderbuffer"));
                }
                wgpu_hal::gles::TextureInner::ExternalFramebuffer { .. } => {
                    return Err(JsValue::from_str("Cannot blit from external framebuffer"));
                }
            };

            let temp_fbo = gl
                .create_framebuffer()
                .ok_or_else(|| JsValue::from_str("Failed to create temporary framebuffer"))?;
            gl.bind_framebuffer(
                web_sys::WebGl2RenderingContext::READ_FRAMEBUFFER,
                Some(&temp_fbo),
            );

            let gl_texture_target = web_sys::WebGl2RenderingContext::TEXTURE_2D;

            let web_gl_texture = unsafe {
                let texture_ref: &web_sys::WebGlTexture = std::mem::transmute(gl_texture);
                texture_ref
            };

            gl.framebuffer_texture_2d(
                web_sys::WebGl2RenderingContext::READ_FRAMEBUFFER,
                web_sys::WebGl2RenderingContext::COLOR_ATTACHMENT0,
                gl_texture_target,
                Some(web_gl_texture),
                0,
            );

            gl.blit_framebuffer(
                0,
                0,
                width as i32,
                height as i32,
                viewport_x,
                viewport_y,
                viewport_x + width as i32,
                viewport_y + height as i32,
                web_sys::WebGl2RenderingContext::COLOR_BUFFER_BIT,
                web_sys::WebGl2RenderingContext::LINEAR,
            );

            gl.bind_framebuffer(web_sys::WebGl2RenderingContext::READ_FRAMEBUFFER, None);
            gl.delete_framebuffer(Some(&temp_fbo));
        }

        gl.disable(web_sys::WebGl2RenderingContext::SCISSOR_TEST);

        Ok(())
    }

    fn render_controller_cube(
        &mut self,
        view_texture: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        projection_matrix: &nalgebra_glm::Mat4,
        view_matrix: &nalgebra_glm::Mat4,
        controller_position: &nalgebra_glm::Vec3,
        trigger_pulled: bool,
    ) {
        let translation_matrix = nalgebra_glm::translation(controller_position);
        let hand_mvp = projection_matrix * view_matrix * translation_matrix;
        self.scene
            .uniform
            .update_buffer(&self.queue, 0, crate::UniformBuffer { mvp: hand_mvp });

        let cube_buffer = if trigger_pulled {
            &self.green_cube_vertex_buffer
        } else {
            &self.cube_vertex_buffer
        };

        let mut hand_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Hand Encoder"),
                });

        {
            let mut render_pass = hand_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Hand Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: view_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.scene.pipeline);
            render_pass.set_bind_group(0, &self.scene.uniform.bind_group, &[]);
            render_pass.set_vertex_buffer(0, cube_buffer.slice(..));
            render_pass
                .set_index_buffer(self.cube_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..36, 0, 0..1);
        }

        self.queue.submit(std::iter::once(hand_encoder.finish()));
    }

    fn update_input_sources(&mut self, frame: &web_sys::XrFrame) -> Result<(), JsValue> {
        let input_sources = self.session.input_sources();

        for index in 0..input_sources.length() {
            if let Some(input_source_js) = input_sources.get(index) {
                let input_source: web_sys::XrInputSource = input_source_js.dyn_into()?;

                if let Some(grip_space) = input_source.grip_space() {
                    if let Some(pose) = frame.get_pose(&grip_space, &self.reference_space) {
                        let transform = pose.transform();
                        let position = transform.position();

                        let controller_pos = nalgebra_glm::vec3(
                            position.x() as f32,
                            position.y() as f32,
                            position.z() as f32,
                        );

                        match input_source.handedness() {
                            web_sys::XrHandedness::Left => {
                                self.left_controller_position = Some(controller_pos);
                            }
                            web_sys::XrHandedness::Right => {
                                self.right_controller_position = Some(controller_pos);
                            }
                            _ => {}
                        }
                    }
                }

                if let Some(gamepad) = input_source.gamepad() {
                    let buttons = gamepad.buttons();
                    if buttons.length() > 0 {
                        let trigger_button_js = buttons.get(0);
                        if let Ok(trigger_button) =
                            trigger_button_js.dyn_into::<web_sys::GamepadButton>()
                        {
                            let trigger_value = trigger_button.value();

                            match input_source.handedness() {
                                web_sys::XrHandedness::Left => {
                                    self.left_trigger_value = trigger_value as f32;
                                }
                                web_sys::XrHandedness::Right => {
                                    self.right_trigger_value = trigger_value as f32;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

pub async fn init_webxr() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    log::info!("=== WebXR Initialization Started ===");

    log::info!("Getting window and navigator...");
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let navigator = window.navigator();
    let xr = navigator.xr();

    log::info!("XR object obtained, checking support...");

    let session_mode = XrSessionMode::ImmersiveVr;

    let supported_promise = xr.is_session_supported(session_mode);
    let supported = wasm_bindgen_futures::JsFuture::from(supported_promise)
        .await?
        .as_bool()
        .ok_or_else(|| JsValue::from_str("is_session_supported returned non-boolean"))?;

    if !supported {
        return Err(JsValue::from_str("WebXR immersive-vr not supported"));
    }

    log::info!("WebXR immersive-vr is supported");

    log::info!("Requesting XR session (must be from user gesture)...");
    let session_init = web_sys::XrSessionInit::new();

    let session_promise = xr.request_session_with_options(session_mode, &session_init);
    let session_result = wasm_bindgen_futures::JsFuture::from(session_promise).await;

    let session: XrSession = match session_result {
        Ok(session_value) => {
            log::info!("XR session promise resolved");
            session_value.dyn_into().map_err(|e| {
                log::error!("Failed to cast session object: {:?}", e);
                JsValue::from_str("Session object is not an XrSession")
            })?
        }
        Err(e) => {
            log::error!("XR session request failed: {:?}", e);
            return Err(JsValue::from_str(&format!(
                "Failed to create XR session. This may be due to: \
                1) Not being called from a user gesture (button click) \
                2) WebXR not being enabled in browser settings \
                3) No VR device connected. Error: {:?}",
                e
            )));
        }
    };

    log::info!("WebXR session created successfully");

    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("No document"))?;
    let canvas = document
        .get_element_by_id("canvas")
        .ok_or_else(|| JsValue::from_str("No canvas element"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let gl_context = {
        use wasm_bindgen::JsCast;

        let context_options = js_sys::Object::new();
        js_sys::Reflect::set(&context_options, &"xrCompatible".into(), &true.into())?;

        let context_result = canvas.get_context_with_context_options("webgl2", &context_options)?;

        if let Some(context) = context_result {
            log::info!("Got WebGL2 context with xrCompatible");
            context.dyn_into::<web_sys::WebGl2RenderingContext>()?
        } else {
            log::warn!("Failed to get xrCompatible context, trying fallback...");
            let fallback_context = canvas
                .get_context("webgl2")?
                .ok_or_else(|| JsValue::from_str("Failed to get WebGL2 context"))?
                .dyn_into::<web_sys::WebGl2RenderingContext>()?;

            log::info!("Making existing context XR compatible...");
            let make_xr_compatible = fallback_context.make_xr_compatible();
            wasm_bindgen_futures::JsFuture::from(make_xr_compatible).await?;

            fallback_context
        }
    };

    log::info!("Requesting reference space (local-floor)...");
    let reference_space_promise = session.request_reference_space(XrReferenceSpaceType::LocalFloor);
    let reference_space: web_sys::XrReferenceSpace =
        wasm_bindgen_futures::JsFuture::from(reference_space_promise)
            .await
            .map_err(|e| {
                log::error!("Failed to get reference space: {:?}", e);
                JsValue::from_str("Failed to get local-floor reference space")
            })?
            .dyn_into()?;

    log::info!("Reference space acquired");

    log::info!("Creating XR WebGL layer...");
    let xr_layer_init = web_sys::XrWebGlLayerInit::new();
    let xr_layer = XrWebGlLayer::new_with_web_gl2_rendering_context_and_layer_init(
        &session,
        &gl_context,
        &xr_layer_init,
    )
    .map_err(|e| {
        log::error!("Failed to create XR WebGL layer: {:?}", e);
        e
    })?;

    log::info!("XR WebGL layer created");

    log::info!("Updating render state...");
    let render_state_init = web_sys::XrRenderStateInit::new();
    render_state_init.set_base_layer(Some(&xr_layer));
    session.update_render_state_with_state(&render_state_init);

    log::info!("WebXR render state configured");

    log::info!("Creating WebXR context...");
    let context = Rc::new(RefCell::new(
        WebXrContext::new(session.clone(), reference_space, xr_layer, gl_context).await?,
    ));
    log::info!("WebXR context created successfully");

    let animation_frame_closure: Rc<RefCell<Option<Closure<dyn FnMut(f64, web_sys::XrFrame)>>>> =
        Rc::new(RefCell::new(None));
    let animation_frame_closure_clone = animation_frame_closure.clone();

    let session_rc = Rc::new(session);
    let session_clone = session_rc.clone();
    let context_clone = context.clone();

    *animation_frame_closure.borrow_mut() =
        Some(Closure::new(move |_time: f64, frame: web_sys::XrFrame| {
            if let Ok(mut ctx) = context_clone.try_borrow_mut() {
                if let Err(e) = ctx.render_frame(&frame) {
                    log::error!("Render frame error: {:?}", e);
                    return;
                }
            }

            if let Some(closure) = animation_frame_closure_clone.borrow().as_ref() {
                session_clone.request_animation_frame(closure.as_ref().unchecked_ref());
            }
        }));

    if let Some(closure) = animation_frame_closure.borrow().as_ref() {
        session_rc.request_animation_frame(closure.as_ref().unchecked_ref());
    }

    log::info!("WebXR frame loop started");

    Ok(())
}
