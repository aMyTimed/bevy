use crate::renderer::{WgpuRenderGraphExecutor, WgpuRenderResourceContext};
use bevy_app::{EventReader, Events};
use bevy_ecs::{Resources, World};
use bevy_render::{
    render_graph::{DependentNodeStager, RenderGraph, RenderGraphStager},
    renderer::RenderResourceContext,
};
use bevy_window::{WindowCreated, WindowResized, Windows};
use std::{ops::Deref, sync::Arc};
pub struct WgpuRenderer {
    pub instance: wgpu::Instance,
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
    pub window_resized_event_reader: EventReader<WindowResized>,
    pub window_created_event_reader: EventReader<WindowCreated>,
    pub intialized: bool,
}

impl WgpuRenderer {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: None,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: true,
                },
                None,
            )
            .await
            .unwrap();
        let device = Arc::new(device);
        WgpuRenderer {
            instance,
            device,
            queue,
            window_resized_event_reader: Default::default(),
            window_created_event_reader: Default::default(),
            intialized: false,
        }
    }

    pub fn handle_window_created_events(&mut self, resources: &Resources) {
        let mut render_resource_context = resources
            .get_mut::<Box<dyn RenderResourceContext>>()
            .unwrap();
        let render_resource_context = render_resource_context
            .downcast_mut::<WgpuRenderResourceContext>()
            .unwrap();
        let windows = resources.get::<Windows>().unwrap();
        let window_created_events = resources.get::<Events<WindowCreated>>().unwrap();
        for window_created_event in self
            .window_created_event_reader
            .iter(&window_created_events)
        {
            let window = windows
                .get(window_created_event.id)
                .expect("Received window created event for non-existent window");
            #[cfg(feature = "bevy_winit")]
            {
                let winit_windows = resources.get::<bevy_winit::WinitWindows>().unwrap();
                let winit_window = winit_windows.get_window(window.id).unwrap();
                let surface = unsafe { self.instance.create_surface(winit_window.deref()) };
                render_resource_context.set_window_surface(window.id, surface);
            }
        }
    }

    pub fn run_graph(&mut self, world: &mut World, resources: &mut Resources) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        // stage nodes
        let mut stager = DependentNodeStager::loose_grouping();
        let stages = stager.get_stages(&render_graph).unwrap();
        let mut borrowed = stages.borrow(&mut render_graph);

        // execute stages
        let graph_executor = WgpuRenderGraphExecutor {
            max_thread_count: 2,
        };
        graph_executor.execute(
            world,
            resources,
            self.device.clone(),
            &mut self.queue,
            &mut borrowed,
        );
    }

    pub fn update(&mut self, world: &mut World, resources: &mut Resources) {
        self.handle_window_created_events(resources);
        self.run_graph(world, resources);

        let render_resource_context = resources.get::<Box<dyn RenderResourceContext>>().unwrap();
        render_resource_context.drop_all_swap_chain_textures();
        render_resource_context.clear_bind_groups();
    }
}
