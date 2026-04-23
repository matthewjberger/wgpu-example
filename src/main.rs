fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "openxr")]
    {
        app::run_xr()
    }

    #[cfg(not(feature = "openxr"))]
    {
        let event_loop = winit::event_loop::EventLoop::builder().build()?;
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        let mut application = app::App::default();
        event_loop.run_app(&mut application)?;
        Ok(())
    }
}
