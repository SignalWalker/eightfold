//! Utilities for deserializing glTF data as a [DataSet].
//!
//! # Notes
//!
//! * Coordinate system: right-handed; the same as is used elsewhere in this library.
//! * glTF data *may* contain scenes, and it *may* specify a default scene, but these aren't required by the format.
//!   * Sceneless data will be considered a library of entity data; ex. meshes, materials
//!
//! # See Also
//!
//! * [glTF 2.0 Spec](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
//! * [glTF Reference Guide](https://www.khronos.org/files/gltf20-reference-guide.pdf)

use eightfold_common::ArrayIndex;
use gltf::Gltf;
use num_traits::AsPrimitive;

use crate::DataSet;

impl<Real, Idx: ArrayIndex> TryFrom<Gltf> for DataSet<Real, Idx>
where
    usize: AsPrimitive<Idx>,
{
    type Error = ();
    fn try_from(doc: Gltf) -> Result<Self, Self::Error> {
        // let mut scenes = StableVec::new();
        // for scene in doc.scenes() {
        //     scenes.insert(scene.index(), todo!());
        // }

        Ok(Self {
            default_scene: doc.default_scene().map(|s| s.index().as_()),
            ..Default::default()
        })
    }
}
