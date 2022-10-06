#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Indexed(u32),
    Rgba(f32, f32, f32, f32),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum VertexField {
    Position,
    Normal,
    Color,
    Uv,
}

// #[derive(Debug)]
// pub struct Triangle {
//     points: [WorldPoint; 3],
//     fields: HashMap<VertexField, Vec<Real>>,
// }

// impl Triangle {
//     pub fn voxelize(&self, tree: &mut Octree<Color>) {}
// }

pub fn main() {
    // let mut tree: Octree<Color> = Octree::new();
    // for tri in [] {}
}
