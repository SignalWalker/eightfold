use std::fmt::Display;

use eightfold_common::ArrayIndex;
use num_traits::AsPrimitive;

use crate::Octant;
use crate::Octree;

use crate::OctreeSlice;
use crate::Proxy;
use crate::ProxyData;

impl<T: std::fmt::Debug, Idx: ArrayIndex> Display for Octree<T, Idx>
where
    u8: AsPrimitive<Idx>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Octree (root: {:?}, ({} branches, {} leaves) / {} proxies) {{",
            self.root,
            self.branch_data.len_init(),
            self.leaf_data.len_init(),
            self.proxies.len_init(),
        )?;
        let mut stack = vec![(self.root, self.proxies[self.root.as_()], 0)];
        while let Some((idx, prox, depth)) = stack.pop() {
            let indent = std::iter::repeat("  ").take(depth + 1).collect::<String>();
            match prox.data {
                ProxyData::Void => write!(f, "\n{indent}<V @ {idx:?}>")?,
                ProxyData::Leaf(l_idx) => write!(
                    f,
                    "\n{indent}<L @ {idx:?}> {:?}",
                    self.leaf_data[l_idx.as_()]
                )?,
                ProxyData::Branch(ch_idx) => {
                    write!(f, "\n{indent}<B @ {idx:?}>")?;
                    for child in self.branch_data[ch_idx.as_()]
                        .iter()
                        .rev()
                        .map(|i| (*i, self.proxies[i.as_()], depth + 1))
                    {
                        stack.push(child);
                    }
                }
            }
        }
        write!(f, "\n}}")
    }
}
