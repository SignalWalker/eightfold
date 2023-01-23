use crate::primitive::attribute::{AttributeComponent, AttributeType};

pub mod dynamic;

#[derive(Debug, thiserror::Error)]
pub enum DynamicAttributeError {
    #[error(
        "cannot construct AttrStore with requested type; base slice not aligned to requested type"
    )]
    Alignment,
    #[error("cannot construct AttrStore with requested type; base slice not properly sized for requested type")]
    Size,
    #[error("cannot borrow AttrStore as slice of requested type (width mismatch)")]
    Width,
    #[error("cannot borrow AttrStore as slice of requested type (component mismatch)")]
    Component,
    #[error("cannot borrow AttrStore as slice of a zero-sized type")]
    BorrowedAsZST,
}

pub trait DynamicAttributeStorage {
    fn len(&self) -> usize;
    fn attr_type(&self) -> AttributeType;
    fn attr_comp(&self) -> AttributeComponent;
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn try_slice<T>(&self) -> Result<&[T], DynamicAttributeError>;
}
