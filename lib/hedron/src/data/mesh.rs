pub mod primitive;

/// A set of vertices, edges, and faces.
///
/// # Considerations
///
/// * Must store glTF meshes losslessly
/// * Should allocate as little memory as possible
///
/// # Characteristics
///
/// * A vertex may have some number of attributes
///   * TODO :: must a vertex have at least one attribute?
pub struct Mesh {
    primitives: Vec<()>,
}
