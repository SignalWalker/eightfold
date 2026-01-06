use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use dashmap::DashMap;
use eightfold::spatial::{Aabb, VoxelOctree};
use gltf::Gltf;
use nalgebra::{vector, Affine3, Matrix4, Point3, Scale3, Vector2};
use ouroboros::self_referencing;
use pollster::FutureExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    platform::wayland::WindowAttributesExtWayland,
    window::{Window, WindowAttributes},
};

use crate::{
    buffer::{BufferCache, GpuMesh},
    Leaf,
};

mod octree;
mod render_gltf;

mod camera;
pub use camera::*;

mod nalgebra_ext;
pub use nalgebra_ext::*;

mod pipeline;
pub use pipeline::*;

mod gpu_state;
use gpu_state::*;

mod event;
pub use event::*;

pub struct NodeStore {
    nodes: DashMap<PipelineData, Vec<GpuMesh>>,
}

impl Default for NodeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeStore {
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
        }
    }

    pub fn insert(node: GpuMesh) {
        todo!()
    }
}

#[self_referencing]
struct WindowState {
    window: Window,
    #[borrows(window)]
    #[covariant]
    gpu: GpuState<'this>,
}

#[derive(Debug)]
struct Stats {
    pub target_phystime: Duration,
    pub target_frametime: Duration,
    pub last_render: Option<Instant>,
    pub last_update: Instant,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            target_phystime: Duration::from_secs_f64(1.0 / 60.0),
            target_frametime: Duration::from_secs_f64(1.0 / 60.0),
            last_render: Default::default(),
            last_update: Instant::now(),
        }
    }
}

struct AppState<'docs, 'caches> {
    window: Option<WindowState>,
    docs: &'docs [(&'docs PathBuf, Gltf)],
    caches: &'caches HashMap<&'caches Path, BufferCache<'docs>>,
    tree: VoxelOctree<Leaf, f32, u32>,
    stats: Stats,
    input: InputState,
    scale: Scale3<f32>,
    base_transform: Affine3<f32>,
    camera: OrbitCamera,
    show_reference: bool,
}

impl<'docs, 'caches> AppState<'docs, 'caches> {
    fn new(
        docs: &'docs [(&'docs PathBuf, Gltf)],
        caches: &'caches HashMap<&'caches Path, BufferCache<'docs>>,
        tree: VoxelOctree<Leaf, f32, u32>,
        scale: Scale3<f32>,
        base_transform: Affine3<f32>,
        camera: OrbitCamera,
        show_reference: bool,
    ) -> Self {
        Self {
            window: None,
            docs,
            caches,
            tree,
            stats: Stats::default(),
            input: InputState::default(),
            scale,
            base_transform,
            camera,
            show_reference,
        }
    }

    fn update(&mut self) {
        const SPIN_SPEED: Turn = Turn::new(u16::MAX / 2);
        const ZOOM_SPEED: f32 = 0.50;
        const MOVE_SPEED: f32 = 0.50;
        let cdist = self.camera.distance();
        let delta = (Instant::now() - self.stats.last_update).as_secs_f64();
        let spin_speed = Turn::from_turn64(SPIN_SPEED.as_turn64() * delta);
        let zoom_speed = cdist * ZOOM_SPEED * delta as f32;
        let move_speed = cdist * MOVE_SPEED * delta as f32;
        let mut trans = vector![0.0, 0.0];
        for key in &self.input.pressed {
            match key {
                KeyCode::KeyH => {
                    self.camera.orbit(spin_speed);
                }
                KeyCode::KeyJ => {
                    self.camera.incline(-spin_speed);
                }
                KeyCode::KeyK => {
                    self.camera.incline(spin_speed);
                }
                KeyCode::KeyL => {
                    self.camera.orbit(-spin_speed);
                }
                KeyCode::KeyU => {
                    self.camera.dolly(-zoom_speed);
                }
                KeyCode::KeyI => {
                    self.camera.dolly(zoom_speed);
                }
                KeyCode::KeyW => {
                    trans.y += 1.0;
                }
                KeyCode::KeyS => {
                    trans.y -= 1.0;
                }
                KeyCode::KeyA => {
                    trans.x -= 1.0;
                }
                KeyCode::KeyD => {
                    trans.x += 1.0;
                }
                KeyCode::KeyQ => {
                    self.camera.target.y += move_speed;
                }
                KeyCode::KeyE => {
                    self.camera.target.y -= move_speed;
                }
                _ => {}
            }
        }
        if trans.norm_squared() > 0.0 {
            trans.normalize_mut();
            self.camera.translate(trans * move_speed);
        }
        self.stats.last_update = Instant::now();
    }
}

impl<'docs, 'caches> ApplicationHandler for AppState<'docs, 'caches> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        tracing::debug!("building new window");
        let window = event_loop
            .create_window(
                WindowAttributes::new()
                    .with_title(concat!(env!("CARGO_PKG_NAME"), " preview"))
                    .with_name(env!("CARGO_PKG_NAME"), "")
                    .with_resizable(true),
            )
            .unwrap();
        self.window = Some(
            WindowStateBuilder {
                window,
                gpu_builder: |window| {
                    let res = GpuState::new(window, &self.camera).block_on();
                    res.push_buffers(
                        self.docs,
                        self.caches,
                        &self.tree,
                        &self.scale,
                        &self.base_transform,
                    );
                    //if self.show_reference {
                    //    res.push_reference();
                    //}
                    res
                },
            }
            .build(),
        );

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    }

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _cause: winit::event::StartCause,
    ) {
        self.update();
        if let Some(ref window) = self.window {
            match self.stats.last_render {
                Some(last_render) => {
                    if (Instant::now() - last_render) >= self.stats.target_frametime {
                        window.borrow_window().request_redraw();
                    }
                }
                None => window.borrow_window().request_redraw(),
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(ref window) = self.window {
                    let gpu = window.borrow_gpu();
                    let size = window.borrow_window().inner_size();
                    gpu.update_camera_buffer(&self.camera, size.width as f32 / size.height as f32);
                    gpu.render().unwrap();
                }
                self.stats.last_render = Some(Instant::now());
            }
            WindowEvent::Resized(size) => {
                if let Some(ref window) = self.window {
                    window.borrow_gpu().resize(&self.camera, size);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state,
                        repeat: false,
                        ..
                    },
                ..
            } => match state {
                ElementState::Pressed => {
                    self.input.pressed.insert(keycode);
                }
                ElementState::Released => {
                    self.input.pressed.remove(&keycode);
                }
            },
            _ => {}
        }
    }
}

pub async fn show_preview<'docs>(
    docs: &'docs [(&PathBuf, Gltf)],
    caches: &HashMap<&Path, BufferCache<'docs>>,
    tree: VoxelOctree<Leaf, f32, u32>,
    scale: Scale3<f32>,
    base_transform: Affine3<f32>,
    show_reference: bool,
) {
    let event_loop = EventLoop::new().unwrap();
    let aabb = tree.aabb();
    let tcent = aabb.center();
    let tdiag = aabb.maxs() - tcent;
    let cam_dist = (tdiag.x.powi(2) + tdiag.y.powi(2) + tdiag.z.powi(2)).sqrt();
    //let cam_dist = Distance::new(1000);
    tracing::debug!(camera_target = %tcent, camera_distance = %cam_dist);
    let camera = OrbitCamera::new(
        tcent,
        Polar3::new(cam_dist, Turn(0), Turn(0)),
        45.0,
        0.1,
        10000.0,
    );
    let mut state = AppState::new(
        docs,
        caches,
        tree,
        scale,
        base_transform,
        camera,
        show_reference,
    );
    event_loop.run_app(&mut state).unwrap();
}
