use eightfold_common::ArrayIndex;
use gltf::Gltf;
use num_traits::AsPrimitive;

use crate::DataSet;

impl<Idx: ArrayIndex> TryFrom<Gltf> for DataSet<Idx>
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
