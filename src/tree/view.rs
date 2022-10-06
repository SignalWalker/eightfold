use crate::{error::Error, Octree, Proxy, ProxyData};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Copy, Clone)]
pub struct View<'tree, Tree: 'tree> {
    tree: Tree,
    target: u32,
    _lifetime: PhantomData<&'tree ()>,
}

impl<'tree, T: 'tree, Tree: Deref<Target = Octree<T>> + 'tree> View<'tree, Tree> {
    pub(crate) fn new(tree: Tree, target: u32) -> Self {
        Self {
            tree,
            target,
            _lifetime: PhantomData::default(),
        }
    }

    pub fn proxy(&self) -> &Proxy {
        &self.tree.proxies[self.target as usize]
    }

    pub fn parent(&self) -> Result<Self, Error>
    where
        Tree: Copy,
    {
        Ok(Self::new(self.tree, {
            let pid = self.proxy().parent;
            if pid == self.target {
                return Err(Error::ParentOfRoot);
            }
            pid
        }))
    }

    pub fn child(&self, index: u8) -> Result<Self, Error>
    where
        Tree: Copy,
    {
        Ok(Self::new(
            self.tree,
            match self.proxy().data {
                ProxyData::Branch(c) => {
                    *c.get(index as usize).ok_or(Error::ChildOutOfRange(index))?
                }
                _ => return Err(Error::NoChildren(self.target)),
            },
        ))
    }

    pub fn children(&self) -> Box<dyn Iterator<Item = Self> + '_>
    where
        Tree: Copy,
    {
        match self.proxy().data {
            ProxyData::Branch(c) => {
                Box::new(c.into_iter().map(|target| Self::new(self.tree, target)))
            }
            _ => Box::new([].into_iter()),
        }
    }
    pub fn data(&self) -> Option<&T> {
        match self.proxy().data {
            ProxyData::Leaf(lid) => Some(&self.tree.leaf_data[lid as usize]),
            _ => None,
        }
    }
}

impl<'tree, T: 'tree, Tree: DerefMut<Target = Octree<T>> + 'tree> View<'tree, Tree> {
    pub fn parent_mut(&mut self) -> Result<&mut Self, Error> {
        let pid = self.proxy().parent;
        if pid == self.target {
            return Err(Error::ParentOfRoot);
        }
        self.target = pid;
        Ok(self)
    }

    pub fn child_mut(&mut self, index: u8) -> Result<&mut Self, Error> {
        self.target = match self.proxy().data {
            ProxyData::Branch(c) => *c.get(index as usize).ok_or(Error::ChildOutOfRange(index))?,
            _ => return Err(Error::NoChildren(self.target)),
        };
        Ok(self)
    }
}
