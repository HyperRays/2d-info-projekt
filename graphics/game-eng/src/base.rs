use std::{fmt::Debug, sync::{Arc, RwLock}};
use wgpu::{Adapter};
use winit::event_loop::EventLoop;

pub struct Base{
    pub instance: wgpu::Instance, 
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}


pub struct Window {
    pub window: winit::window::Window,
    pub event_loop: Option<EventLoop<()>>,
    size: winit::dpi::PhysicalSize<u32>,
}

#[derive(PartialEq, PartialOrd, Debug)]
enum BaseBuildOrder {
    InstanceAndBackend,
    Window,
    Surface,
    Adapter,
    DeviceAndQueue,
    SurfaceConfiguration,
    Build
}

pub struct BuilderOrderChecker<O> {
    stage: O,
}

impl<O: PartialEq + PartialOrd + Debug> BuilderOrderChecker<O> {
    pub fn check_order(&self, other: &O){
        assert!(&self.stage < other, "{:?} was called after stage {:?}", self.stage, other);
    }

    pub fn check_order_else(&self, other: &O, f: fn()){
        match &self.stage < other {
            true => {},
            false => {f()}
        }
    }

    pub fn check_and_change(&mut self, mut other: O){
        self.check_order(&other);
        std::mem::swap(&mut self.stage, &mut other)
    }

    pub fn check_and_change_else(&mut self, mut other: O, f: fn()){
        self.check_order_else(&other, f);
        std::mem::swap(&mut self.stage, &mut other)
    }

    pub fn new(stage: O) -> Self{
        Self{
            stage
        }
    }
}

pub struct BaseBuilder {
    // window
    window: Option<Arc<RwLock<Window>>>,
    // base elements
    instance: wgpu::Instance,
    backend: wgpu::Backends,
    surface: Option<wgpu::Surface>,
    config: Option<wgpu::SurfaceConfiguration>,
    adapter: Option<wgpu::Adapter>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,

    //build order check
    stage: BuilderOrderChecker<BaseBuildOrder>
}

impl Base {
    pub fn init() -> BaseBuilder{
        let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
        let instance =  wgpu::Instance::new(backend);
        BaseBuilder { window: None, instance, backend, surface: None,config: None, adapter: None, device: None, queue: None , stage: BuilderOrderChecker::new(BaseBuildOrder::InstanceAndBackend)}
    }
}



impl Window {
    pub fn init(title: &str) -> Self{
        let event_loop = Some(EventLoop::new());
        let mut builder = winit::window::WindowBuilder::new();
        builder = builder.with_title(title);
        let window = builder.build(event_loop.as_ref().unwrap()).unwrap();
        let size = window.inner_size();

        Self {
            window,
            event_loop,
            size
        }
    }
}

impl BaseBuilder {
    pub fn pass_window(mut self, window: Arc<RwLock<Window>>) -> Self{
        self.stage.check_and_change(BaseBuildOrder::Window);
        self.window = Some(window);
        self
    }

    pub fn get_surface(mut self) -> Self{
        self.stage.check_and_change(BaseBuildOrder::Surface);
        let surface = unsafe {
            self.instance.create_surface(&(self.window.as_ref().unwrap().read().unwrap()).window)
        };
        self.surface = Some(surface);
        println!("{:?}",&self.surface);
        self
    }

    pub fn adapter_from_all_compatible_adapters(mut self, f: fn(&Adapter) -> bool) -> Self{
        self.stage.check_and_change(BaseBuildOrder::Adapter);
        let adapter = self.instance
        .enumerate_adapters(wgpu::Backends::all())
        .filter(|adapter| {
            // Check if this adapter supports our surface
            !self.surface.as_ref().unwrap().get_supported_formats(&adapter).is_empty()
        }).find(f);
        
        self.adapter = Some(adapter.expect("No suitable GPU adapters found on the system!"));
        self
    }

    pub fn adapter_from_compatible_adapters(mut self) -> Self{
        self.stage.check_and_change(BaseBuildOrder::Adapter);
        let adapter =
        pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(
            &self.instance, 
            self.backend, 
            Some(self.surface.as_ref().expect("surface not configured"))))
            .expect("No suitable GPU adapters found on the system!");
        self.adapter = Some(adapter);
        self
    }

    pub fn get_device_and_queue(mut self, required_features: wgpu::Features, optional_features: wgpu::Features, needed_limits: wgpu::Limits) -> Self {
        self.stage.check_and_change(BaseBuildOrder::DeviceAndQueue);
        let trace_dir = std::env::var("WGPU_TRACE");    
        let (device, queue) = pollster::block_on(self.adapter.as_ref().unwrap().request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: (optional_features & self.adapter.as_ref().unwrap().features()) | required_features,
                limits: needed_limits.using_resolution(self.adapter.as_ref().unwrap().limits()),
            },
            trace_dir.ok().as_ref().map(std::path::Path::new),
        ))
        .expect("Unable to find a suitable GPU adapter!");
        
        (self.device, self.queue) = (Some(device), Some(queue));
        self
    }

    pub fn configure_surface(mut self) -> Self {
        self.stage.check_and_change(BaseBuildOrder::SurfaceConfiguration);
        

        let size = (self.window.as_ref().unwrap().read().unwrap()).window.inner_size();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface.as_ref().unwrap().get_supported_formats(self.adapter.as_ref().unwrap())[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: self.surface.as_ref().unwrap().get_supported_alpha_modes(self.adapter.as_ref().unwrap())[0],
        };

        self.surface.as_ref().unwrap().configure(&self.device.as_ref().expect("no device found"), &config);
        self.config = Some(config);
        self
    }

    pub fn build(self) -> Base{
        self.stage.check_order(&BaseBuildOrder::Build);
        Base { instance: self.instance, surface: self.surface.unwrap(),config: self.config.unwrap() ,adapter: self.adapter.unwrap(), device: self.device.unwrap(), queue: self.queue.unwrap() }
    }
}