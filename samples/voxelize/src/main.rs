#![allow(unsafe_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use buffer::{BufferCache, BufferError};
use clap::Parser;
use eightfold::spatial::{Float, VoxelOctree};
use eightfold::ArrayIndex;

use gltf::accessor::DataType;
use gltf::mesh::Mode;
use gltf::{Gltf, Node, Semantic};
use nalgebra::{Affine3, Isometry3, Matrix4, Point3, Quaternion, Translation3, Unit, Vector3};
use time::Instant;

/// Functions and structures related specifically to the command-line interface.
pub mod cli;

/// Utilities for managing and accessing `glTF` data buffers.
pub mod buffer;

/// Utilities for presenting a preview window to the user
pub mod preview;

#[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL_ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[derive(Debug, thiserror::Error)]
pub enum Error<Idx: ArrayIndex, Real: Float> {
    #[error(transparent)]
    Buffer(#[from] BufferError),
    #[error(transparent)]
    Tree(#[from] eightfold::Error<Idx>),
    #[error(transparent)]
    SpatialTree(#[from] eightfold::spatial::Error<Idx, Real>),
}

#[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
fn trace_memory_stats(prev: Option<(usize, usize)>) -> (usize, usize) {
    use tikv_jemalloc_ctl::{epoch, stats};
    #[inline]
    fn sub_to_isize(a: usize, b: usize) -> isize {
        match a.overflowing_sub(b) {
            (val, false) => isize::try_from(val).unwrap(),
            (val, true) => unsafe { std::mem::transmute::<usize, isize>(val) },
        }
    }
    epoch::advance().unwrap();
    let allocated = stats::allocated::read().unwrap();
    let resident = stats::resident::read().unwrap();
    tracing::info!(
        "Memory: {}B / {}B{} (Allocated/Resident)",
        allocated,
        resident,
        if let Some((pa, pr)) = prev {
            format!(
                " (Δ {}B, {}B)",
                sub_to_isize(allocated, pa),
                sub_to_isize(resident, pr)
            )
        } else {
            "".to_string()
        }
    );
    (allocated, resident)
}

type Leaf = Vec<u8>;

/// Convert a `glTF` [Transform](gltf::scene::Transform) to a [nalgebra]
/// [Affine3].
///
/// An affine transformation is, in order, a non-uniform scaling, a rotation, and then a
/// translation.
///
/// A `glTF` transformation is stored either as an affine transformation matrix, or as separate
/// translation, rotation, and scale components. Therefore, the most general possible kind of
/// transformation is affine, which means that the *least* general kind of transformation we can
/// return is an [Affine3].
///
/// ## See Also
///
/// * [nalgebra's explanation of transformations](https://www.nalgebra.org/docs/user_guide/points_and_transformations/#transformations)
pub fn gltf_to_nalgebra(g: &gltf::scene::Transform) -> Affine3<f32> {
    match g {
        // the Matrix variant is stored as a column-major [[f32; 4]; 4], so we can just transmute
        // that into an [f32; 16] and use that directly.
        gltf::scene::Transform::Matrix { matrix: ref m } => {
            // the glTF spec states that matrix transformations *must* be decomposable to their
            // translation, rotation, and scale components. Therefore, a matrix from a compliant
            // glTF file can be converted directly to an Affine3.
            Affine3::<f32>::from_matrix_unchecked(Matrix4::from_column_slice(
                // arrays are stored contiguously, so, in memory, an [[f32; 4]; 4] is identical to
                // an [f32; 16], which means we can safely interpret one to the other.
                //
                // `std::mem::transmute` tells Rust's compiler that we want to interpret something of
                // type `A` as, instead, something of type `B`. It doesn't actually do anything at
                // runtime.
                unsafe { std::mem::transmute::<&[[f32; 4]; 4], &[f32; 16]>(m) }.as_slice(),
            ))
        }
        // this is a bit more complicated, because we have to convert these three components into
        // a single Transform3.
        gltf::scene::Transform::Decomposed {
            translation: ref trans, // [x, y, z]
            rotation: ref rot,      // unit quaternion, [x, y, z, w]
            scale: ref scl,         // [x, y, z]
        } => {
            // Store the resulting homogeneous Matrix4 as an Affine3.
            // We don't have to check for correctness, because we already know
            // that the matrix we're storing represents an affine transformation.
            Affine3::from_matrix_unchecked(
                // construct an Isometry (a rotation followed by a
                // translation) from `trans` and `rot`
                Isometry3::from_parts(
                    Translation3::from(*trans), // <- we can convert `trans` directly
                    Unit::new_unchecked(Quaternion::from(*rot)), // <- same with `rot`. The glTF spec
                                                                 // requires rotations to be stored as
                                                                 // unit quaternions, so we don't need
                                                                 // to validate that here.
                                                                 // Conveniently, nalgebra and glTF
                                                                 // use the same format for
                                                                 // quaternions.
                )
                // convert the Isometry3 to a homogenous Matrix4, so we can
                // apply the scaling (remember, an isometry is a rotation
                // followed by a translation; it, by definition, cannot have a
                // rotation, and the Isometry3 struct reflects that.)
                .to_homogeneous()
                // apply the scaling, resulting in a matrix M = Translation * Rotation * Scale.
                //
                // Reminder: when transforming a point using a matrix, the transformations
                // are applied to the point in the reverse of the order they were applied to the
                // matrix. So, a point transformed by a `TRS` (`Translation * Rotation * Scale`)
                // matrix is first scaled, then rotated, then translated. This is important because
                // applying those transformations in another order would produce a different end
                // result.
                .prepend_nonuniform_scaling(&Vector3::from(*scl)),
            )
        }
    }
}

/// Struct for keeping track of statistical data so we can print it out once we're done voxelizing.
#[derive(Debug, Copy, Clone)]
struct Stats {
    pub start_time: Instant,
    pub meshes: usize,
    pub points: usize,
    pub lines: usize,
    pub triangles: usize,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            meshes: Default::default(),
            points: Default::default(),
            lines: Default::default(),
            triangles: Default::default(),
        }
    }
}

/// Index a set of `glTF` mesh instances using an [Octree], then generate a voxel representation of
/// those meshes from that Octree.
pub fn main() {
    // initialize command-line interface
    let cli = cli::Cli::parse();
    crate::cli::initialize_tracing(&cli.log_filter, cli.log_format);

    // if we're using jemalloc, print out some memory usage stats and store the result for
    // comparison later
    #[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
    let memstats = trace_memory_stats(None);

    // prepare transformation applied to each mesh before processing
    let base_transform = Affine3::from_matrix_unchecked(cli.mesh_scale.to_homogeneous());

    // open all the gltf documents (we're going to store references to them in the tree, so they
    // need to live in memory longer than the tree)
    let docs: Vec<(&PathBuf, Gltf)> = cli
        .files
        .iter()
        .map(|p| {
            (
                p, // <- holding onto the file path so we can use it in tracing output
                Gltf::open(p)
                    .unwrap_or_else(|_| panic!("failed to deserialize {p:?} as glTF data")),
            )
        })
        .collect::<Vec<_>>();

    // storing buffer cache data outside the loop for the same reason as with the gltf documents
    let mut caches: HashMap<&Path, BufferCache> = HashMap::new();

    // build the octree
    let mut tree: VoxelOctree<Leaf, f32, u32> = VoxelOctree::new(cli.voxel_size);

    // keep track of stats so we can print it out at the end of the program
    let mut stats = Stats::default();

    for (path, doc) in docs.iter() {
        // enter a tracing span for this glTF document. This is just for nicer log output.
        let _doc_span =
            tracing::info_span!("glTF_document", path = path.as_os_str().to_str()).entered();

        // glTF data can be split into multiple files, which may be used more than once.
        // To keep things efficient, we'll use a cache for this data.
        let mut buffer_cache = caches.entry(path.as_path()).or_insert_with(|| {
            BufferCache::new(&doc, path)
                .unwrap_or_else(|_| panic!("failed to construct buffer cache for {path:?}"))
        });

        // gltf files are organized as a tree, where the root nodes are `scenes`, branch nodes are
        // `nodes`, and each `node` may have leaves of data, such as meshes or cameras.
        //
        // We need to loop through every mesh primitive so that we can add them to the octree,
        // so we have to descend through every node to find all mesh instances in the scene.
        for scene in doc.scenes() {
            // enter another tracing span for this scene; again, just for nicer log output
            let _scene_span =
                tracing::info_span!("glTF_scene", index = scene.index(), name = scene.name())
                    .entered();

            // recurse through all nodes in the scene, depth-first
            for scene_node in scene.nodes() {
                process_node(
                    &mut stats,
                    &mut tree,
                    &cli.voxel_size,
                    &mut buffer_cache,
                    scene_node.clone(),
                    &base_transform,
                )
                .unwrap();
            }
        }
    }

    let duration = Instant::now() - stats.start_time;

    #[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
    trace_memory_stats(Some(memstats));

    tracing::debug!(%tree, "done");

    tracing::info!(?stats, %duration, "done");

    pollster::block_on(preview::show_preview(&docs, &caches, tree));
}

/// Process a [Node] and its descendants into an [Octree].
#[tracing::instrument(name = "glTF_node", skip(tree, buffer_cache, node, parent_transform), fields(index = node.index(), name = node.name()))]
fn process_node<'data>(
    stats: &mut Stats,
    tree: &mut VoxelOctree<Leaf, f32, u32>,
    voxel_size: &Vector3<f32>,
    buffer_cache: &'data mut BufferCache<'_>,
    node: Node<'_>,
    parent_transform: &Affine3<f32>,
) -> Result<(), Error<u32, f32>> {
    tracing::trace!("processing node");
    // Each node has a transform relative to its parent. We need to keep track of this
    // so that we know the location of each mesh instance in world space. The glTF
    // library's [Transform] enum isn't very useful, so we convert it to a nalgebra
    // [Affine3].
    let transform = gltf_to_nalgebra(&node.transform()) * parent_transform;
    if let Some(mesh) = node.mesh() {
        process_mesh(stats, tree, voxel_size, buffer_cache, mesh, &transform)?;
    }
    for child in node.children() {
        process_node(stats, tree, voxel_size, buffer_cache, child, &transform)?;
    }
    Ok(())
}

/// Process a [`gltf::Mesh`] into an [Octree].
#[allow(clippy::needless_pass_by_value)]
fn process_mesh<'data>(
    stats: &mut Stats,
    tree: &mut VoxelOctree<Leaf, f32, u32>,
    voxel_size: &Vector3<f32>,
    buffer_cache: &'data mut BufferCache<'_>,
    mesh: gltf::Mesh<'_>,
    transform: &Affine3<f32>,
) -> Result<(), Error<u32, f32>> {
    tracing::info!("processing mesh");
    stats.meshes += 1;
    for primitive in mesh.primitives() {
        let mode = primitive.mode();
        let _p_span = tracing::trace_span!(
            "glTF_primitive",
            index = primitive.index(),
            mode = format!("{mode:?}")
        )
        .entered();
        let positions = match primitive.get(&Semantic::Positions) {
            Some(p) => buffer_cache
                .access(&p)?
                .try_as_slice::<Point3<f32>>()
                .map_err(BufferError::from)?,
            None => {
                tracing::error!("skipping primitive: no position attribute");
                continue;
            }
        };

        let indices = get_primitive_indices(buffer_cache, &primitive, positions.len())?;

        match mode {
            Mode::Points => {
                tracing::trace!("voxelizing points...");
                for point in indices
                    .map(|i| &positions[i as usize])
                    .map(|p| transform.transform_point(p))
                {
                    stats.points += 1;
                    tree.grow_to_contain(&point);
                    // this should never fail, because we just ensured that `point` lies within the
                    // space encompassed by `tree`
                    tree.node_at_mut(&point)?
                        .leaf_data_or_insert_with(Vec::default)?
                        .push(0);
                }
            }
            Mode::Lines => {
                for Line(a, b) in indices.map(|_i| todo!("lines from indices")) {
                    stats.lines += 1;
                    tree.grow_to_contain(&a);
                    tree.grow_to_contain(&b);
                    tree.node_at_mut(&a)?
                        .leaf_data_or_insert_with(Vec::default)?
                        .push(0);
                    tree.node_at_mut(&b)?
                        .leaf_data_or_insert_with(Vec::default)?
                        .push(0);
                }
            }
            Mode::LineLoop => todo!("process line loops"),
            Mode::LineStrip => todo!("process line strips"),
            Mode::Triangles => {
                let indices = indices.collect::<Vec<_>>();
                let amt = indices.len() / 3;
                stats.triangles += amt;
                tracing::trace!(%amt, "voxelizing triangles");
                for Triangle(a, b, c) in indices.chunks_exact(3).map(|i| {
                    Triangle(
                        transform.transform_point(&positions[i[0] as usize]),
                        transform.transform_point(&positions[i[1] as usize]),
                        transform.transform_point(&positions[i[2] as usize]),
                    )
                }) {
                    tree.grow_to_contain(&a);
                    tree.grow_to_contain(&b);
                    tree.grow_to_contain(&c);
                    tree.node_at_mut(&a)?
                        .leaf_data_or_insert_with(Vec::default)?
                        .push(0);
                    tree.node_at_mut(&b)?
                        .leaf_data_or_insert_with(Vec::default)?
                        .push(0);
                    tree.node_at_mut(&c)?
                        .leaf_data_or_insert_with(Vec::default)?
                        .push(0);
                }
            }
            Mode::TriangleStrip => todo!("process triangle strips"),
            Mode::TriangleFan => todo!("process triangle fans"),
        };
    }
    Ok(())
}

pub struct Line(Point3<f32>, Point3<f32>);
pub struct Triangle(Point3<f32>, Point3<f32>, Point3<f32>);

/// Get an iterator of buffer indices from a [Primitive](gltf::Primitive).
fn get_primitive_indices<'buf>(
    buffer_cache: &'buf BufferCache<'_>,
    primitive: &gltf::Primitive<'_>,
    position_count: usize,
) -> Result<Box<dyn Iterator<Item = u32> + 'buf>, BufferError> {
    match primitive.indices() {
        None => Ok(Box::new(0..(position_count as u32))),
        Some(ind_accessor) => {
            let ind_accessor = buffer_cache.access(&ind_accessor)?;
            match ind_accessor.data_type {
                DataType::U8 => {
                    let indices = ind_accessor.try_as_slice::<u8>()?;
                    // tracing::trace!("indices: {:?}", indices);
                    Ok(Box::new(indices.iter().copied().map(u32::from)))
                }
                DataType::U16 => {
                    let indices = ind_accessor.try_as_slice::<u16>()?;
                    // tracing::trace!("indices: {:?}", indices);
                    Ok(Box::new(indices.iter().copied().map(u32::from)))
                }
                DataType::U32 => {
                    let indices = ind_accessor.try_as_slice::<u32>()?;
                    // tracing::trace!("indices: {:?}", indices);
                    Ok(Box::new(indices.iter().copied()))
                }
                _ => unreachable!(), // anything else would be outside of the glTF spec
            }
        }
    }
}
