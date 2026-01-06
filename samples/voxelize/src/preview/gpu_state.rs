use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use crossbeam::atomic::AtomicCell;
use eightfold::spatial::VoxelOctree;
use gltf::Gltf;
use nalgebra::{Affine3, Isometry3, Matrix4, Scale3};
use parking_lot::RwLock;
use time::ext::InstantExt;
use wgpu::{
    util::{DeviceExt, RenderEncoder},
    BufferUsages, InstanceFlags,
};
use winit::window::Window;

use crate::{
    buffer::{BufferCache, GpuBufferCache, GpuMesh, GpuTexture, UniformBuffer},
    gltf_to_nalgebra,
    preview::{OrbitCamera, PipelineData, PipelineStore},
    Leaf,
};

pub(super) struct GpuState<'window> {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'window>,
    adapter: wgpu::Adapter,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface_config: RwLock<wgpu::SurfaceConfiguration>,
    size: AtomicCell<winit::dpi::PhysicalSize<u32>>,

    pub pipeline_store: PipelineStore,
    nodes: RwLock<Vec<GpuMesh>>,
    node_draw_sort: RwLock<HashMap<PipelineData, HashSet<usize>>>,

    ref_nodes: RwLock<Vec<GpuMesh>>,

    window: &'window Window,

    camera_buffer: UniformBuffer<Matrix4<f32>>,
    world_bind_group: wgpu::BindGroup,

    buffer_cache: GpuBufferCache,

    depth_texture: RwLock<GpuTexture>,
}

impl<'win> GpuState<'win> {
    #[tracing::instrument]
    pub(super) async fn new(window: &'win Window, camera: &OrbitCamera) -> Self {
        tracing::trace!("building new GPU state...");
        let size = window.inner_size();

        tracing::trace!(?window, "creating instance...");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: InstanceFlags::DEBUG
                | InstanceFlags::VALIDATION
                | InstanceFlags::GPU_BASED_VALIDATION,
            ..Default::default()
        });

        tracing::trace!(?instance, "creating surface...");
        let surface = instance.create_surface(window).unwrap();

        tracing::trace!(?surface, "requesting adapter...");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("could not find wgpu adapter");

        tracing::trace!(?adapter, "requesting device...");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("Render Device"),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let surface_caps = surface.get_capabilities(&adapter);

        let format = surface_caps.formats[0];
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps
                .present_modes
                .iter()
                .copied()
                .find(|m| *m == wgpu::PresentMode::Mailbox)
                .unwrap_or(wgpu::PresentMode::AutoVsync),
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![format], // TODO :: surface_caps.formats.iter().cloned().filter(|f| f.is).collect::<Vec<_>>(),
            desired_maximum_frame_latency: 2,
        };

        tracing::trace!(?device, ?queue, ?surface_config, "configuring surface...");
        surface.configure(&device, &surface_config);

        let pipeline_store = PipelineStore::new(
            device.clone(),
            wgpu::naga::front::wgsl::parse_str(
                &std::fs::read_to_string("samples/voxelize/src/shader/shader.wgsl").unwrap(),
            )
            .unwrap(),
            format,
        );

        let camera_buffer = UniformBuffer::create_init(
            &device,
            Some("Camera Buffer"),
            BufferUsages::COPY_DST,
            &camera.proj_view_mat(size.width as f32 / size.height as f32),
        );

        let world_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("World Bind Group"),
            layout: &pipeline_store.bind_group_layouts()[0],
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let buffer_cache = GpuBufferCache::new(device.clone(), queue.clone());

        let depth_texture = RwLock::new(GpuTexture::new_depth_texture(
            &device,
            surface_config.width.try_into().unwrap(),
            surface_config.height.try_into().unwrap(),
        ));

        Self {
            instance,
            surface,
            surface_config: RwLock::new(surface_config),
            adapter,
            device,
            queue,
            size: AtomicCell::new(size),
            pipeline_store,
            nodes: Default::default(),
            node_draw_sort: Default::default(),

            ref_nodes: Default::default(),

            window,

            camera_buffer,
            camera_bind_group,

            buffer_cache,

            depth_texture,
        }
    }

    pub fn resize(&self, camera: &OrbitCamera, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            tracing::trace!(?new_size, "resizing surface...");
            self.size.store(new_size);
            let mut cfg_lock = self.surface_config.write();
            cfg_lock.width = new_size.width;
            cfg_lock.height = new_size.height;
            self.surface.configure(&self.device, &cfg_lock);

            *self.depth_texture.write() = GpuTexture::new_depth_texture(
                &self.device,
                new_size.width.try_into().unwrap(),
                new_size.height.try_into().unwrap(),
            );

            self.update_camera_buffer(camera, new_size.width as f32 / new_size.height as f32);
        }
    }

    pub fn update_camera_buffer(&self, camera: &OrbitCamera, aspect: f32) {
        self.camera_buffer
            .write(&self.queue, &camera.proj_view_mat(aspect));
    }

    pub fn render(&self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = self.depth_texture.read();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 0.6,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            let nodes = self.nodes.read();
            let node_draw_sort = self.node_draw_sort.read();
            for (bgroup_index, bind_group) in self.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(bgroup_index as u32, bind_group, &[]);
            }
            for (p_data, mesh_indices) in &*node_draw_sort {
                render_pass.set_pipeline(&self.pipeline_store.get(p_data).unwrap());
                for mesh in mesh_indices.iter().map(|&i| nodes.get(i)) {
                    #[cfg(debug_assertions)]
                    if mesh.is_none() {
                        tracing::error!("incorrect mesh index");
                        continue;
                    }
                    mesh.unwrap().draw(p_data, &mut render_pass);
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn push_reference(&self) {
        let mesh_bind_group_layout = &self.pipeline_store.bind_group_layouts()[1];
        let mut ref_nodes = self.ref_nodes.write();
        ref_nodes.push(GpuMesh::create_cube(
            &self.device,
            Some("Ref X"),
            Affine3::from_matrix_unchecked(Isometry3::translation(1.0, 0.0, 0.0).to_homogeneous()),
            mesh_bind_group_layout,
            [1.0, 0.0, 0.0],
        ));
        ref_nodes.push(GpuMesh::create_cube(
            &self.device,
            Some("Ref Y"),
            Affine3::from_matrix_unchecked(Isometry3::translation(0.0, 1.0, 0.0).to_homogeneous()),
            mesh_bind_group_layout,
            [0.0, 1.0, 0.0],
        ));
        ref_nodes.push(GpuMesh::create_cube(
            &self.device,
            Some("Ref Z"),
            Affine3::from_matrix_unchecked(Isometry3::translation(0.0, 0.0, 1.0).to_homogeneous()),
            mesh_bind_group_layout,
            [0.0, 0.0, 1.0],
        ));
        for node in &*ref_nodes {
            for (pdata, _) in &node.primitives {
                self.pipeline_store.get_or_create(&self.device, pdata);
            }
        }
    }

    #[tracing::instrument(skip(self, docs, doc_caches, tree))]
    pub fn push_buffers(
        &self,
        docs: &[(&PathBuf, Gltf)],
        doc_caches: &HashMap<&Path, BufferCache<'_>>,
        tree: &VoxelOctree<Leaf, f32, u32>,
        scale: &Scale3<f32>,
        base_transform: &Affine3<f32>,
    ) {
        fn process_node(
            gpu: &GpuState<'_>,
            doc_cache: &BufferCache<'_>,
            scale: &Scale3<f32>,
            parent_transform: &Affine3<f32>,
            node: gltf::Node<'_>,
        ) -> Result<Vec<GpuMesh>, crate::Error<u32, f32>> {
            tracing::trace!("building device buffer for node");
            let transform = gltf_to_nalgebra(&node.transform()) * parent_transform;
            let scaled_transform = {
                let mut res: Affine3<f32> = transform;
                res.matrix_mut_unchecked()
                    .append_nonuniform_scaling_mut(&scale.vector);
                res
            };
            let mut res = Vec::new();
            if let Some(mesh) = node.mesh() {
                let bgroup_layouts = gpu.pipeline_store.bind_group_layouts();
                res.push(GpuMesh::create_init(
                    &gpu.buffer_cache,
                    &gpu.pipeline_store,
                    doc_cache,
                    mesh,
                    scaled_transform,
                    // FIX :: this seems bad
                    &bgroup_layouts[1],
                    &bgroup_layouts[2],
                ));
            }
            for child in node.children() {
                res.append(&mut process_node(gpu, doc_cache, scale, &transform, child)?);
            }
            Ok(res)
        }
        tracing::debug!("pushing mesh buffers to gpu...");
        let mut nodes = self.nodes.write();
        let mut node_sort = self.node_draw_sort.write();
        let mut node_index = 0;
        let push_start = Instant::now();
        for (path, doc) in docs {
            let _doc_span = tracing::info_span!("glTF_doc", ?path).entered();
            let cache = &doc_caches[path.as_path()];
            for scene in doc.scenes() {
                let _scene_span =
                    tracing::info_span!("glTF_scene", index = scene.index(), name = scene.name())
                        .entered();
                for scene_node in scene.nodes() {
                    let mut meshes =
                        process_node(self, cache, scale, base_transform, scene_node).unwrap();
                    for (i, mesh) in meshes.iter().enumerate() {
                        for p_data in mesh.primitives.keys() {
                            node_sort
                                .entry(p_data.clone())
                                .or_default()
                                .insert(i + node_index);
                        }
                    }
                    node_index += meshes.len();
                    nodes.append(&mut meshes);
                }
            }
        }
        tracing::info!(
            pipelines = self.pipeline_store.len(),
            nodes = nodes.len(),
            duration = %(Instant::now().signed_duration_since(push_start)),
            "pushed buffers to GPU"
        );
        for entry in &self.pipeline_store.pipelines {
            tracing::debug!(pipeline = %entry.key());
        }
    }
}
