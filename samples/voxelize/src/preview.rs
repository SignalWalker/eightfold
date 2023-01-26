use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use eightfold::spatial::VoxelOctree;
use gltf::Gltf;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::{buffer::BufferCache, Leaf};

struct GpuState {
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
}

impl GpuState {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

        let surface = unsafe { instance.create_surface(&window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("could not find wgpu adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: surface
                .get_supported_present_modes(&adapter)
                .iter()
                .copied()
                .find(|m| *m == wgpu::PresentMode::Mailbox)
                .unwrap_or(wgpu::PresentMode::AutoVsync),
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };

        surface.configure(&device, &surface_config);

        Self {
            instance,
            surface,
            surface_config,
            adapter,
            device,
            queue,
            size,
            window,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn update(&mut self) {}
}

pub async fn show_preview<'docs>(
    docs: &'docs [(&PathBuf, Gltf)],
    caches: &HashMap<&Path, BufferCache<'docs>>,
    tree: VoxelOctree<Leaf, f32, u32>,
) -> ! {
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();

    let mut gpu_state = GpuState::new(window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == gpu_state.window.id() => match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(size) => gpu_state.resize(*size),
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                gpu_state.resize(**new_inner_size)
            }
            _ => {}
        },
        Event::RedrawRequested(window_id) if window_id == gpu_state.window.id() => {
            gpu_state.update();
            match gpu_state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => gpu_state.resize(gpu_state.size),
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    tracing::error!("out of memory");
                    *control_flow = ControlFlow::Exit;
                }
                Err(e) => {
                    tracing::warn!(?e);
                }
            }
        }
        Event::MainEventsCleared => {
            gpu_state.window.request_redraw();
        }
        _ => {}
    })
}
