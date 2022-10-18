// mod tex_array_old;
// pub use tex_array::run;

mod base;
mod texture;
mod tex_array;
use std::{time::Instant, sync::{Arc, RwLock}};

use tex_array::vertex;

use winit::{
    event::{self, WindowEvent},
    event_loop::{ControlFlow},
};



pub fn run() {

    env_logger::init();
    
    let window_n = Arc::from(RwLock::from(base::Window::init("test Window")));
    let base_n = Arc::from(RwLock::from(base::Base::init()
                                .pass_window(window_n.clone())
                                .get_surface()
                                .adapter_from_compatible_adapters()
                                .get_device_and_queue(tex_array::REQUIRED_FEATURES(), wgpu::Features::empty(), wgpu::Limits::downlevel_webgl2_defaults())
                                .configure_surface()
                                .build()));

    let test_texture = texture::Texture::from_bytes(&(*base_n.read().unwrap()).device, &(*base_n.read().unwrap()).queue, include_bytes!("../../../assets/blue1x1.png"), &"Test Texture").unwrap();

    let texture_renderer = Arc::from(RwLock::from(tex_array::TextureRenderer::init(base_n.clone(), window_n.clone())
                                                                .create_pipeline(2)
                                                                .create_index_buffer()
                                                                .create_vertex_buffer()
                                                                .create_bind_group(vec![&test_texture,&test_texture])
                                                                .set_index_format(wgpu::IndexFormat::Uint16)
                                                                .build()));
    
    let texture_renderer_clone = texture_renderer.clone();

    (*texture_renderer_clone.write().unwrap()).modify_index_buffer(vec![2,1,0]);
    (*texture_renderer_clone.write().unwrap()).modify_vertex_buffer(
        vec![
            vertex!([0.0,-1.0], [0.0,-1.0], 0),
            vertex!([1.0,1.0], [1.0,1.0], 0),
            vertex!([-1.0,1.0], [-1.0,1.0], 0),
        ]
    );



    let mut last_frame_inst = Instant::now();

    let (mut frame_count, mut accum_time) = (0, 0.0);

    let window = window_n.clone();
    let base = base_n.clone();
    log::info!("Entering render loop...");
    window_n.write().unwrap().event_loop.take().unwrap().run(move |event, _, control_flow| {
        *control_flow = {
            ControlFlow::Poll
        };
        match event {
            event::Event::RedrawEventsCleared => {


                window.read().unwrap().window.request_redraw();
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
                base.write().unwrap().config.width = size.width.max(1); 
                base.write().unwrap().config.height = size.height.max(1);
                base.read().unwrap().surface.configure(&base.read().unwrap().device, &base.read().unwrap().config);
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
                #[cfg(not(target_arch = "wasm32"))]
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(event::VirtualKeyCode::R),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    println!("{:#?}", &base.read().unwrap().instance.generate_report());
                }
                _ => {
                    
                }
            },
            event::Event::RedrawRequested(_) => {
                #[cfg(not(target_arch = "wasm32"))]
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

                let frame = match base.read().unwrap().surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        base.read().unwrap().surface.configure(&base.read().unwrap().device, &base.read().unwrap().config);
                        base.read().unwrap().surface
                            .get_current_texture()
                            .expect("Failed to acquire next surface texture!")
                    }
                };
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                (*texture_renderer.read().unwrap()).render(&view);

                frame.present();
            }
            _ => {}
        }
    });


}