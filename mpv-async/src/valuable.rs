use std::{
    ffi::{CStr, c_char},
    slice,
};

use mpv_sys::{mpv_byte_array, mpv_format, mpv_node_list};
use valuable::{Listable, Mappable, Slice, Valuable, Value, Visit};

use crate::nodes::{MpvNode, MpvString};

#[repr(transparent)]
struct ByteArrayValuable {
    inner: mpv_byte_array,
}

impl Listable for ByteArrayValuable {
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.inner.size, Some(self.inner.size))
    }
}
impl Valuable for ByteArrayValuable {
    fn as_value(&self) -> Value<'_> {
        Value::Listable(self)
    }

    fn visit(&self, visit: &mut dyn valuable::Visit) {
        visit.visit_primitive_slice(Slice::U8(unsafe {
            slice::from_raw_parts(self.inner.data.cast(), self.inner.size)
        }));
    }
}

#[repr(transparent)]
struct ArrayValuable {
    inner: mpv_node_list,
}

impl Listable for ArrayValuable {
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size: usize = self.inner.num.try_into().expect("array size negative?");
        (size, Some(size))
    }
}

impl Valuable for ArrayValuable {
    fn as_value(&self) -> Value<'_> {
        Value::Listable(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        unsafe { visit_array(&self.inner, visit) };
    }
}

unsafe fn visit_array(this: &mpv_node_list, visit: &mut dyn Visit) {
    let size: usize = this.num.try_into().expect("array size negative?");
    let slice = unsafe { slice::from_raw_parts(this.values.cast::<MpvNode>(), size) };
    for v in slice {
        visit.visit_value(v.as_value());
    }
}

#[repr(transparent)]
struct MapValuable {
    inner: mpv_node_list,
}

impl Mappable for MapValuable {
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size: usize = self.inner.num.try_into().expect("array size negative?");
        (size, Some(size))
    }
}

impl Valuable for MapValuable {
    fn as_value(&self) -> Value<'_> {
        Value::Mappable(self)
    }

    fn visit(&self, visit: &mut dyn valuable::Visit) {
        unsafe { visit_map(&self.inner, visit) };
    }
}

unsafe fn visit_map(this: &mpv_node_list, visit: &mut dyn Visit) {
    let size: isize = this.num.try_into().expect("how even");
    for (key, val) in (0..size).map(|i| unsafe {
        let key = cstr_val(this.keys.offset(i).read());
        let val = this.values.offset(i).cast::<MpvNode>().as_ref_unchecked();
        (key, val)
    }) {
        visit.visit_entry(key, val.as_value());
    }
}

unsafe fn cstr_val<'s>(v: *const c_char) -> Value<'s> {
    Value::String(
        unsafe { CStr::from_ptr(v) }
            .to_str()
            .unwrap_or("invalid utf8"),
    )
}

impl Valuable for MpvNode {
    fn as_value(&self) -> Value<'_> {
        match self.inner.format {
            mpv_format::MPV_FORMAT_NONE => Value::Unit,
            mpv_format::MPV_FORMAT_STRING => unsafe { cstr_val(self.inner.u.string) },
            mpv_format::MPV_FORMAT_FLAG => match unsafe { self.inner.u.flag } {
                0 => Value::Bool(false),
                1 => Value::Bool(true),
                _ => Value::String("invalid flag value"),
            },
            mpv_format::MPV_FORMAT_INT64 => Value::I64(unsafe { self.inner.u.int64 }),
            mpv_format::MPV_FORMAT_DOUBLE => Value::F64(unsafe { self.inner.u.double_ }),
            mpv_format::MPV_FORMAT_BYTE_ARRAY => Value::Listable(unsafe {
                self.inner
                    .u
                    .ba
                    .cast::<ByteArrayValuable>()
                    .as_ref_unchecked()
            }),
            mpv_format::MPV_FORMAT_NODE_ARRAY => Value::Listable(unsafe {
                self.inner.u.list.cast::<ArrayValuable>().as_ref_unchecked()
            }),
            mpv_format::MPV_FORMAT_NODE_MAP => Value::Mappable(unsafe {
                self.inner.u.list.cast::<MapValuable>().as_ref_unchecked()
            }),
            _ => Value::String("unknown node type"),
        }
    }

    fn visit(&self, visit: &mut dyn valuable::Visit) {
        match self.inner.format {
            mpv_format::MPV_FORMAT_BYTE_ARRAY => visit.visit_primitive_slice(Slice::U8(unsafe {
                let ba = self.inner.u.ba.as_ref_unchecked();
                slice::from_raw_parts(ba.data.cast(), ba.size)
            })),
            mpv_format::MPV_FORMAT_NODE_ARRAY => {
                unsafe { visit_array(self.inner.u.list.as_ref_unchecked(), visit) };
            }
            mpv_format::MPV_FORMAT_NODE_MAP => {
                unsafe { visit_map(self.inner.u.list.as_ref_unchecked(), visit) };
            }

            _ => {}
        }
    }
}

impl Valuable for MpvString {
    fn as_value(&self) -> Value<'_> {
        unsafe { cstr_val(self.as_ptr()) }
    }
    fn visit(&self, _visit: &mut dyn Visit) {}
}
