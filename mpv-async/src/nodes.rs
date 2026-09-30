use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    fmt::Debug,
    hint::unreachable_unchecked,
    marker::PhantomData,
    ops::Deref,
    ptr::{null, null_mut},
    slice,
};

use mpv_sys::{
    mpv_byte_array, mpv_event_property, mpv_format, mpv_free, mpv_node, mpv_node__bindgen_ty_1,
    mpv_node_list,
};
use valuable::Valuable;

#[doc(hidden)]
pub mod macro_support {
    pub use std::ffi::CStr;
}

/**
 * [`mpv_format`] repackaged into an enum
 *  */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpvFormat {
    ByteArray,
    Double,
    Flag,
    Int64,
    Node,
    NodeArray,
    NodeMap,
    None,
    String,
}

impl MpvFormat {
    #[inline]
    pub(crate) const fn ffi(self) -> mpv_format {
        match self {
            Self::ByteArray => mpv_format::MPV_FORMAT_BYTE_ARRAY,
            Self::Double => mpv_format::MPV_FORMAT_DOUBLE,
            Self::Flag => mpv_format::MPV_FORMAT_FLAG,
            Self::Int64 => mpv_format::MPV_FORMAT_INT64,
            Self::Node => mpv_format::MPV_FORMAT_NODE,
            Self::NodeArray => mpv_format::MPV_FORMAT_NODE_ARRAY,
            Self::NodeMap => mpv_format::MPV_FORMAT_NODE_MAP,
            Self::None => mpv_format::MPV_FORMAT_NONE,
            Self::String => mpv_format::MPV_FORMAT_STRING,
        }
    }
}

impl From<MpvFormat> for mpv_format {
    #[inline]
    fn from(value: MpvFormat) -> Self {
        value.ffi()
    }
}

#[repr(transparent)]
/**
 * Safe wrapper for mpv nodes.
 *
 * Only meant to be used through a
 * reference and only useful for reading nodes
 *  */
pub struct MpvNode {
    pub(crate) inner: mpv_node,
}

unsafe impl Send for MpvNode {}
unsafe impl Sync for MpvNode {}

impl MpvNode {
    /**
     * Differentisate between the different node formats
     *  */
    pub fn differentiate(&self) -> MpvNodeRef<'_> {
        unsafe { MpvNodeRef::new(&self.inner) }
    }
    /**
     * # Safety
     * Node must be valid.
     *  */
    #[must_use]
    pub const unsafe fn unsafe_new(inner: mpv_node) -> Self {
        Self { inner }
    }
    /**
     * # Safety
     * Node must be valid.
     *  */
    #[must_use]
    pub const unsafe fn unsafe_new_ref(inner: &mpv_node) -> &Self {
        unsafe { (&raw const *inner).cast::<Self>().as_ref_unchecked() }
    }
    #[must_use]
    pub const fn inner(&self) -> &mpv_node {
        &self.inner
    }
}

impl Debug for MpvNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.differentiate(), f)
    }
}

#[must_use]
#[derive(Clone, Copy)]
/**
 * Mpv node content
 *  */
pub enum MpvNodeRef<'r> {
    String(&'r CStr),
    Bool(bool),
    Int(i64),
    Float(f64),
    Array(&'r [MpvNode]),
    Map(MpvNodeMapRef<'r>),
    Bytes(&'r [u8]),
    None,
}

impl<'r> MpvNodeRef<'r> {
    #[must_use]
    pub const fn string(self) -> Option<&'r CStr> {
        if let MpvNodeRef::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn bool(self) -> Option<bool> {
        if let MpvNodeRef::Bool(v) = self {
            Some(v)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn int(self) -> Option<i64> {
        if let MpvNodeRef::Int(v) = self {
            Some(v)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn float(self) -> Option<f64> {
        if let MpvNodeRef::Float(v) = self {
            Some(v)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn array(self) -> Option<&'r [MpvNode]> {
        if let MpvNodeRef::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn map(self) -> Option<MpvNodeMapRef<'r>> {
        if let MpvNodeRef::Map(v) = self {
            Some(v)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn bytes(self) -> Option<&'r [u8]> {
        if let MpvNodeRef::Bytes(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl Debug for MpvNodeRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(arg0) => Debug::fmt(arg0, f),
            Self::Bool(arg0) => Debug::fmt(arg0, f),
            Self::Int(arg0) => Debug::fmt(arg0, f),
            Self::Float(arg0) => Debug::fmt(arg0, f),
            Self::Array(arg0) => Debug::fmt(arg0, f),
            Self::Map(arg0) => Debug::fmt(arg0, f),
            Self::Bytes(arg0) => Debug::fmt(arg0, f),
            Self::None => write!(f, "None"),
        }
    }
}

unsafe impl Send for MpvNodeRef<'_> {}
unsafe impl Sync for MpvNodeRef<'_> {}

#[derive(Clone, Copy)]
/**
 * Read only mpv node map
 *
 * Mpv nodes do not provide efficient indexing like the normal rust \*Maps.
 * Instead it behaves more like `&Vec<&[CStr],[MpvNode]>`.
 * Mainly used through it's iterators.
 * */
pub struct MpvNodeMapRef<'r> {
    len: usize,
    nodes: *const MpvNode,
    keys: *const *const c_char,
    lifetime: PhantomData<&'r MpvNode>,
}

unsafe impl Send for MpvNodeMapRef<'_> {}
unsafe impl Sync for MpvNodeMapRef<'_> {}

impl<'r> MpvNodeMapRef<'r> {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Iterate over key value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&'r CStr, &'r MpvNode)> + Clone + use<'r> {
        let keys = self.keys;
        let nodes = self.nodes;
        std::range::Range::from(0..self.len)
            .iter()
            .map(move |i| unsafe {
                let key = CStr::from_ptr(keys.add(i).read());
                let val = nodes.add(i).as_ref_unchecked();
                (key, val)
            })
    }
    /// Iterate over map keys
    pub fn keys(&self) -> impl Iterator<Item = &'r CStr> + Clone + 'r {
        unsafe {
            slice::from_raw_parts(self.keys, self.len)
                .iter()
                .map(|p| CStr::from_ptr(*p))
        }
    }
    /// Iterate over map values
    pub fn nodes(&self) -> impl Iterator<Item = &'r MpvNode> + Clone + 'r {
        unsafe { slice::from_raw_parts(self.nodes, self.len).iter() }
    }
}

impl Debug for MpvNodeMapRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<'r> MpvNodeRef<'r> {
    /**
     * Construct differentiated enum from [`mpv_node`] struct
     *
     * # Safety
     *  `node` must be a valid [`mpv_node`]
     *  */
    pub unsafe fn new(node: &'r mpv_node) -> Self {
        #[allow(non_upper_case_globals)]
        match node.format {
            mpv_format::MPV_FORMAT_STRING => unsafe { Self::String(CStr::from_ptr(node.u.string)) },
            mpv_format::MPV_FORMAT_FLAG => unsafe {
                let val = node.u.flag;
                Self::Bool(match val {
                    0 => false,
                    1 => true,
                    _ => unreachable_unchecked(),
                })
            },
            mpv_format::MPV_FORMAT_INT64 => unsafe { Self::Int(node.u.int64) },
            mpv_format::MPV_FORMAT_DOUBLE => unsafe { Self::Float(node.u.double_) },
            mpv_format::MPV_FORMAT_NODE_ARRAY => unsafe {
                let list = node.u.list.as_ref_unchecked();
                let len: usize = list.num.try_into().unwrap_unchecked();
                let slice = if len == 0 {
                    &[]
                } else {
                    std::slice::from_raw_parts(list.values.cast_const().cast::<MpvNode>(), len)
                };
                Self::Array(slice)
            },
            mpv_format::MPV_FORMAT_NODE_MAP => unsafe {
                let list = node.u.list.as_ref_unchecked();
                let len: usize = list.num.try_into().unwrap_unchecked();
                Self::Map(MpvNodeMapRef {
                    len,
                    nodes: list.values.cast_const().cast(),
                    keys: list.keys.cast_const().cast(),
                    lifetime: PhantomData,
                })
            },
            mpv_format::MPV_FORMAT_BYTE_ARRAY => unsafe {
                let array = node.u.ba.as_ref_unchecked();
                Self::Bytes(std::slice::from_raw_parts(
                    array.data.cast_const().cast(),
                    array.size,
                ))
            },
            mpv_format::MPV_FORMAT_NONE => Self::None,
            _ => panic!("unknown format inside a mpv node"),
        }
    }
    /**
     * Construct differentiated enum from [`mpv_event_property`]
     * # Safety
     * `property` must be a valid [`mpv_event_property`]
     *  */
    pub unsafe fn from_property(property: &'r mpv_event_property) -> Self {
        match property.format {
            mpv_format::MPV_FORMAT_STRING => unsafe {
                Self::String(CStr::from_ptr(property.data.cast::<*const c_char>().read()))
            },
            mpv_format::MPV_FORMAT_FLAG => unsafe {
                let val = property.data.cast::<c_int>().read();
                Self::Bool(match val {
                    0 => false,
                    1 => true,
                    _ => unreachable_unchecked(),
                })
            },
            mpv_format::MPV_FORMAT_INT64 => unsafe {
                Self::Int(property.data.cast::<i64>().read())
            },
            mpv_format::MPV_FORMAT_DOUBLE => unsafe {
                Self::Float(property.data.cast::<f64>().read())
            },
            mpv_format::MPV_FORMAT_NODE_ARRAY => unsafe {
                let list = property.data.cast::<mpv_node_list>().as_ref_unchecked();
                let len: usize = list.num.try_into().unwrap_unchecked();
                let slice = if len == 0 {
                    &[]
                } else {
                    std::slice::from_raw_parts(list.values.cast_const().cast::<MpvNode>(), len)
                };
                Self::Array(slice)
            },
            mpv_format::MPV_FORMAT_NODE_MAP => unsafe {
                let list = property.data.cast::<mpv_node_list>().as_ref_unchecked();
                let len: usize = list.num.try_into().unwrap_unchecked();
                Self::Map(MpvNodeMapRef {
                    len,
                    nodes: list.values.cast_const().cast(),
                    keys: list.keys.cast_const().cast(),
                    lifetime: PhantomData,
                })
            },
            mpv_format::MPV_FORMAT_BYTE_ARRAY => unsafe {
                let array = property.data.cast::<mpv_byte_array>().as_ref_unchecked();
                Self::Bytes(std::slice::from_raw_parts(
                    array.data.cast_const().cast(),
                    array.size,
                ))
            },
            mpv_format::MPV_FORMAT_NONE => Self::None,
            _ => panic!("unknown format inside a mpv node"),
        }
    }
}

/**
 *
 * [`MpvNode`] suitable for constructing owned node structures.
 *
 * Typically used by storing them on the stack before passing the to libmpv apis.
 * Can be safely constructed through [`ToMpvNode::node()`]
 *  */
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct MpvStackNode<'r> {
    inner: mpv_node,
    inner_lifetime: PhantomData<&'r MpvNode>,
}

impl MpvStackNode<'_> {
    /**
     * Unsafe constructor with unbounded lifetime
     *
     * *You should prefer using [`ToMpvNode::node()`] instead*
     * # Safety
     * `inner` must be valid for the lifetime `r`
     *  */
    #[must_use]
    pub const unsafe fn new_unsafe<'r>(inner: mpv_node) -> MpvStackNode<'r> {
        MpvStackNode {
            inner,
            inner_lifetime: PhantomData,
        }
    }
}

impl Deref for MpvStackNode<'_> {
    type Target = MpvNode;

    fn deref(&self) -> &Self::Target {
        unsafe { MpvNode::unsafe_new_ref(&self.inner) }
    }
}

/**
 * Construct Ownded [`MpvNode`] from inner value
 *  */
pub trait ToMpvNode<'r> {
    /**
     * Construct [`MpvStackNode`] from value.
     * The format of the resulting Node depends on the type of `self`.
     *  */
    fn node(self) -> MpvStackNode<'r>;
}

impl<'r> ToMpvNode<'r> for &'r CStr {
    fn node(self) -> MpvStackNode<'r> {
        MpvStackNode {
            inner: mpv_node {
                u: mpv_node__bindgen_ty_1 {
                    string: self.as_ptr().cast_mut(),
                },
                format: mpv_format::MPV_FORMAT_STRING,
            },
            inner_lifetime: PhantomData,
        }
    }
}
impl<'r> ToMpvNode<'r> for &'r CString {
    fn node(self) -> MpvStackNode<'r> {
        self.as_c_str().node()
    }
}

impl ToMpvNode<'static> for bool {
    fn node(self) -> MpvStackNode<'static> {
        let flag = i32::from(self);
        MpvStackNode {
            inner: mpv_node {
                u: mpv_node__bindgen_ty_1 { flag },
                format: mpv_format::MPV_FORMAT_FLAG,
            },
            inner_lifetime: PhantomData,
        }
    }
}

impl ToMpvNode<'static> for i64 {
    fn node(self) -> MpvStackNode<'static> {
        MpvStackNode {
            inner: mpv_node {
                u: mpv_node__bindgen_ty_1 { int64: self },
                format: mpv_format::MPV_FORMAT_INT64,
            },
            inner_lifetime: PhantomData,
        }
    }
}

impl ToMpvNode<'static> for f64 {
    fn node(self) -> MpvStackNode<'static> {
        MpvStackNode {
            inner: mpv_node {
                u: mpv_node__bindgen_ty_1 { double_: self },
                format: mpv_format::MPV_FORMAT_DOUBLE,
            },
            inner_lifetime: PhantomData,
        }
    }
}

//TODO: simplyfy once super let is stable
#[cfg(feature = "macros")]
/**
 * Construct [`MpvNodeList`] from array stored on the stack
 *
 * # Usage
 *
 * All values must implement [`ToMpvNode`]
 *
 * ```
 * # use mpv_async::nodes::mpv_node_list;
 * // construct empty list in variable `name`
 * mpv_node_list!(name; []);
 * // with actual values
 * mpv_node_list!(name; [true, 5i64, c"test"]);
 * ```
 * ## Lifetime Extension
 * Temporaries created directly in the macro invocation are automatically extended.
 * This only applies to the [set of generally extended patterns](https://doc.rust-lang.org/reference/destructors.html#r-destructors.scope.lifetime-extension)
 * and [constant promoted values](https://doc.rust-lang.org/reference/destructors.html#r-destructors.scope.const-promotion).
 * Temporaries created in call expressions are one example where lifetime extension is not applied.
 *
 * ```
 * # use mpv_async::nodes::{mpv_node_list,MpvByteArray};
 * let bytes = [1u8,2,3,4,5];
 * // lifetime extension
 * mpv_node_list!(name; [&MpvByteArray::new(&bytes)]);
 * // u8 array is constant promoted
 * mpv_node_list!(name; [&MpvByteArray::new(&[1u8,2,3,4,5])]);
 * ```
 * ```compile_fail
 * # use mpv_async::nodes::{mpv_node_list,MpvByteArray};
 * //in contrast to arrays, Vec is not constant promoted
 * mpv_node_list!(name; [&MpvByteArray::new(&vec![1u8,2,3,4,5])]);
 * ```
 **/
#[doc(hidden)]
#[macro_export]
macro_rules! mpv_node_list {
    ($var:ident; []) => {
        let $var = $crate::nodes::MpvNodeList::new(&[]);
    };
    ($var:ident; [$($v:tt)+]) => {
        $crate::macros::mpv_node_list_internal!($var; $crate::nodes ; $($v)*)
    };
}
#[doc(inline)]
pub use crate::mpv_node_list;

/**
 * List of [`MpvNode`] that can be converted to [`MpvStackNode`].
 **/
#[repr(transparent)]
pub struct MpvNodeList<'r> {
    inner: mpv_node_list,
    inner_lifetime: PhantomData<&'r MpvNode>,
}

impl<'r> MpvNodeList<'r> {
    /// Create list from slice of nodes.
    ///
    /// The slice length must be equal to or below [`i32::MAX`].
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub const fn try_new(content: &'r [MpvStackNode<'r>]) -> Option<Self> {
        if content.len() > (i32::MAX as usize) {
            return None;
        }
        Some(Self {
            inner: mpv_node_list {
                num: content.len() as i32,
                values: content.as_ptr().cast_mut().cast(),
                keys: null_mut(),
            },
            inner_lifetime: PhantomData,
        })
    }
    /// Create list from slice of Nodes.
    ///
    /// Panics if slice length is grater than [`i32::MAX`].
    #[must_use]
    pub const fn new(content: &'r [MpvStackNode<'r>]) -> Self {
        Self::try_new(content).expect("Node list is too large")
    }
}

impl<'r> From<&'r [MpvStackNode<'r>]> for MpvNodeList<'r> {
    fn from(value: &'r [MpvStackNode<'r>]) -> Self {
        Self::new(value)
    }
}

impl<'r> ToMpvNode<'r> for &'r MpvNodeList<'r> {
    fn node(self) -> MpvStackNode<'r> {
        MpvStackNode {
            inner: mpv_node {
                u: mpv_node__bindgen_ty_1 {
                    list: (&raw const self.inner).cast_mut(),
                },
                format: mpv_format::MPV_FORMAT_NODE_ARRAY,
            },
            inner_lifetime: PhantomData,
        }
    }
}

/// [`CStr`] wrapper for [`MpvNodeMap`]
///
/// [`mpv_node_map`] requires a slice of [`c_char`] ptrs, but [`CStr`] is not a ptr.
/// To be used in a node map, it must be wrapped in this (which is abi compatible to [`c_char`] but with an attached lifetime).
#[repr(transparent)]
pub struct CStrPtr<'r> {
    inner: *mut c_char,
    inner_lifetime: PhantomData<&'r MpvNode>,
}

impl<'r> CStrPtr<'r> {
    #[must_use]
    pub const fn new(cstr: &'r CStr) -> Self {
        Self {
            inner: cstr.as_ptr().cast_mut(),
            inner_lifetime: PhantomData,
        }
    }
}

impl<'r> From<&'r CStr> for CStrPtr<'r> {
    fn from(value: &'r CStr) -> Self {
        Self::new(value)
    }
}

//TODO: simplyfy once super let is stable
#[cfg(feature = "macros")]
/**
 * Construct [`MpvNodeMap`] from arrays stored on the stack.
 * This enables construction from something that looks like a mixed key and value array.
 *
 * # Usage
 * ```
 * # use mpv_async::nodes::mpv_node_map;
 * // construct empty map in variable `name`
 * mpv_node_map!(name; {});
 * // with actual values
 * mpv_node_map!(name; {
 *     c"flag": true,
 *     c"int": 5164,
 *     c"string": c"test"
 * });
 * ```
 * ## Lifetime Extension
 * Temporaries created directly in the macro invocation are automatically extended.
 * This only applies to the [set of generally extended patterns](https://doc.rust-lang.org/reference/destructors.html#r-destructors.scope.lifetime-extension)
 * and [constant promoted values](https://doc.rust-lang.org/reference/destructors.html#r-destructors.scope.const-promotion).
 * Temporaries created in call expressions are one example where lifetime extension is not applied.
 * ```
 * # use mpv_async::nodes::{mpv_node_map,MpvByteArray, ToMpvNode};
 * let bytes = [1u8,2,3,4,5];
 * // lifetime extension
 * mpv_node_map!(name; {&c"byte_array".to_owned(): &MpvByteArray::new(&bytes)});
 * // u8 array is constant promoted
 * mpv_node_map!(name; {&c"byte_array".to_owned(): &MpvByteArray::new(&[1u8,2,3,4,5])});
 * ```
 * ```compile_fail
 * # use mpv_async::nodes::{mpv_node_map,MpvByteArray, ToMpvNode};
 * //in contrast to arrays, Vec is not constant promoted
 * mpv_node_map!(name; {c"byte_array": &MpvByteArray::new(&vec![1u8,2,3,4,5])});
 * ```
 *  */
#[doc(hidden)]
#[macro_export]
macro_rules! mpv_node_map {
    ($var: ident; {}) => {
        let $var = $crate::nodes::MpvNodeMap::new(&[],&[]);
    };
    ($var: ident; {$($kv:tt)*}) => {
        $crate::macros::mpv_node_map_internal!($var; $crate::nodes ; $($kv)*)
    };
}
#[doc(inline)]
pub use crate::mpv_node_map;

/**
 * Map of [`CStr`] and [`MpvNode`] that can be converted to [`MpvStackNode`].
 *  */
#[repr(transparent)]
pub struct MpvNodeMap<'r> {
    inner: mpv_node_list,
    inner_lifetime: PhantomData<&'r MpvNode>,
}

impl<'r> MpvNodeMap<'r> {
    /// Create list from slices of strings and nodes.
    ///
    /// The slice length must be equal to or below [`i32::MAX`]
    /// and both slices must be of equal length.
    #[must_use]
    pub fn try_new(keys: &'r [CStrPtr<'r>], vals: &'r [MpvStackNode<'r>]) -> Option<Self> {
        if keys.len() != vals.len() {
            return None;
        }
        Some(Self {
            inner: mpv_node_list {
                num: vals.len().try_into().ok()?,
                values: vals.as_ptr().cast_mut().cast(),
                keys: keys.as_ptr().cast_mut().cast(),
            },
            inner_lifetime: PhantomData,
        })
    }
    /**
     * Create map from cstr and node slices.
     *
     * Panics if the slice lengths are above [`i32::MAX`]
     * or the slices don't have equal length.
     *  */
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub const fn new(keys: &'r [CStrPtr<'r>], vals: &'r [MpvStackNode<'r>]) -> Self {
        assert!(
            keys.len() == vals.len(),
            "size mismatch between keys and vals"
        );
        assert!(keys.len() <= (i32::MAX as usize), "Node list is too large");
        Self {
            inner: mpv_node_list {
                num: vals.len() as i32,
                values: vals.as_ptr().cast_mut().cast(),
                keys: keys.as_ptr().cast_mut().cast(),
            },
            inner_lifetime: PhantomData,
        }
    }
}

impl<'r> ToMpvNode<'r> for &'r MpvNodeMap<'r> {
    fn node(self) -> MpvStackNode<'r> {
        MpvStackNode {
            inner: mpv_node {
                u: mpv_node__bindgen_ty_1 {
                    list: (&raw const self.inner).cast_mut(),
                },
                format: mpv_format::MPV_FORMAT_NODE_MAP,
            },
            inner_lifetime: PhantomData,
        }
    }
}

/**
 * `&[u8]` repackaged to be consumable by mpv
 *  */
#[repr(transparent)]
pub struct MpvByteArray<'r> {
    inner: mpv_byte_array,
    inner_lifetime: PhantomData<&'r MpvNode>,
}

impl<'r> MpvByteArray<'r> {
    #[must_use]
    pub const fn new(val: &'r [u8]) -> Self {
        Self {
            inner: mpv_byte_array {
                data: val.as_ptr().cast_mut().cast(),
                size: val.len(),
            },
            inner_lifetime: PhantomData,
        }
    }
}

impl<'r> From<&'r [u8]> for MpvByteArray<'r> {
    fn from(value: &'r [u8]) -> Self {
        Self::new(value)
    }
}

impl<'r> ToMpvNode<'r> for &'r MpvByteArray<'r> {
    fn node(self) -> MpvStackNode<'r> {
        MpvStackNode {
            inner: mpv_node {
                u: mpv_node__bindgen_ty_1 {
                    ba: (&raw const self.inner).cast_mut(),
                },
                format: mpv_format::MPV_FORMAT_BYTE_ARRAY,
            },
            inner_lifetime: PhantomData,
        }
    }
}

impl ToMpvNode<'static> for () {
    fn node(self) -> MpvStackNode<'static> {
        MpvStackNode {
            inner: mpv_node {
                u: mpv_node__bindgen_ty_1 { flag: 0 },
                format: mpv_format::MPV_FORMAT_NONE,
            },
            inner_lifetime: PhantomData,
        }
    }
}

impl<'r> ToMpvNode<'r> for MpvStackNode<'r> {
    fn node(self) -> Self {
        self
    }
}

impl<'r> ToMpvNode<'r> for &'r MpvNode {
    fn node(self) -> MpvStackNode<'r> {
        MpvStackNode {
            inner: mpv_node {
                u: self.inner.u,
                format: self.inner.format,
            },
            inner_lifetime: PhantomData,
        }
    }
}

/**
 * Sending a value to mpv
 * # Safety
 * the `FORMAT` and `MpvType` must fit together
 *  */
pub unsafe trait ToFormat {
    /// `mpv_format` value
    const FORMAT: mpv_format;
    /// prepare self for consumption by mpv
    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T;
}

/**
 * Receiving a value from mpv
 * # Safety
 * the `FORMAT` and `MpvType` must fit together
 *  */
pub unsafe trait FromFormat {
    /// `mpv_format` value
    const FORMAT: mpv_format;
    /// A reference to a value of this type will be passed to mpv.
    /// It must be zero initializable
    type MpvType;
    /// if `MpvType` and self differ (for example FLAG) convert to self
    fn finanlize(receiver: Self::MpvType) -> Self;
}

unsafe impl ToFormat for () {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_NONE;

    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T {
        f(null())
    }
}
unsafe impl FromFormat for () {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_NONE;
    fn finanlize(_receiver: Self::MpvType) -> Self {}
    type MpvType = u8;
}

unsafe impl ToFormat for i64 {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_INT64;
    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T {
        f((&raw const self).cast())
    }
}
unsafe impl FromFormat for i64 {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_INT64;
    type MpvType = Self;
    fn finanlize(receiver: Self::MpvType) -> Self {
        receiver
    }
}

unsafe impl ToFormat for f64 {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_DOUBLE;
    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T {
        f((&raw const self).cast())
    }
}
unsafe impl FromFormat for f64 {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_DOUBLE;
    type MpvType = Self;
    fn finanlize(receiver: Self::MpvType) -> Self {
        receiver
    }
}

unsafe impl ToFormat for bool {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_FLAG;

    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T {
        let this: c_int = self.into();
        f((&raw const this).cast())
    }
}
unsafe impl FromFormat for bool {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_FLAG;
    type MpvType = c_int;
    fn finanlize(receiver: Self::MpvType) -> Self {
        unsafe {
            match receiver {
                0 => false,
                1 => true,
                _ => unreachable_unchecked(),
            }
        }
    }
}

unsafe impl ToFormat for &CStr {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_STRING;
    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T {
        let p = self.as_ptr();
        f((&raw const p).cast())
    }
}

/**
 * [`CString`] owned by mpv
 *  */
#[repr(transparent)]
pub struct MpvString {
    inner: *const c_char,
}

impl Deref for MpvString {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        unsafe { CStr::from_ptr(self.inner) }
    }
}

impl Drop for MpvString {
    fn drop(&mut self) {
        unsafe { mpv_free(self.inner.cast_mut().cast()) }
    }
}

unsafe impl FromFormat for MpvString {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_STRING;
    type MpvType = Self;
    fn finanlize(receiver: Self::MpvType) -> Self {
        receiver
    }
}

unsafe impl ToFormat for &MpvNode {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_NODE;
    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T {
        f((&raw const self.inner).cast())
    }
}

unsafe impl ToFormat for &MpvStackNode<'_> {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_NODE;

    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T {
        self.deref().do_with(f)
    }
}

/**
 * [`MpvNode`] owned by mpv
 *  */
#[repr(transparent)]
pub struct MpvOwnedNode {
    inner: MpvNode,
}

impl MpvOwnedNode {
    /**
     * # Safety
     * `mpv_node` must have been constructed correctly
     *  */
    #[must_use]
    pub const unsafe fn new(inner: mpv_node) -> Self {
        Self {
            inner: MpvNode { inner },
        }
    }
}

impl Drop for MpvOwnedNode {
    fn drop(&mut self) {
        if self.inner.inner.format != mpv_format::MPV_FORMAT_NONE {
            unsafe { mpv_sys::mpv_free_node_contents(&raw mut self.inner.inner) };
        }
    }
}

impl Deref for MpvOwnedNode {
    type Target = MpvNode;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Valuable for MpvOwnedNode {
    fn as_value(&self) -> valuable::Value<'_> {
        self.inner.as_value()
    }

    fn visit(&self, visit: &mut dyn valuable::Visit) {
        self.inner.visit(visit);
    }
}

impl Debug for MpvOwnedNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

unsafe impl FromFormat for MpvOwnedNode {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_NODE;

    type MpvType = mpv_node;

    fn finanlize(receiver: Self::MpvType) -> Self {
        Self {
            inner: MpvNode { inner: receiver },
        }
    }
}

unsafe impl ToFormat for &MpvNodeList<'_> {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_NODE_ARRAY;
    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T {
        f((&raw const self.inner).cast())
    }
}

unsafe impl ToFormat for &MpvNodeMap<'_> {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_NODE_MAP;
    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T {
        f((&raw const self.inner).cast())
    }
}
unsafe impl ToFormat for &MpvByteArray<'_> {
    const FORMAT: mpv_format = mpv_format::MPV_FORMAT_BYTE_ARRAY;
    fn do_with<T>(self, f: impl FnOnce(*const c_void) -> T) -> T {
        f((&raw const self.inner).cast())
    }
}
