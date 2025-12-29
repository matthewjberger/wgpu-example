use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{
    Document, HtmlButtonElement, HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer,
    WebGlProgram, WebGlShader, WebGlUniformLocation, Window, XrFrame, XrReferenceSpace,
    XrReferenceSpaceType, XrRenderStateInit, XrSession, XrSessionMode, XrWebGlLayer,
};

struct WebXrState {
    session: Option<XrSession>,
    reference_space: Option<XrReferenceSpace>,
    gl_layer: Option<XrWebGlLayer>,
    gl_context: Option<WebGl2RenderingContext>,
    program: Option<WebGlProgram>,
    vertex_buffer: Option<WebGlBuffer>,
    mvp_location: Option<WebGlUniformLocation>,
    position_attrib_location: u32,
    color_attrib_location: u32,
    model_rotation: f32,
    last_time: Option<f64>,
}

fn get_window() -> Window {
    web_sys::window().expect("No global window exists")
}

fn get_document() -> Document {
    get_window()
        .document()
        .expect("Window should have a document")
}

fn get_canvas() -> HtmlCanvasElement {
    get_document()
        .get_element_by_id("canvas")
        .expect("Canvas element not found")
        .dyn_into::<HtmlCanvasElement>()
        .expect("Element is not a canvas")
}

pub fn initialize_webxr() {
    let window = get_window();
    let navigator = window.navigator();
    let xr_system = navigator.xr();

    let check_support = async move {
        let supported = wasm_bindgen_futures::JsFuture::from(
            xr_system.is_session_supported(XrSessionMode::ImmersiveVr),
        )
        .await;

        match supported {
            Ok(value) => {
                if value.as_bool().unwrap_or(false) {
                    log::info!("WebXR immersive-vr is supported");
                    create_enter_vr_button();
                } else {
                    log::warn!("WebXR immersive-vr is not supported on this device/browser");
                }
            }
            Err(error) => {
                log::error!("Failed to check WebXR support: {:?}", error);
            }
        }
    };

    wasm_bindgen_futures::spawn_local(check_support);
}

fn create_enter_vr_button() {
    let document = get_document();

    let button = document
        .create_element("button")
        .expect("Failed to create button")
        .dyn_into::<HtmlButtonElement>()
        .expect("Element is not a button");

    button.set_id("enter-vr-button");
    button.set_inner_text("Enter VR");

    let style = button.style();
    style.set_property("position", "fixed").ok();
    style.set_property("bottom", "20px").ok();
    style.set_property("left", "50%").ok();
    style.set_property("transform", "translateX(-50%)").ok();
    style.set_property("padding", "15px 30px").ok();
    style.set_property("font-size", "18px").ok();
    style.set_property("font-weight", "bold").ok();
    style.set_property("background-color", "#4CAF50").ok();
    style.set_property("color", "white").ok();
    style.set_property("border", "none").ok();
    style.set_property("border-radius", "8px").ok();
    style.set_property("cursor", "pointer").ok();
    style.set_property("z-index", "1000").ok();
    style
        .set_property("box-shadow", "0 4px 6px rgba(0,0,0,0.3)")
        .ok();

    let onclick = Closure::wrap(Box::new(move || {
        start_xr_session();
    }) as Box<dyn Fn()>);

    button.set_onclick(Some(onclick.as_ref().unchecked_ref()));
    onclick.forget();

    let body = document.body().expect("Document should have a body");
    body.append_child(&button)
        .expect("Failed to append button to body");

    log::info!("Enter VR button created");
}

fn start_xr_session() {
    let window = get_window();
    let navigator = window.navigator();
    let xr_system = navigator.xr();

    let session_init = web_sys::XrSessionInit::new();

    let session_promise =
        xr_system.request_session_with_options(XrSessionMode::ImmersiveVr, &session_init);

    let future = async move {
        match wasm_bindgen_futures::JsFuture::from(session_promise).await {
            Ok(session_value) => {
                let session: XrSession = session_value.dyn_into().expect("Expected XrSession");
                log::info!("WebXR session started");

                if let Some(button) = get_document().get_element_by_id("enter-vr-button")
                    && let Some(b) = button.dyn_ref::<HtmlButtonElement>()
                {
                    b.set_disabled(true);
                }

                setup_xr_rendering(session).await;
            }
            Err(error) => {
                log::error!("Failed to start WebXR session: {:?}", error);
            }
        }
    };

    wasm_bindgen_futures::spawn_local(future);
}

fn compile_shader(
    gl: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| "Unable to create shader object".to_string())?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "Unknown error creating shader".to_string()))
    }
}

fn link_program(
    gl: &WebGl2RenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = gl
        .create_program()
        .ok_or_else(|| "Unable to create shader object".to_string())?;

    gl.attach_shader(&program, vert_shader);
    gl.attach_shader(&program, frag_shader);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| "Unknown error creating program object".to_string()))
    }
}

struct GlResources {
    program: WebGlProgram,
    vertex_buffer: WebGlBuffer,
    mvp_location: WebGlUniformLocation,
    position_attrib_location: u32,
    color_attrib_location: u32,
}

fn setup_gl_resources(gl: &WebGl2RenderingContext) -> GlResources {
    let vert_shader = compile_shader(gl, WebGl2RenderingContext::VERTEX_SHADER, VERTEX_SHADER)
        .expect("Failed to compile vertex shader");
    let frag_shader = compile_shader(gl, WebGl2RenderingContext::FRAGMENT_SHADER, FRAGMENT_SHADER)
        .expect("Failed to compile fragment shader");
    let program = link_program(gl, &vert_shader, &frag_shader).expect("Failed to link program");

    let vertices: [f32; 21] = [
        1.0, -1.0, 0.0, 1.0, 0.0, 0.0, 1.0, -1.0, -1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 1.0, 1.0,
    ];

    let vertex_buffer = gl.create_buffer().expect("Failed to create buffer");
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&vertex_buffer));

    let vertices_array = unsafe { js_sys::Float32Array::view(&vertices) };
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &vertices_array,
        WebGl2RenderingContext::STATIC_DRAW,
    );

    let mvp_location = gl
        .get_uniform_location(&program, "u_mvp")
        .expect("Failed to get MVP uniform location");

    let position_attrib_location = gl.get_attrib_location(&program, "a_position") as u32;
    let color_attrib_location = gl.get_attrib_location(&program, "a_color") as u32;

    GlResources {
        program,
        vertex_buffer,
        mvp_location,
        position_attrib_location,
        color_attrib_location,
    }
}

async fn setup_xr_rendering(session: XrSession) {
    let canvas = get_canvas();

    let context_options = js_sys::Object::new();
    js_sys::Reflect::set(&context_options, &"xrCompatible".into(), &true.into()).ok();

    let gl_context: WebGl2RenderingContext = canvas
        .get_context_with_context_options("webgl2", &context_options)
        .expect("Failed to get WebGL2 context")
        .expect("WebGL2 context is None")
        .dyn_into()
        .expect("Context is not WebGL2");

    if let Err(error) = wasm_bindgen_futures::JsFuture::from(gl_context.make_xr_compatible()).await
    {
        log::error!("Failed to make WebGL context XR compatible: {:?}", error);
        return;
    }

    let layer_init = web_sys::XrWebGlLayerInit::new();
    let gl_layer = XrWebGlLayer::new_with_web_gl2_rendering_context_and_layer_init(
        &session,
        &gl_context,
        &layer_init,
    )
    .expect("Failed to create XRWebGLLayer");

    let render_state_init = XrRenderStateInit::new();
    render_state_init.set_base_layer(Some(&gl_layer));
    session.update_render_state_with_state(&render_state_init);

    let reference_space_promise = session.request_reference_space(XrReferenceSpaceType::LocalFloor);
    let reference_space: XrReferenceSpace =
        match wasm_bindgen_futures::JsFuture::from(reference_space_promise).await {
            Ok(space) => space.dyn_into().expect("Expected XrReferenceSpace"),
            Err(_) => {
                log::warn!("local-floor not available, trying local");
                let local_promise = session.request_reference_space(XrReferenceSpaceType::Local);
                match wasm_bindgen_futures::JsFuture::from(local_promise).await {
                    Ok(space) => space.dyn_into().expect("Expected XrReferenceSpace"),
                    Err(error) => {
                        log::error!("Failed to create reference space: {:?}", error);
                        return;
                    }
                }
            }
        };

    let resources = setup_gl_resources(&gl_context);

    let state = Rc::new(RefCell::new(WebXrState {
        session: Some(session.clone()),
        reference_space: Some(reference_space),
        gl_layer: Some(gl_layer),
        gl_context: Some(gl_context),
        program: Some(resources.program),
        vertex_buffer: Some(resources.vertex_buffer),
        mvp_location: Some(resources.mvp_location),
        position_attrib_location: resources.position_attrib_location,
        color_attrib_location: resources.color_attrib_location,
        model_rotation: 0.0,
        last_time: None,
    }));

    let on_end = {
        let state = state.clone();
        Closure::wrap(Box::new(move |_event: web_sys::Event| {
            log::info!("WebXR session ended");
            let mut state = state.borrow_mut();
            state.session = None;
            state.reference_space = None;
            state.gl_layer = None;
            state.gl_context = None;

            if let Some(button) = get_document().get_element_by_id("enter-vr-button")
                && let Some(b) = button.dyn_ref::<HtmlButtonElement>()
            {
                b.set_disabled(false);
            }
        }) as Box<dyn Fn(web_sys::Event)>)
    };
    session.set_onend(Some(on_end.as_ref().unchecked_ref()));
    on_end.forget();

    request_animation_frame(state, session);
}

fn request_animation_frame(state: Rc<RefCell<WebXrState>>, session: XrSession) {
    let callback = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64, XrFrame)>>));
    let callback_clone = callback.clone();
    let state_for_callback = state.clone();

    *callback.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64, frame: XrFrame| {
        let mut state = state_for_callback.borrow_mut();

        let delta_time = if let Some(last) = state.last_time {
            ((time - last) / 1000.0) as f32
        } else {
            0.0
        };
        state.last_time = Some(time);

        state.model_rotation += 30_f32.to_radians() * delta_time;

        if let (
            Some(reference_space),
            Some(gl_layer),
            Some(gl),
            Some(program),
            Some(vertex_buffer),
            Some(mvp_location),
        ) = (
            &state.reference_space,
            &state.gl_layer,
            &state.gl_context,
            &state.program,
            &state.vertex_buffer,
            &state.mvp_location,
        ) && let Some(viewer_pose) = frame.get_viewer_pose(reference_space)
        {
            let framebuffer = gl_layer.framebuffer();
            gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, framebuffer.as_ref());

            gl.clear_color(0.19, 0.24, 0.42, 1.0);
            gl.clear(
                WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT,
            );
            gl.enable(WebGl2RenderingContext::DEPTH_TEST);

            let views = viewer_pose.views();
            for view_index in 0..views.length() {
                let view: web_sys::XrView = views.get(view_index).dyn_into().unwrap();

                if let Some(viewport) = gl_layer.get_viewport(&view) {
                    gl.viewport(
                        viewport.x(),
                        viewport.y(),
                        viewport.width(),
                        viewport.height(),
                    );

                    let projection_matrix = view.projection_matrix();
                    let transform = view.transform();
                    let position = transform.position();
                    let orientation = transform.orientation();

                    let projection = nalgebra_glm::Mat4::from_column_slice(&projection_matrix);

                    let eye_position = nalgebra_glm::vec3(
                        position.x() as f32,
                        position.y() as f32,
                        position.z() as f32,
                    );

                    let eye_orientation = nalgebra_glm::quat(
                        orientation.w() as f32,
                        orientation.x() as f32,
                        orientation.y() as f32,
                        orientation.z() as f32,
                    );

                    let rotation_matrix = nalgebra_glm::quat_to_mat4(&eye_orientation);
                    let translation_matrix = nalgebra_glm::translation(&eye_position);
                    let view_matrix_inv = translation_matrix * rotation_matrix;
                    let view_matrix = nalgebra_glm::inverse(&view_matrix_inv);

                    let model_translation =
                        nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 1.5, -2.0));
                    let model_rotation_matrix = nalgebra_glm::rotation(
                        state.model_rotation,
                        &nalgebra_glm::vec3(0.0, 1.0, 0.0),
                    );
                    let model_matrix = model_translation * model_rotation_matrix;

                    let mvp = projection * view_matrix * model_matrix;

                    gl.use_program(Some(program));

                    let mvp_array: [f32; 16] = mvp.as_slice().try_into().unwrap();
                    gl.uniform_matrix4fv_with_f32_array(Some(mvp_location), false, &mvp_array);

                    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(vertex_buffer));

                    gl.enable_vertex_attrib_array(state.position_attrib_location);
                    gl.vertex_attrib_pointer_with_i32(
                        state.position_attrib_location,
                        3,
                        WebGl2RenderingContext::FLOAT,
                        false,
                        28,
                        0,
                    );

                    gl.enable_vertex_attrib_array(state.color_attrib_location);
                    gl.vertex_attrib_pointer_with_i32(
                        state.color_attrib_location,
                        4,
                        WebGl2RenderingContext::FLOAT,
                        false,
                        28,
                        12,
                    );

                    gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);
                }
            }
        }

        if let Some(ref session) = state.session {
            let session = session.clone();
            drop(state);
            let callback_ref = callback_clone.borrow();
            if let Some(ref cb) = *callback_ref {
                let _ = session.request_animation_frame(cb.as_ref().unchecked_ref());
            }
        }
    }) as Box<dyn FnMut(f64, XrFrame)>));

    let callback_ref = callback.borrow();
    if let Some(ref cb) = *callback_ref {
        let _ = session.request_animation_frame(cb.as_ref().unchecked_ref());
    }
}

const VERTEX_SHADER: &str = r#"#version 300 es
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec4 a_color;

uniform mat4 u_mvp;

out vec4 v_color;

void main() {
    v_color = a_color;
    gl_Position = u_mvp * vec4(a_position, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;

in vec4 v_color;
out vec4 fragColor;

void main() {
    fragColor = v_color;
}
"#;
