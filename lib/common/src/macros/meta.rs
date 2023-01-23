/// Duplicate an [item](https://doc.rust-lang.org/nightly/reference/items.html) `$imp` for each type
/// `$Real`, making it available within `$imp` under the alias `$RealAlias`.
///
/// Please use traits instead of this, where reasonable.
#[macro_export]
macro_rules! item_with {
    {$RealAlias:ident: $($Real:ty),+ => $imp:item} => {
        $(
            const _: () = { // anonymous module
                type $RealAlias = $Real;
                $imp
            };
        )+
    };
}
