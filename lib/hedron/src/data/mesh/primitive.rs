use std::collections::HashMap;

use crate::primitive::attribute::AttributeUsage;

pub mod attribute;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Points = 0,
    Lines = 1,
    LineLoop = 2,
    LineStrip = 3,
    Triangles = 4,
    TriangleStrip = 5,
    TriangleFan = 6,
}

#[derive(Debug, Clone)]
pub struct Primitive<AttrStore, MatRef> {
    /// The method by which vertices are interpreted as topological primitives
    mode: Mode,
    /// Indices of each vertex within each attribute. If empty, equivalent to [0, 1, 2, 3, ...]
    indices: Vec<u32>,
    /// Vertex attribute data
    attributes: HashMap<AttributeUsage, AttrStore>,
    /// Material with which this primitive is rendered
    material: MatRef,
    // ignoring morph targets for now
}

eightfold_common::impl_index!(self: Primitive<A, M> -> A, i: AttributeUsage;
    &self.attributes[&i];
    self.attributes.get_mut(&i).unwrap());

eightfold_common::impl_index!(self: Primitive<A, M> -> A, i: &AttributeUsage;
    &self.attributes[i];
    self.attributes.get_mut(i).unwrap());

impl<A, M> Primitive<A, M> {
    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn attributes(&self) -> &HashMap<AttributeUsage, A> {
        &self.attributes
    }

    pub fn material(&self) -> &M {
        &self.material
    }

    pub fn get_attr(&self, attr: &AttributeUsage) -> Option<&A> {
        self.attributes.get(attr)
    }

    pub fn get_attr_mut(&mut self, attr: &AttributeUsage) -> Option<&mut A> {
        self.attributes.get_mut(attr)
    }
}

impl<A, M> Primitive<A, M> {
    pub fn iter_attr(&self, attr: &AttributeUsage) -> Option<()> {
        None
    }
}
