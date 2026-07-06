use std::iter;
use std::sync::Arc;

use nalgebra_glm as glm;
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;

const LIGHT_POSITION: [f32; 3] = [-5.5, 6.5, 3.5];
const LIGHT_RADIUS: f32 = 1.0;
const LIGHT_INTENSITY: f32 = 1.5;
const MAX_BOUNCES: f32 = 8.0;
const SAMPLES_PER_FRAME: f32 = 6.0;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos_refl: [f32; 4],
    normal: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_inverse: glm::Mat4,
    proj_inverse: glm::Mat4,
    light_pos: [f32; 4],
    params: [f32; 4],
    frame: [u32; 4],
}

struct Camera {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            yaw: 0.9,
            pitch: 0.36,
            distance: 12.0,
        }
    }

    fn eye(&self) -> glm::Vec3 {
        let center = glm::vec3(0.0, 1.0, 0.0);
        let cos_pitch = self.pitch.cos();
        let sin_pitch = self.pitch.sin();
        let cos_yaw = self.yaw.cos();
        let sin_yaw = self.yaw.sin();
        glm::vec3(
            center.x + self.distance * cos_pitch * cos_yaw,
            center.y + self.distance * sin_pitch,
            center.z + self.distance * cos_pitch * sin_yaw,
        )
    }
}

fn look_at_rh(eye: glm::Vec3, center: glm::Vec3, up: glm::Vec3) -> glm::Mat4 {
    let forward = glm::normalize(&(center - eye));
    let side = glm::normalize(&glm::cross(&forward, &up));
    let real_up = glm::cross(&side, &forward);
    glm::Mat4::from_column_slice(&[
        side.x,
        real_up.x,
        -forward.x,
        0.0,
        side.y,
        real_up.y,
        -forward.y,
        0.0,
        side.z,
        real_up.z,
        -forward.z,
        0.0,
        -glm::dot(&side, &eye),
        -glm::dot(&real_up, &eye),
        glm::dot(&forward, &eye),
        1.0,
    ])
}

fn perspective_vk(fov_y: f32, aspect: f32, z_near: f32, z_far: f32) -> glm::Mat4 {
    let tangent = (fov_y * 0.5).tan();
    glm::Mat4::from_column_slice(&[
        1.0 / (aspect * tangent),
        0.0,
        0.0,
        0.0,
        0.0,
        -1.0 / tangent,
        0.0,
        0.0,
        0.0,
        0.0,
        z_far / (z_near - z_far),
        -1.0,
        0.0,
        0.0,
        -(z_far * z_near) / (z_far - z_near),
        0.0,
    ])
}

struct SceneData {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

fn build_scene() -> SceneData {
    let mut scene = SceneData {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    add_floor(&mut scene, 25.0, 0.4);

    add_box(
        &mut scene,
        [-1.4, 1.25, 0.6],
        [1.25, 1.25, 1.25],
        0.4,
        [1.0, 1.0, 1.0],
        0.0,
        2.0,
    );

    add_box(
        &mut scene,
        [-4.2, 2.4, -3.2],
        [0.55, 2.4, 0.55],
        0.0,
        [0.75, 0.78, 0.82],
        0.7,
        1.0,
    );

    add_torus(
        &mut scene,
        [2.7, 1.55, -0.4],
        1.3,
        0.42,
        [0.15, 0.72, 0.70],
        0.55,
        1.0,
    );

    add_sphere(
        &mut scene,
        [3.5, 0.85, 1.9],
        0.85,
        [1.0, 1.0, 1.0],
        0.0,
        2.0,
    );

    add_box(
        &mut scene,
        [0.9, 0.5, 2.7],
        [0.5, 0.5, 0.5],
        0.7,
        [0.95, 0.4, 0.3],
        0.1,
        1.0,
    );
    add_box(
        &mut scene,
        [4.5, 0.62, -2.6],
        [0.62, 0.62, 0.62],
        0.2,
        [0.96, 0.76, 0.2],
        0.1,
        1.0,
    );

    add_sphere(
        &mut scene,
        [-3.7, 0.6, 1.5],
        0.6,
        [0.55, 0.3, 0.86],
        0.5,
        1.0,
    );
    add_sphere(
        &mut scene,
        [1.7, 2.7, -2.2],
        0.45,
        [0.92, 0.3, 0.62],
        0.5,
        1.0,
    );

    add_sphere(
        &mut scene,
        LIGHT_POSITION,
        LIGHT_RADIUS,
        [1.0, 0.90, 0.72],
        0.0,
        3.0,
    );
    scene
}

fn add_floor(scene: &mut SceneData, half: f32, reflectivity: f32) {
    let base = scene.vertices.len() as u32;
    let normal = [0.0, 1.0, 0.0, 0.0];
    let color = [0.8, 0.8, 0.8, 0.0];
    let corners = [
        [-half, 0.0, -half],
        [half, 0.0, -half],
        [half, 0.0, half],
        [-half, 0.0, half],
    ];
    for corner in corners {
        scene.vertices.push(Vertex {
            pos_refl: [corner[0], corner[1], corner[2], reflectivity],
            normal: [normal[0], normal[1], normal[2], 0.0],
            color,
        });
    }
    scene
        .indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn add_sphere(
    scene: &mut SceneData,
    center: [f32; 3],
    radius: f32,
    color: [f32; 3],
    reflectivity: f32,
    mat_id: f32,
) {
    let stacks = 24;
    let slices = 48;
    let base = scene.vertices.len() as u32;

    for stack in 0..=stacks {
        let v = stack as f32 / stacks as f32;
        let phi = v * std::f32::consts::PI;
        for slice in 0..=slices {
            let u = slice as f32 / slices as f32;
            let theta = u * 2.0 * std::f32::consts::PI;
            let normal = [phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()];
            let position = [
                center[0] + normal[0] * radius,
                center[1] + normal[1] * radius,
                center[2] + normal[2] * radius,
            ];
            scene.vertices.push(Vertex {
                pos_refl: [position[0], position[1], position[2], reflectivity],
                normal: [normal[0], normal[1], normal[2], mat_id],
                color: [color[0], color[1], color[2], 0.0],
            });
        }
    }

    let ring = slices + 1;
    for stack in 0..stacks {
        for slice in 0..slices {
            let a = base + (stack * ring + slice) as u32;
            let b = base + ((stack + 1) * ring + slice) as u32;
            scene
                .indices
                .extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
}

fn rotate_y(vector: [f32; 3], yaw: f32) -> [f32; 3] {
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();
    [
        vector[0] * cos_yaw + vector[2] * sin_yaw,
        vector[1],
        -vector[0] * sin_yaw + vector[2] * cos_yaw,
    ]
}

fn add_box(
    scene: &mut SceneData,
    center: [f32; 3],
    half: [f32; 3],
    yaw: f32,
    color: [f32; 3],
    reflectivity: f32,
    mat_id: f32,
) {
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        (
            [1.0, 0.0, 0.0],
            [
                [1.0, -1.0, -1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, -1.0],
            ],
        ),
        (
            [-1.0, 0.0, 0.0],
            [
                [-1.0, -1.0, 1.0],
                [-1.0, -1.0, -1.0],
                [-1.0, 1.0, -1.0],
                [-1.0, 1.0, 1.0],
            ],
        ),
        (
            [0.0, 1.0, 0.0],
            [
                [-1.0, 1.0, -1.0],
                [1.0, 1.0, -1.0],
                [1.0, 1.0, 1.0],
                [-1.0, 1.0, 1.0],
            ],
        ),
        (
            [0.0, -1.0, 0.0],
            [
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, -1.0, -1.0],
                [-1.0, -1.0, -1.0],
            ],
        ),
        (
            [0.0, 0.0, 1.0],
            [
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0],
                [-1.0, 1.0, 1.0],
            ],
        ),
        (
            [0.0, 0.0, -1.0],
            [
                [1.0, -1.0, -1.0],
                [-1.0, -1.0, -1.0],
                [-1.0, 1.0, -1.0],
                [1.0, 1.0, -1.0],
            ],
        ),
    ];

    for (normal, corners) in faces {
        let base = scene.vertices.len() as u32;
        let world_normal = rotate_y(normal, yaw);
        for corner in corners {
            let local = [
                corner[0] * half[0],
                corner[1] * half[1],
                corner[2] * half[2],
            ];
            let rotated = rotate_y(local, yaw);
            let position = [
                center[0] + rotated[0],
                center[1] + rotated[1],
                center[2] + rotated[2],
            ];
            scene.vertices.push(Vertex {
                pos_refl: [position[0], position[1], position[2], reflectivity],
                normal: [world_normal[0], world_normal[1], world_normal[2], mat_id],
                color: [color[0], color[1], color[2], 0.0],
            });
        }
        scene
            .indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

fn add_torus(
    scene: &mut SceneData,
    center: [f32; 3],
    major_radius: f32,
    minor_radius: f32,
    color: [f32; 3],
    reflectivity: f32,
    mat_id: f32,
) {
    let major_segments = 48;
    let minor_segments = 24;
    let base = scene.vertices.len() as u32;

    for major in 0..=major_segments {
        let u = major as f32 / major_segments as f32 * 2.0 * std::f32::consts::PI;
        let cos_u = u.cos();
        let sin_u = u.sin();
        for minor in 0..=minor_segments {
            let v = minor as f32 / minor_segments as f32 * 2.0 * std::f32::consts::PI;
            let cos_v = v.cos();
            let sin_v = v.sin();
            let normal = [cos_v * cos_u, cos_v * sin_u, sin_v];
            let ring = major_radius + minor_radius * cos_v;
            let position = [
                center[0] + ring * cos_u,
                center[1] + ring * sin_u,
                center[2] + minor_radius * sin_v,
            ];
            scene.vertices.push(Vertex {
                pos_refl: [position[0], position[1], position[2], reflectivity],
                normal: [normal[0], normal[1], normal[2], mat_id],
                color: [color[0], color[1], color[2], 0.0],
            });
        }
    }

    let ring = minor_segments + 1;
    for major in 0..major_segments {
        for minor in 0..minor_segments {
            let a = base + (major * ring + minor) as u32;
            let b = base + ((major + 1) * ring + minor) as u32;
            scene
                .indices
                .extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
}

struct RayTracer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,

    uniform_buffer: wgpu::Buffer,
    accum_buffer: wgpu::Buffer,
    output_view: wgpu::TextureView,

    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    compute_bind_group: wgpu::BindGroup,

    blit_pipeline: wgpu::RenderPipeline,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    blit_bind_group: wgpu::BindGroup,

    tlas: wgpu::Tlas,
    #[allow(dead_code)]
    blas: wgpu::Blas,
    #[allow(dead_code)]
    vertex_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    index_buffer: wgpu::Buffer,

    camera: Camera,
    camera_dirty: bool,
    accum_frame: u32,
    total_frame: u32,
    start_time: Instant,
}

impl RayTracer {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to request adapter!");

        if !adapter
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            panic!(
                "The selected GPU/driver does not support hardware ray tracing \
                 (wgpu::Features::EXPERIMENTAL_RAY_QUERY). A ray-tracing capable GPU \
                 with current drivers is required."
            );
        }

        log::info!("Ray tracing on: {}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Ray Tracing Device"),
                required_features: wgpu::Features::EXPERIMENTAL_RAY_QUERY,
                required_limits: wgpu::Limits::default()
                    .using_resolution(adapter.limits())
                    .using_minimum_supported_acceleration_structure_values(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to request a device!");

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);
        let present_mode = if surface_capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Fifo)
        {
            wgpu::PresentMode::Fifo
        } else {
            surface_capabilities.present_modes[0]
        };

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let scene = build_scene();
        let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Scene Vertices"),
                contents: bytemuck::cast_slice(&scene.vertices),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::BLAS_INPUT,
            },
        );
        let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Scene Indices"),
                contents: bytemuck::cast_slice(&scene.indices),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::BLAS_INPUT,
            },
        );

        let (blas, tlas) = Self::build_acceleration_structures(
            &device,
            &queue,
            &vertex_buffer,
            &index_buffer,
            &scene,
        );

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let accum_buffer = Self::create_accum_buffer(&device, width, height);
        let output_view = Self::create_output_texture(&device, width, height);

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Ray Tracing Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::AccelerationStructure {
                            vertex_return: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let compute_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ray Tracing Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "raytrace.wgsl"
            ))),
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ray Tracing Pipeline Layout"),
                bind_group_layouts: &[Some(&compute_bind_group_layout)],
                immediate_size: 0,
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Ray Tracing Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let compute_bind_group = Self::create_compute_bind_group(
            &device,
            &compute_bind_group_layout,
            &tlas,
            &output_view,
            &uniform_buffer,
            &vertex_buffer,
            &index_buffer,
            &accum_buffer,
        );

        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Blit Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });

        let blit_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("blit.wgsl"))),
        });

        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[Some(&blit_bind_group_layout)],
            immediate_size: 0,
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_module,
                entry_point: Some("vertex_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_module,
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        let blit_bind_group =
            Self::create_blit_bind_group(&device, &blit_bind_group_layout, &output_view);

        Self {
            surface,
            device,
            queue,
            surface_config,
            uniform_buffer,
            accum_buffer,
            output_view,
            compute_pipeline,
            compute_bind_group_layout,
            compute_bind_group,
            blit_pipeline,
            blit_bind_group_layout,
            blit_bind_group,
            tlas,
            blas,
            vertex_buffer,
            index_buffer,
            camera: Camera::new(),
            camera_dirty: true,
            accum_frame: 0,
            total_frame: 0,
            start_time: Instant::now(),
        }
    }

    fn build_acceleration_structures(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertex_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
        scene: &SceneData,
    ) -> (wgpu::Blas, wgpu::Tlas) {
        let size_descriptor = wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count: scene.vertices.len() as u32,
            index_format: Some(wgpu::IndexFormat::Uint32),
            index_count: Some(scene.indices.len() as u32),
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        };

        let blas = device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: Some("Scene BLAS"),
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![size_descriptor.clone()],
            },
        );

        let mut tlas = device.create_tlas(&wgpu::CreateTlasDescriptor {
            label: Some("Scene TLAS"),
            max_instances: 1,
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        });

        tlas[0] = Some(wgpu::TlasInstance::new(
            &blas,
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            0,
            0xFF,
        ));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Acceleration Structure Build"),
        });
        encoder.build_acceleration_structures(
            iter::once(&wgpu::BlasBuildEntry {
                blas: &blas,
                geometry: wgpu::BlasGeometries::TriangleGeometries(vec![
                    wgpu::BlasTriangleGeometry {
                        size: &size_descriptor,
                        vertex_buffer,
                        first_vertex: 0,
                        vertex_stride: std::mem::size_of::<Vertex>() as u64,
                        index_buffer: Some(index_buffer),
                        first_index: Some(0),
                        transform_buffer: None,
                        transform_buffer_offset: None,
                    },
                ]),
            }),
            iter::once(&tlas),
        );
        queue.submit(iter::once(encoder.finish()));

        (blas, tlas)
    }

    fn create_accum_buffer(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Accumulation Buffer"),
            size: (width as u64) * (height as u64) * 16,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    fn create_output_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Ray Tracing Output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    #[allow(clippy::too_many_arguments)]
    fn create_compute_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        tlas: &wgpu::Tlas,
        output_view: &wgpu::TextureView,
        uniform_buffer: &wgpu::Buffer,
        vertex_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
        accum_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ray Tracing Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tlas.as_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: index_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: accum_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn create_blit_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        output_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(output_view),
            }],
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);

        self.accum_buffer = Self::create_accum_buffer(&self.device, width, height);
        self.output_view = Self::create_output_texture(&self.device, width, height);
        self.compute_bind_group = Self::create_compute_bind_group(
            &self.device,
            &self.compute_bind_group_layout,
            &self.tlas,
            &self.output_view,
            &self.uniform_buffer,
            &self.vertex_buffer,
            &self.index_buffer,
            &self.accum_buffer,
        );
        self.blit_bind_group = Self::create_blit_bind_group(
            &self.device,
            &self.blit_bind_group_layout,
            &self.output_view,
        );
        self.camera_dirty = true;
    }

    fn update_uniforms(&mut self) {
        if self.camera_dirty {
            self.accum_frame = 0;
            self.camera_dirty = false;
        } else {
            self.accum_frame += 1;
        }

        let aspect = self.surface_config.width as f32 / self.surface_config.height.max(1) as f32;
        let view = look_at_rh(
            self.camera.eye(),
            glm::vec3(0.0, 1.0, 0.0),
            glm::vec3(0.0, 1.0, 0.0),
        );
        let projection = perspective_vk(60_f32.to_radians(), aspect, 0.1, 100.0);

        let time = self.start_time.elapsed().as_secs_f32();
        let uniforms = Uniforms {
            view_inverse: glm::inverse(&view),
            proj_inverse: glm::inverse(&projection),
            light_pos: [
                LIGHT_POSITION[0],
                LIGHT_POSITION[1],
                LIGHT_POSITION[2],
                LIGHT_RADIUS,
            ],
            params: [time, MAX_BOUNCES, LIGHT_INTENSITY, SAMPLES_PER_FRAME],
            frame: [self.accum_frame, self.total_frame, 0, 0],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        self.total_frame += 1;
    }

    fn render(&mut self) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                return;
            }
            other => panic!("Failed to acquire surface texture: {other:?}"),
        };

        self.update_uniforms();

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Frame Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Ray Tracing Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups_x = self.surface_config.width.div_ceil(8);
            let workgroups_y = self.surface_config.height.div_ceil(8);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blit Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
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
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.blit_pipeline);
            render_pass.set_bind_group(0, &self.blit_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        surface_texture.present();
    }
}

#[derive(Default)]
struct RayTracingApp {
    window: Option<Arc<Window>>,
    ray_tracer: Option<RayTracer>,
    dragging: bool,
    last_cursor: Option<PhysicalPosition<f64>>,
}

impl ApplicationHandler for RayTracingApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Wgpu Hardware Ray Tracing")
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
            .with_resizable(false);
        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        self.window = Some(window.clone());
        let ray_tracer = pollster::block_on(RayTracer::new(window.clone()));
        self.ray_tracer = Some(ray_tracer);
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let (Some(window), Some(ray_tracer)) = (self.window.as_ref(), self.ray_tracer.as_mut())
        else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                ray_tracer.resize(width, height);
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.dragging = state == ElementState::Pressed;
                if !self.dragging {
                    self.last_cursor = None;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.dragging {
                    if let Some(last) = self.last_cursor {
                        let delta_x = (position.x - last.x) as f32;
                        let delta_y = (position.y - last.y) as f32;
                        ray_tracer.camera.yaw += delta_x * 0.005;
                        ray_tracer.camera.pitch += delta_y * 0.005;
                        let limit = 1.5;
                        ray_tracer.camera.pitch = ray_tracer.camera.pitch.clamp(-limit, limit);
                        ray_tracer.camera_dirty = true;
                    }
                    self.last_cursor = Some(position);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32 / 120.0,
                };
                ray_tracer.camera.distance *= 0.9_f32.powf(scroll);
                ray_tracer.camera.distance = ray_tracer.camera.distance.clamp(3.0, 30.0);
                ray_tracer.camera_dirty = true;
            }
            WindowEvent::RedrawRequested => {
                ray_tracer.render();
                window.request_redraw();
            }
            _ => {}
        }
    }
}

pub fn run_raytracing() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::builder().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = RayTracingApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
