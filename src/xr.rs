use crate::Scene;
use ash::vk::{self, Handle};
use openxr as xr;
use std::ffi::{CString, c_void};
use web_time::Instant;

const VK_TARGET_VERSION: xr::Version = xr::Version::new(1, 1, 0);
const VK_TARGET_VERSION_ASH: u32 = vk::make_api_version(
    0,
    VK_TARGET_VERSION.major() as u32,
    VK_TARGET_VERSION.minor() as u32,
    VK_TARGET_VERSION.patch(),
);

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

pub struct XrContext {
    _vk_entry: ash::Entry,
    _vk_instance: ash::Instance,
    instance: xr::Instance,
    _system: xr::SystemId,
    session: xr::Session<xr::Vulkan>,
    frame_wait: xr::FrameWaiter,
    frame_stream: xr::FrameStream<xr::Vulkan>,
    stage: xr::Space,
    swapchain: xr::Swapchain<xr::Vulkan>,
    swapchain_buffers: Vec<wgpu::Texture>,
    resolution: (u32, u32),
    _views: Vec<xr::ViewConfigurationView>,
    action_set: xr::ActionSet,
    move_action: xr::Action<xr::Vector2f>,
    left_hand_action: xr::Action<xr::Posef>,
    right_hand_action: xr::Action<xr::Posef>,
    left_trigger_action: xr::Action<f32>,
    right_trigger_action: xr::Action<f32>,
    left_hand_space: xr::Space,
    right_hand_space: xr::Space,
    player_position: nalgebra_glm::Vec3,
    cube_vertex_buffer: wgpu::Buffer,
    cube_index_buffer: wgpu::Buffer,
    green_cube_vertex_buffer: wgpu::Buffer,
    grid_uniform_buffer: wgpu::Buffer,
    grid_bind_group: wgpu::BindGroup,
    grid_pipeline: wgpu::RenderPipeline,
    sky_uniform_buffer: wgpu::Buffer,
    sky_bind_group: wgpu::BindGroup,
    sky_pipeline: wgpu::RenderPipeline,
}

impl XrContext {
    pub fn new() -> Result<(Self, wgpu::Device, wgpu::Queue), Box<dyn std::error::Error>> {
        let xr_entry = xr::Entry::linked();

        let mut required_extensions = xr::ExtensionSet::default();
        required_extensions.khr_vulkan_enable2 = true;

        let xr_instance = xr_entry.create_instance(
            &xr::ApplicationInfo {
                application_name: "wgpu-xr-example",
                application_version: 1,
                engine_name: "wgpu",
                engine_version: 1,
                api_version: xr::Version::new(1, 0, 0),
            },
            &required_extensions,
            &[],
        )?;

        let system = xr_instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;

        let views = xr_instance.enumerate_view_configuration_views(
            system,
            xr::ViewConfigurationType::PRIMARY_STEREO,
        )?;

        let resolution = (
            views[0].recommended_image_rect_width,
            views[0].recommended_image_rect_height,
        );

        let reqs = xr_instance.graphics_requirements::<xr::Vulkan>(system)?;
        if VK_TARGET_VERSION < reqs.min_api_version_supported
            || VK_TARGET_VERSION.major() > reqs.max_api_version_supported.major()
        {
            return Err(format!(
                "OpenXR runtime requires Vulkan version > {}, < {}.0.0",
                reqs.min_api_version_supported,
                reqs.max_api_version_supported.major() + 1
            )
            .into());
        }

        let vk_entry = unsafe { ash::Entry::load()? };
        let flags = wgpu::InstanceFlags::default();
        let instance_exts = <wgpu_hal::vulkan::Api as wgpu_hal::Api>::Instance::desired_extensions(
            &vk_entry,
            VK_TARGET_VERSION_ASH,
            flags,
        )?;

        let vk_instance = unsafe {
            let extensions_cchar: Vec<_> = instance_exts.iter().map(|s| s.as_ptr()).collect();

            let app_name = CString::new("wgpu-xr-example")?;
            let vk_app_info = vk::ApplicationInfo::default()
                .application_name(&app_name)
                .application_version(1)
                .engine_name(&app_name)
                .engine_version(1)
                .api_version(VK_TARGET_VERSION_ASH);

            let vk_instance = xr_instance
                .create_vulkan_instance(
                    system,
                    std::mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
                    &vk::InstanceCreateInfo::default()
                        .application_info(&vk_app_info)
                        .enabled_extension_names(&extensions_cchar) as *const _
                        as *const _,
                )?
                .map_err(vk::Result::from_raw)?;

            ash::Instance::load(
                vk_entry.static_fn(),
                vk::Instance::from_raw(vk_instance as _),
            )
        };

        let vk_physical_device = vk::PhysicalDevice::from_raw(unsafe {
            xr_instance.vulkan_graphics_device(system, vk_instance.handle().as_raw() as _)? as _
        });

        let vk_instance_ptr = vk_instance.handle().as_raw() as *const c_void;
        let vk_physical_device_ptr = vk_physical_device.as_raw() as *const c_void;

        let vk_device_properties =
            unsafe { vk_instance.get_physical_device_properties(vk_physical_device) };

        if vk_device_properties.api_version < VK_TARGET_VERSION_ASH {
            return Err("Vulkan physical device doesn't support version 1.1".into());
        }

        let wgpu_vk_instance = unsafe {
            <wgpu_hal::vulkan::Api as wgpu_hal::Api>::Instance::from_raw(
                vk_entry.clone(),
                vk_instance.clone(),
                vk_device_properties.api_version,
                0,
                None,
                instance_exts,
                flags,
                wgpu::MemoryBudgetThresholds::default(),
                false,
                None,
            )?
        };

        let wgpu_exposed_adapter = wgpu_vk_instance
            .expose_adapter(vk_physical_device)
            .ok_or("Failed to expose adapter")?;

        let wgpu_features = wgpu_exposed_adapter.features;
        let enabled_extensions = wgpu_exposed_adapter
            .adapter
            .required_device_extensions(wgpu_features);

        let device_exts = vec![ash::khr::swapchain::NAME];

        let (wgpu_open_device, vk_device_ptr, queue_family_index) = {
            let extensions_cchar: Vec<_> = device_exts.iter().map(|s| s.as_ptr()).collect();
            let mut enabled_phd_features = wgpu_exposed_adapter
                .adapter
                .physical_device_features(&enabled_extensions, wgpu_features);
            let family_index = 0;
            let family_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(family_index)
                .queue_priorities(&[1.0]);
            let family_infos = [family_info];
            let mut physical_device_multiview_features = vk::PhysicalDeviceMultiviewFeatures {
                multiview: vk::TRUE,
                ..Default::default()
            };
            let info = enabled_phd_features
                .add_to_device_create(
                    vk::DeviceCreateInfo::default()
                        .queue_create_infos(&family_infos)
                        .push_next(&mut physical_device_multiview_features),
                )
                .enabled_extension_names(&extensions_cchar);

            let vk_device = unsafe {
                let vk_device = xr_instance
                    .create_vulkan_device(
                        system,
                        std::mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
                        vk_physical_device.as_raw() as _,
                        &info as *const _ as *const _,
                    )?
                    .map_err(vk::Result::from_raw)?;

                ash::Device::load(vk_instance.fp_v1_0(), vk::Device::from_raw(vk_device as _))
            };
            let vk_device_ptr = vk_device.handle().as_raw() as *const c_void;

            let wgpu_open_device = unsafe {
                wgpu_exposed_adapter.adapter.device_from_raw(
                    vk_device,
                    None,
                    &enabled_extensions,
                    wgpu_features,
                    &wgpu::MemoryHints::Performance,
                    family_info.queue_family_index,
                    0,
                )
            }?;

            (
                wgpu_open_device,
                vk_device_ptr,
                family_info.queue_family_index,
            )
        };

        let wgpu_instance =
            unsafe { wgpu::Instance::from_hal::<wgpu_hal::api::Vulkan>(wgpu_vk_instance) };
        let wgpu_adapter = unsafe { wgpu_instance.create_adapter_from_hal(wgpu_exposed_adapter) };
        let limits = wgpu_adapter.limits();
        let (wgpu_device, wgpu_queue) = unsafe {
            wgpu_adapter.create_device_from_hal(
                wgpu_open_device,
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu_features,
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::Performance,
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    trace: wgpu::Trace::Off,
                },
            )
        }?;

        let (session, frame_wait, frame_stream) = unsafe {
            xr_instance.create_session::<xr::Vulkan>(
                system,
                &xr::vulkan::SessionCreateInfo {
                    instance: vk_instance_ptr,
                    physical_device: vk_physical_device_ptr,
                    device: vk_device_ptr,
                    queue_family_index,
                    queue_index: 0,
                },
            )
        }?;

        let action_set = xr_instance.create_action_set("gameplay", "Gameplay Actions", 0)?;
        let move_action = action_set.create_action::<xr::Vector2f>("move", "Move", &[])?;

        let left_hand_action =
            action_set.create_action::<xr::Posef>("left_hand_pose", "Left Hand Pose", &[])?;

        let right_hand_action =
            action_set.create_action::<xr::Posef>("right_hand_pose", "Right Hand Pose", &[])?;

        let left_trigger_action =
            action_set.create_action::<f32>("left_trigger", "Left Trigger", &[])?;

        let right_trigger_action =
            action_set.create_action::<f32>("right_trigger", "Right Trigger", &[])?;

        xr_instance.suggest_interaction_profile_bindings(
            xr_instance.string_to_path("/interaction_profiles/oculus/touch_controller")?,
            &[
                xr::Binding::new(
                    &move_action,
                    xr_instance.string_to_path("/user/hand/left/input/thumbstick")?,
                ),
                xr::Binding::new(
                    &left_hand_action,
                    xr_instance.string_to_path("/user/hand/left/input/grip/pose")?,
                ),
                xr::Binding::new(
                    &right_hand_action,
                    xr_instance.string_to_path("/user/hand/right/input/grip/pose")?,
                ),
                xr::Binding::new(
                    &left_trigger_action,
                    xr_instance.string_to_path("/user/hand/left/input/trigger/value")?,
                ),
                xr::Binding::new(
                    &right_trigger_action,
                    xr_instance.string_to_path("/user/hand/right/input/trigger/value")?,
                ),
            ],
        )?;

        session.attach_action_sets(&[&action_set])?;

        let left_hand_space =
            left_hand_action.create_space(session.clone(), xr::Path::NULL, xr::Posef::IDENTITY)?;

        let right_hand_space =
            right_hand_action.create_space(session.clone(), xr::Path::NULL, xr::Posef::IDENTITY)?;

        let stage =
            session.create_reference_space(xr::ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)?;

        let swapchain = session.create_swapchain(&xr::SwapchainCreateInfo {
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                | xr::SwapchainUsageFlags::SAMPLED,
            format: vk::Format::R8G8B8A8_SRGB.as_raw() as _,
            sample_count: 1,
            width: resolution.0,
            height: resolution.1,
            face_count: 1,
            array_size: 2,
            mip_count: 1,
        })?;

        let swapchain_images = swapchain.enumerate_images()?;
        let swapchain_buffers: Vec<wgpu::Texture> = swapchain_images
            .into_iter()
            .map(|color_image| {
                let color_image = vk::Image::from_raw(color_image);
                let wgpu_hal_texture = unsafe {
                    let hal_dev = wgpu_device
                        .as_hal::<wgpu_hal::vulkan::Api>()
                        .ok_or("Failed to get HAL device")?;
                    hal_dev.texture_from_raw(
                        color_image,
                        &wgpu_hal::TextureDescriptor {
                            label: Some("VR Swapchain"),
                            size: wgpu::Extent3d {
                                width: resolution.0,
                                height: resolution.1,
                                depth_or_array_layers: 2,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUses::COLOR_TARGET | wgpu::TextureUses::COPY_DST,
                            memory_flags: wgpu_hal::MemoryFlags::empty(),
                            view_formats: vec![],
                        },
                        None,
                    )
                };
                let texture = unsafe {
                    wgpu_device.create_texture_from_hal::<wgpu_hal::vulkan::Api>(
                        wgpu_hal_texture,
                        &wgpu::TextureDescriptor {
                            label: Some("VR Swapchain"),
                            size: wgpu::Extent3d {
                                width: resolution.0,
                                height: resolution.1,
                                depth_or_array_layers: 2,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                | wgpu::TextureUsages::COPY_DST,
                            view_formats: &[],
                        },
                    )
                };
                Ok(texture)
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;

        let cube_vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &wgpu_device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Cube Vertex Buffer"),
                contents: bytemuck::cast_slice(&crate::CUBE_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let cube_index_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &wgpu_device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Cube Index Buffer"),
                contents: bytemuck::cast_slice(&crate::CUBE_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            },
        );

        let green_cube_vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &wgpu_device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Green Cube Vertex Buffer"),
                contents: bytemuck::cast_slice(&crate::GREEN_CUBE_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let grid_uniform_buffer = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Grid Uniform Buffer"),
            size: std::mem::size_of::<GridUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let grid_bind_group_layout =
            wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let grid_bind_group = wgpu_device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &grid_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: grid_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Grid Bind Group"),
        });

        let grid_shader = wgpu_device.create_shader_module(wgpu::include_wgsl!("grid.wgsl"));

        let grid_pipeline_layout =
            wgpu_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Grid Pipeline Layout"),
                bind_group_layouts: &[&grid_bind_group_layout],
                push_constant_ranges: &[],
            });

        let grid_pipeline = wgpu_device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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

        let sky_uniform_buffer = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sky Uniform Buffer"),
            size: std::mem::size_of::<SkyUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sky_bind_group_layout =
            wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let sky_bind_group = wgpu_device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &sky_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sky_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Sky Bind Group"),
        });

        let sky_shader = wgpu_device.create_shader_module(wgpu::include_wgsl!("sky.wgsl"));

        let sky_pipeline_layout =
            wgpu_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Sky Pipeline Layout"),
                bind_group_layouts: &[&sky_bind_group_layout],
                push_constant_ranges: &[],
            });

        let sky_pipeline = wgpu_device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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

        log::info!("OpenXR session created successfully");

        Ok((
            Self {
                _vk_entry: vk_entry,
                _vk_instance: vk_instance,
                instance: xr_instance,
                _system: system,
                session,
                frame_wait,
                frame_stream,
                stage,
                swapchain,
                swapchain_buffers,
                resolution,
                _views: views,
                action_set,
                move_action,
                left_hand_action,
                right_hand_action,
                left_trigger_action,
                right_trigger_action,
                left_hand_space,
                right_hand_space,
                player_position: nalgebra_glm::vec3(0.0, 0.0, 0.0),
                cube_vertex_buffer,
                cube_index_buffer,
                green_cube_vertex_buffer,
                grid_uniform_buffer,
                grid_bind_group,
                grid_pipeline,
                sky_uniform_buffer,
                sky_bind_group,
                sky_pipeline,
            },
            wgpu_device,
            wgpu_queue,
        ))
    }

    pub fn poll_events(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let mut event_buffer = xr::EventDataBuffer::new();
        while let Some(event) = self.instance.poll_event(&mut event_buffer)? {
            match event {
                xr::Event::SessionStateChanged(state_change) => {
                    log::info!("XR Session state changed to: {:?}", state_change.state());
                    match state_change.state() {
                        xr::SessionState::READY => {
                            self.session
                                .begin(xr::ViewConfigurationType::PRIMARY_STEREO)?;
                            log::info!("XR Session started");
                        }
                        xr::SessionState::STOPPING => {
                            self.session.end()?;
                            log::info!("XR Session ended");
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            log::info!("XR Session exiting");
                            return Ok(false);
                        }
                        _ => {}
                    }
                }
                xr::Event::InstanceLossPending(_) => {
                    log::info!("XR Instance loss pending");
                    return Ok(false);
                }
                _ => {}
            }
        }
        Ok(true)
    }

    pub fn wait_frame(&mut self) -> Result<xr::FrameState, Box<dyn std::error::Error>> {
        Ok(self.frame_wait.wait()?)
    }

    pub fn update_movement(
        &mut self,
        delta_time: f32,
        predicted_display_time: xr::Time,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.session.sync_actions(&[(&self.action_set).into()])?;

        let move_state = self.move_action.state(&self.session, xr::Path::NULL)?;

        if move_state.current_state.x.abs() > 0.1 || move_state.current_state.y.abs() > 0.1 {
            let (_, views) = self.session.locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                predicted_display_time,
                &self.stage,
            )?;

            if !views.is_empty() {
                let head_pose = &views[0].pose;
                let head_quat = nalgebra_glm::quat(
                    head_pose.orientation.w,
                    head_pose.orientation.z,
                    head_pose.orientation.y,
                    head_pose.orientation.x,
                );
                let head_forward =
                    nalgebra_glm::quat_rotate_vec3(&head_quat, &nalgebra_glm::vec3(0.0, 0.0, -1.0));
                let head_yaw = (-head_forward.x).atan2(-head_forward.z);

                let move_speed = 2.0;
                let move_x = move_state.current_state.x;
                let move_z = -move_state.current_state.y;

                let rotated_x = move_x * head_yaw.cos() - move_z * head_yaw.sin();
                let rotated_z = move_x * head_yaw.sin() + move_z * head_yaw.cos();

                self.player_position.x += rotated_x * move_speed * delta_time;
                self.player_position.z += rotated_z * move_speed * delta_time;
            }
        }

        Ok(())
    }

    pub fn render_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &mut Scene,
        _delta_time: f32,
        frame_state: xr::FrameState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.frame_stream.begin()?;

        if !frame_state.should_render {
            self.frame_stream.end(
                frame_state.predicted_display_time,
                xr::EnvironmentBlendMode::OPAQUE,
                &[],
            )?;
            return Ok(());
        }

        let (view_state_flags, views) = self.session.locate_views(
            xr::ViewConfigurationType::PRIMARY_STEREO,
            frame_state.predicted_display_time,
            &self.stage,
        )?;

        if !view_state_flags
            .contains(xr::ViewStateFlags::POSITION_VALID | xr::ViewStateFlags::ORIENTATION_VALID)
        {
            self.frame_stream.end(
                frame_state.predicted_display_time,
                xr::EnvironmentBlendMode::OPAQUE,
                &[],
            )?;
            return Ok(());
        }

        let image_index = self.swapchain.acquire_image()?;
        self.swapchain.wait_image(xr::Duration::INFINITE)?;

        let swapchain_texture = &self.swapchain_buffers[image_index as usize];
        let resolution = self.resolution;

        for (view_index, view) in views.iter().enumerate() {
            let pose = &view.pose;
            let fov = &view.fov;

            let view_matrix = {
                let rotation = {
                    let o = pose.orientation;
                    let flip_x = nalgebra_glm::quat_angle_axis(
                        180.0_f32.to_radians(),
                        &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                    );
                    let openxr_quat = nalgebra_glm::quat(o.w, o.z, o.y, o.x);
                    flip_x * openxr_quat
                };

                let translation =
                    nalgebra_glm::vec3(-pose.position.x, pose.position.y, -pose.position.z);

                let eye = translation + self.player_position;
                let target = eye
                    + nalgebra_glm::quat_rotate_vec3(&rotation, &nalgebra_glm::vec3(0.0, 0.0, 1.0));
                let up =
                    nalgebra_glm::quat_rotate_vec3(&rotation, &nalgebra_glm::vec3(0.0, 1.0, 0.0));

                nalgebra_glm::look_at_rh(&eye, &target, &up)
            };

            let projection_matrix = {
                let tan_left = fov.angle_left.tan();
                let tan_right = fov.angle_right.tan();
                let tan_up = fov.angle_up.tan();
                let tan_down = fov.angle_down.tan();

                let near = 0.1_f32;
                let far = 1000.0_f32;

                let tan_width = tan_right - tan_left;
                let tan_height = tan_up - tan_down;

                let a11 = 2.0 / tan_width;
                let a22 = 2.0 / tan_height;
                let a31 = (tan_right + tan_left) / tan_width;
                let a32 = (tan_up + tan_down) / tan_height;
                let a33 = -far / (far - near);
                let a43 = -(far * near) / (far - near);

                let mut proj = nalgebra_glm::Mat4::zeros();
                proj[(0, 0)] = a11;
                proj[(1, 1)] = a22;
                proj[(0, 2)] = a31;
                proj[(1, 2)] = a32;
                proj[(2, 2)] = a33;
                proj[(2, 3)] = a43;
                proj[(3, 2)] = -1.0;

                proj
            };

            let view_texture_view = swapchain_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("XR View {}", view_index)),
                format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: view_index as u32,
                array_layer_count: Some(1),
                usage: None,
            });

            let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("XR Depth Texture"),
                size: wgpu::Extent3d {
                    width: resolution.0,
                    height: resolution.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

            let camera_position = {
                let pose = &view.pose;
                let translation =
                    nalgebra_glm::vec3(-pose.position.x, pose.position.y, -pose.position.z);
                translation + self.player_position
            };

            let sky_uniform = SkyUniform {
                proj: projection_matrix.into(),
                proj_inv: nalgebra_glm::inverse(&projection_matrix).into(),
                view: view_matrix.into(),
                cam_pos: [camera_position.x, camera_position.y, camera_position.z, 1.0],
            };
            queue.write_buffer(
                &self.sky_uniform_buffer,
                0,
                bytemuck::cast_slice(&[sky_uniform]),
            );

            let mut sky_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Sky Render Encoder"),
            });

            {
                let mut render_pass = sky_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Sky Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view_texture_view,
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

            queue.submit(std::iter::once(sky_encoder.finish()));

            let model_translation = nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 1.5, 2.0));
            let model = model_translation * scene.model;
            let triangle_mvp = projection_matrix * view_matrix * model;
            scene
                .uniform
                .update_buffer(queue, 0, crate::UniformBuffer { mvp: triangle_mvp });

            let mut triangle_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Triangle Render Encoder"),
                });

            {
                let mut render_pass =
                    triangle_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Triangle Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view_texture_view,
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

                scene.render(&mut render_pass);
            }

            queue.submit(std::iter::once(triangle_encoder.finish()));

            let grid_uniform = GridUniform {
                view_proj: (projection_matrix * view_matrix).into(),
                camera_world_pos: [camera_position.x, camera_position.y, camera_position.z],
                grid_size: 100.0,
                grid_min_pixels: 2.0,
                grid_cell_size: 0.025,
                orthographic_scale: 1.0,
                is_orthographic: 0.0,
            };
            queue.write_buffer(
                &self.grid_uniform_buffer,
                0,
                bytemuck::cast_slice(&[grid_uniform]),
            );

            let mut grid_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Grid Render Encoder"),
            });

            {
                let mut render_pass = grid_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Grid Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view_texture_view,
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

            queue.submit(std::iter::once(grid_encoder.finish()));

            let left_hand_location = self
                .left_hand_space
                .locate(&self.stage, frame_state.predicted_display_time);
            if let Ok(location) = left_hand_location {
                if location.location_flags.contains(
                    xr::SpaceLocationFlags::POSITION_VALID
                        | xr::SpaceLocationFlags::ORIENTATION_VALID,
                ) {
                    let hand_pose = location.pose;
                    let rotation = {
                        let o = hand_pose.orientation;
                        let flip_x = nalgebra_glm::quat_angle_axis(
                            180.0_f32.to_radians(),
                            &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                        );
                        let openxr_quat = nalgebra_glm::quat(o.w, o.z, o.y, o.x);
                        flip_x * openxr_quat
                    };
                    let translation = nalgebra_glm::vec3(
                        -hand_pose.position.x,
                        hand_pose.position.y,
                        -hand_pose.position.z,
                    );
                    let hand_world_position = translation + self.player_position;

                    let rotation_matrix = nalgebra_glm::quat_to_mat4(&rotation);
                    let translation_matrix = nalgebra_glm::translation(&hand_world_position);
                    let hand_model = translation_matrix * rotation_matrix;

                    let left_hand_mvp = projection_matrix * view_matrix * hand_model;
                    scene.uniform.update_buffer(
                        queue,
                        0,
                        crate::UniformBuffer { mvp: left_hand_mvp },
                    );

                    let left_trigger_state = self
                        .left_trigger_action
                        .state(&self.session, xr::Path::NULL)
                        .ok();
                    let left_trigger_pulled = left_trigger_state
                        .map(|s| s.current_state > 0.5)
                        .unwrap_or(false);
                    let left_cube_buffer = if left_trigger_pulled {
                        &self.green_cube_vertex_buffer
                    } else {
                        &self.cube_vertex_buffer
                    };

                    let mut left_hand_encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Left Hand Encoder"),
                        });

                    {
                        let mut render_pass =
                            left_hand_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Left Hand Render Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view_texture_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    },
                                    depth_slice: None,
                                })],
                                depth_stencil_attachment: Some(
                                    wgpu::RenderPassDepthStencilAttachment {
                                        view: &depth_view,
                                        depth_ops: Some(wgpu::Operations {
                                            load: wgpu::LoadOp::Load,
                                            store: wgpu::StoreOp::Store,
                                        }),
                                        stencil_ops: None,
                                    },
                                ),
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });

                        render_pass.set_pipeline(&scene.pipeline);
                        render_pass.set_bind_group(0, &scene.uniform.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, left_cube_buffer.slice(..));
                        render_pass.set_index_buffer(
                            self.cube_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.draw_indexed(0..36, 0, 0..1);
                    }

                    queue.submit(std::iter::once(left_hand_encoder.finish()));
                }
            }

            let right_hand_location = self
                .right_hand_space
                .locate(&self.stage, frame_state.predicted_display_time);
            if let Ok(location) = right_hand_location {
                if location.location_flags.contains(
                    xr::SpaceLocationFlags::POSITION_VALID
                        | xr::SpaceLocationFlags::ORIENTATION_VALID,
                ) {
                    let hand_pose = location.pose;
                    let rotation = {
                        let o = hand_pose.orientation;
                        let flip_x = nalgebra_glm::quat_angle_axis(
                            180.0_f32.to_radians(),
                            &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                        );
                        let openxr_quat = nalgebra_glm::quat(o.w, o.z, o.y, o.x);
                        flip_x * openxr_quat
                    };
                    let translation = nalgebra_glm::vec3(
                        -hand_pose.position.x,
                        hand_pose.position.y,
                        -hand_pose.position.z,
                    );
                    let hand_world_position = translation + self.player_position;

                    let rotation_matrix = nalgebra_glm::quat_to_mat4(&rotation);
                    let translation_matrix = nalgebra_glm::translation(&hand_world_position);
                    let hand_model = translation_matrix * rotation_matrix;

                    let right_hand_mvp = projection_matrix * view_matrix * hand_model;
                    scene.uniform.update_buffer(
                        queue,
                        0,
                        crate::UniformBuffer {
                            mvp: right_hand_mvp,
                        },
                    );

                    let right_trigger_state = self
                        .right_trigger_action
                        .state(&self.session, xr::Path::NULL)
                        .ok();
                    let right_trigger_pulled = right_trigger_state
                        .map(|s| s.current_state > 0.5)
                        .unwrap_or(false);
                    let right_cube_buffer = if right_trigger_pulled {
                        &self.green_cube_vertex_buffer
                    } else {
                        &self.cube_vertex_buffer
                    };

                    let mut right_hand_encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Right Hand Encoder"),
                        });

                    {
                        let mut render_pass =
                            right_hand_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Right Hand Render Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view_texture_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    },
                                    depth_slice: None,
                                })],
                                depth_stencil_attachment: Some(
                                    wgpu::RenderPassDepthStencilAttachment {
                                        view: &depth_view,
                                        depth_ops: Some(wgpu::Operations {
                                            load: wgpu::LoadOp::Load,
                                            store: wgpu::StoreOp::Store,
                                        }),
                                        stencil_ops: None,
                                    },
                                ),
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });

                        render_pass.set_pipeline(&scene.pipeline);
                        render_pass.set_bind_group(0, &scene.uniform.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, right_cube_buffer.slice(..));
                        render_pass.set_index_buffer(
                            self.cube_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.draw_indexed(0..36, 0, 0..1);
                    }

                    queue.submit(std::iter::once(right_hand_encoder.finish()));
                }
            }
        }

        self.swapchain.release_image()?;

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: resolution.0 as i32,
                height: resolution.1 as i32,
            },
        };

        let sub_images: Vec<_> = views
            .iter()
            .enumerate()
            .map(|(view_index, view)| {
                xr::CompositionLayerProjectionView::new()
                    .pose(view.pose)
                    .fov(view.fov)
                    .sub_image(
                        xr::SwapchainSubImage::new()
                            .swapchain(&self.swapchain)
                            .image_array_index(view_index as u32)
                            .image_rect(rect),
                    )
            })
            .collect();

        let projection_layer = xr::CompositionLayerProjection::new()
            .space(&self.stage)
            .views(&sub_images);

        self.frame_stream.end(
            frame_state.predicted_display_time,
            xr::EnvironmentBlendMode::OPAQUE,
            &[&projection_layer],
        )?;

        Ok(())
    }
}

pub fn run_xr() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::info!("Initializing OpenXR mode");

    let (mut xr_context, device, queue) = XrContext::new()?;
    let mut scene = Scene::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let mut last_render_time = Instant::now();

    log::info!("Starting XR render loop");

    loop {
        if !xr_context.poll_events()? {
            log::info!("XR session ended, exiting");
            break;
        }

        let now = Instant::now();
        let delta_time = (now - last_render_time).as_secs_f32();
        last_render_time = now;

        scene.model = nalgebra_glm::rotate(
            &scene.model,
            30_f32.to_radians() * delta_time,
            &nalgebra_glm::Vec3::y(),
        );

        let frame_state = xr_context.wait_frame()?;
        xr_context.update_movement(delta_time, frame_state.predicted_display_time)?;

        xr_context.render_frame(&device, &queue, &mut scene, delta_time, frame_state)?;
    }

    Ok(())
}
