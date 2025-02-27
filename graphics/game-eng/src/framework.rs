use std::time::Instant;

use winit::{
    event::{self, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub trait Framework {
    fn optional_features() -> wgpu::Features {
        wgpu::Features::empty()
    }
    fn required_features() -> wgpu::Features {
        wgpu::Features::empty()
    }
    fn required_downlevel_capabilities() -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities {
            flags: wgpu::DownlevelFlags::empty(),
            shader_model: wgpu::ShaderModel::Sm5,
            ..wgpu::DownlevelCapabilities::default()
        }
    }
    fn required_limits() -> wgpu::Limits {
        wgpu::Limits::downlevel_webgl2_defaults() // These downlevel limits will allow the code to run on all possible hardware
    }

    type SetupStruct;
    fn init(
        config: &wgpu::SurfaceConfiguration,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        setup_struct: Self::SetupStruct, 
    ) -> Self;

    fn resize(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    fn update(&mut self, event: WindowEvent);

    fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
}

pub struct Setup {
    window: winit::window::Window,
    event_loop: EventLoop<()>,
    instance: wgpu::Instance,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Setup {
    pub async fn setup<E: Framework>(title: &str) -> Setup{

        let event_loop = EventLoop::new();
        let mut builder = winit::window::WindowBuilder::new();
        builder = builder.with_title(title);
        let window = builder.build(&event_loop).unwrap();

        let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
        let instance = wgpu::Instance::new(backend);

        let (size, surface) = unsafe {
            let size = window.inner_size();
            let surface = instance.create_surface(&window);
            (size, surface)
        };

        let adapter =
        wgpu::util::initialize_adapter_from_env_or_default(&instance, backend, Some(&surface))
            .await
            .expect("No suitable GPU adapters found on the system!");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let adapter_info = adapter.get_info();
        println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    }

    let optional_features = E::optional_features();
    let required_features = E::required_features();
    let adapter_features = adapter.features();
    assert!(
        adapter_features.contains(required_features),
        "Adapter does not support required features for this example: {:?}",
        required_features - adapter_features
    );

    let required_downlevel_capabilities = E::required_downlevel_capabilities();
    let downlevel_capabilities = adapter.get_downlevel_capabilities();
    assert!(
        downlevel_capabilities.shader_model >= required_downlevel_capabilities.shader_model,
        "Adapter does not support the minimum shader model required to run this example: {:?}",
        required_downlevel_capabilities.shader_model
    );
    assert!(
        downlevel_capabilities
            .flags
            .contains(required_downlevel_capabilities.flags),
        "Adapter does not support the downlevel capabilities required to run this example: {:?}",
        required_downlevel_capabilities.flags - downlevel_capabilities.flags
    );

    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the surface.
    let needed_limits = E::required_limits().using_resolution(adapter.limits());

    let trace_dir = std::env::var("WGPU_TRACE");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: (optional_features & adapter_features) | required_features,
                limits: needed_limits,
            },
            trace_dir.ok().as_ref().map(std::path::Path::new),
        )
        .await
        .expect("Unable to find a suitable GPU adapter!");
    
        Setup {
            window,
            event_loop,
            instance,
            size,
            surface,
            adapter,
            device,
            queue,
        }
    }

    pub fn start<E>(Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        device,
        queue,
    }: Setup
    , setup_struct: E::SetupStruct) where E: Framework + 'static {
        let mut config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface.get_supported_alpha_modes(&adapter)[0],
        };
        surface.configure(&device, &config);

        log::info!("Initializing the example...");
        let mut state = E::init(&config, &adapter, &device, &queue, setup_struct);

        #[cfg(not(target_arch = "wasm32"))]
        let mut last_frame_inst = Instant::now();
        #[cfg(not(target_arch = "wasm32"))]
        let (mut frame_count, mut accum_time) = (0, 0.0);

        log::info!("Entering render loop...");
        event_loop.run(move |event, _, control_flow| {
            let _ = (&instance, &adapter); // force ownership by the closure
            *control_flow = if cfg!(feature = "metal-auto-capture") {
                ControlFlow::Exit
            } else {
                ControlFlow::Poll
            };
            match event {
                event::Event::RedrawEventsCleared => {
                    window.request_redraw();
                }
                event::Event::WindowEvent {
                    event:
                        WindowEvent::Resized(size)
                        | WindowEvent::ScaleFactorChanged {
                            new_inner_size: &mut size,
                            ..
                        },
                    ..
                } => {
                    log::info!("Resizing to {:?}", size);
                    config.width = size.width.max(1); 
                    config.height = size.height.max(1);
                    state.resize(&config, &device, &queue);
                    surface.configure(&device, &config);
                }
                event::Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput {
                        input:
                            event::KeyboardInput {
                                virtual_keycode: Some(event::VirtualKeyCode::Escape),
                                state: event::ElementState::Pressed,
                                ..
                            },
                        ..
                    }
                    | WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            event::KeyboardInput {
                                virtual_keycode: Some(event::VirtualKeyCode::R),
                                state: event::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        println!("{:#?}", instance.generate_report());
                    }
                    _ => {
                        state.update(event);
                    }
                },
                event::Event::RedrawRequested(_) => {

                    {
                        accum_time += last_frame_inst.elapsed().as_secs_f32();
                        last_frame_inst = Instant::now();
                        frame_count += 1;
                        if frame_count == 100 {
                            println!(
                                "Avg frame time {}ms",
                                accum_time * 1000.0 / frame_count as f32
                            );
                            accum_time = 0.0;
                            frame_count = 0;
                        }
                    }

                    let frame = match surface.get_current_texture() {
                        Ok(frame) => frame,
                        Err(_) => {
                            surface.configure(&device, &config);
                            surface
                                .get_current_texture()
                                .expect("Failed to acquire next surface texture!")
                        }
                    };
                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    state.render(&view, &device, &queue);

                    frame.present();

                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(offscreen_canvas_setup) = &offscreen_canvas_setup {
                            let image_bitmap = offscreen_canvas_setup
                                .offscreen_canvas
                                .transfer_to_image_bitmap()
                                .expect("couldn't transfer offscreen canvas to image bitmap.");
                            offscreen_canvas_setup
                                .bitmap_renderer
                                .transfer_from_image_bitmap(&image_bitmap);

                            log::info!("Transferring OffscreenCanvas to ImageBitmapRenderer");
                        }
                    }
                }
                _ => {}
            }
        });
    }
}