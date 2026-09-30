use std::{
    ffi::{CStr, c_char},
    slice,
};

use mpv_sys::{mpv_byte_array, mpv_format, mpv_node_list};
use valuable::{
    EnumDef, Enumerable, Fields, Listable, Mappable, NamedField, NamedValues, Slice, StructDef,
    Structable, Valuable, Value, Variant, VariantDef, Visit,
};

use crate::{
    MpvError,
    events::{ClientMessage, MpvEvent, MpvProperty},
    nodes::{MpvNode, MpvNodeRef, MpvString},
};

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
    if size == 0 {
        return;
    }
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
    cstr_value(unsafe { CStr::from_ptr(v) })
}

impl Valuable for MpvError {
    fn as_value(&self) -> Value<'_> {
        cstr_value(self.get_error_string())
    }

    fn visit(&self, _visit: &mut dyn Visit) {}
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

impl Valuable for MpvNodeRef<'_> {
    fn as_value(&self) -> Value<'_> {
        match self {
            MpvNodeRef::String(cstr) => cstr_value(cstr),
            MpvNodeRef::Bool(v) => Value::Bool(*v),
            MpvNodeRef::Int(v) => Value::I64(*v),
            MpvNodeRef::Float(v) => Value::F64(*v),
            MpvNodeRef::Map(_) => Value::Mappable(self),
            MpvNodeRef::Array(_) | MpvNodeRef::Bytes(_) => Value::Listable(self),
            MpvNodeRef::None => Value::Unit,
        }
    }

    fn visit(&self, visit: &mut dyn Visit) {
        match self {
            MpvNodeRef::Array(mpv_nodes) => mpv_nodes.visit(visit),

            MpvNodeRef::Map(mpv_node_map_ref) => {
                mpv_node_map_ref.iter().for_each(|(key, val)| {
                    visit.visit_entry(cstr_value(key), val.as_value());
                });
            }
            MpvNodeRef::Bytes(items) => visit.visit_primitive_slice(Slice::U8(items)),
            _ => {}
        }
    }
}

impl Listable for MpvNodeRef<'_> {
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = match self {
            Self::Array(nodes) => nodes.len(),
            Self::Bytes(val) => val.len(),
            _ => unimplemented!(),
        };
        (len, Some(len))
    }
}

impl Mappable for MpvNodeRef<'_> {
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = match self {
            Self::Map(map) => map.len(),
            _ => unimplemented!(),
        };
        (len, Some(len))
    }
}

impl Valuable for MpvString {
    fn as_value(&self) -> Value<'_> {
        unsafe { cstr_val(self.as_ptr()) }
    }
    fn visit(&self, _visit: &mut dyn Visit) {}
}

fn cstr_value(val: &CStr) -> Value<'_> {
    Value::String(val.to_str().unwrap_or("invalid utf-8"))
}

const _: () = {
    static FILEDS: &[NamedField] = &[NamedField::new("name"), NamedField::new("value")];

    impl Valuable for MpvProperty {
        fn as_value(&self) -> Value<'_> {
            Value::Structable(self)
        }

        fn visit(&self, visit: &mut dyn Visit) {
            visit.visit_named_fields(&NamedValues::new(
                FILEDS,
                &[cstr_value(self.name()), self.differentiate().as_value()],
            ));
        }
    }

    impl Structable for MpvProperty {
        fn definition(&self) -> StructDef<'_> {
            StructDef::new_static("MpvProperty", Fields::Named(FILEDS))
        }
    }
};

impl Valuable for ClientMessage<'_> {
    fn as_value(&self) -> Value<'_> {
        Value::Listable(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        self.into_iter()
            .map(cstr_value)
            .for_each(|v| visit.visit_value(v));
    }
}

impl Listable for ClientMessage<'_> {
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

const _: () = {
    static MPV_EVENT_LOG_MESSAGE_FIELDS: &[NamedField<'static>] = &[
        NamedField::new("prefix"),
        NamedField::new("level"),
        NamedField::new("text"),
    ];
    static MPV_EVENT_GET_PROPERTY_REPLY_FIELDS: &[NamedField<'static>] =
        &[NamedField::new("userdata"), NamedField::new("data")];
    static MPV_EVENT_SET_PROPERTY_REPLY_FIELDS: &[NamedField<'static>] =
        &[NamedField::new("userdata")];
    static MPV_EVENT_COMMAND_REPLY_FIELDS: &[NamedField<'static>] =
        &[NamedField::new("userdata"), NamedField::new("data")];
    static MPV_EVENT_START_FILE_FIELDS: &[NamedField<'static>] =
        &[NamedField::new("playlist_entry_id")];
    static MPV_EVENT_END_FILE_FIELDS: &[NamedField<'static>] =
        &[NamedField::new("entry_id"), NamedField::new("reason")];
    static MPV_EVENT_PROPERTY_CHANGE_FIELDS: &[NamedField<'static>] =
        &[NamedField::new("userdata"), NamedField::new("data")];
    static MPV_EVENT_HOOK_FIELDS: &[NamedField<'static>] =
        &[NamedField::new("name"), NamedField::new("userdata")];
    static MPV_EVENT_VARIANTS: &[VariantDef<'static>] = &[
        VariantDef::new("Shutdown", Fields::Unnamed(0)),
        VariantDef::new("LogMessage", Fields::Named(MPV_EVENT_LOG_MESSAGE_FIELDS)),
        VariantDef::new(
            "GetPropertyReply",
            Fields::Named(MPV_EVENT_GET_PROPERTY_REPLY_FIELDS),
        ),
        VariantDef::new(
            "SetPropertyReply",
            Fields::Named(MPV_EVENT_SET_PROPERTY_REPLY_FIELDS),
        ),
        VariantDef::new(
            "CommandReply",
            Fields::Named(MPV_EVENT_COMMAND_REPLY_FIELDS),
        ),
        VariantDef::new("StartFile", Fields::Named(MPV_EVENT_START_FILE_FIELDS)),
        VariantDef::new("EndFile", Fields::Named(MPV_EVENT_END_FILE_FIELDS)),
        VariantDef::new("FileLoaded", Fields::Unnamed(0)),
        VariantDef::new("ClientMessage", Fields::Unnamed(1usize)),
        VariantDef::new("VideoReconfig", Fields::Unnamed(0)),
        VariantDef::new("AudioReconfig", Fields::Unnamed(0)),
        VariantDef::new("Seek", Fields::Unnamed(0)),
        VariantDef::new("PlaybackRestart", Fields::Unnamed(0)),
        VariantDef::new(
            "PropertyChange",
            Fields::Named(MPV_EVENT_PROPERTY_CHANGE_FIELDS),
        ),
        VariantDef::new("QueueOverflow", Fields::Unnamed(0)),
        VariantDef::new("Hook", Fields::Named(MPV_EVENT_HOOK_FIELDS)),
        VariantDef::new("Unknown", Fields::Unnamed(0)),
    ];

    impl Enumerable for MpvEvent<'_> {
        fn definition(&self) -> EnumDef<'_> {
            EnumDef::new_static("MpvEvent", MPV_EVENT_VARIANTS)
        }
        fn variant(&self) -> Variant<'_> {
            match self {
                Self::Shutdown => Variant::Static(&MPV_EVENT_VARIANTS[0usize]),
                Self::LogMessage { .. } => Variant::Static(&MPV_EVENT_VARIANTS[1usize]),
                Self::GetPropertyReply { .. } => Variant::Static(&MPV_EVENT_VARIANTS[2usize]),
                Self::SetPropertyReply { .. } => Variant::Static(&MPV_EVENT_VARIANTS[3usize]),
                Self::CommandReply { .. } => Variant::Static(&MPV_EVENT_VARIANTS[4usize]),
                Self::StartFile { .. } => Variant::Static(&MPV_EVENT_VARIANTS[5usize]),
                Self::EndFile { .. } => Variant::Static(&MPV_EVENT_VARIANTS[6usize]),
                Self::FileLoaded => Variant::Static(&MPV_EVENT_VARIANTS[7usize]),
                Self::ClientMessage(..) => Variant::Static(&MPV_EVENT_VARIANTS[8usize]),
                Self::VideoReconfig => Variant::Static(&MPV_EVENT_VARIANTS[9usize]),
                Self::AudioReconfig => Variant::Static(&MPV_EVENT_VARIANTS[10usize]),
                Self::Seek => Variant::Static(&MPV_EVENT_VARIANTS[11usize]),
                Self::PlaybackRestart => Variant::Static(&MPV_EVENT_VARIANTS[12usize]),
                Self::PropertyChange { .. } => Variant::Static(&MPV_EVENT_VARIANTS[13usize]),
                Self::QueueOverflow => Variant::Static(&MPV_EVENT_VARIANTS[14usize]),
                Self::Hook { .. } => Variant::Static(&MPV_EVENT_VARIANTS[15usize]),
                Self::Unknown => Variant::Static(&MPV_EVENT_VARIANTS[16usize]),
            }
        }
    }
    #[automatically_derived]
    impl Valuable for MpvEvent<'_> {
        fn as_value(&self) -> Value<'_> {
            Value::Enumerable(self)
        }
        fn visit(&self, visitor: &mut dyn Visit) {
            match self {
                Self::LogMessage {
                    prefix,
                    level,
                    level_str,
                    text,
                } => {
                    visitor.visit_named_fields(&NamedValues::new(
                        MPV_EVENT_LOG_MESSAGE_FIELDS,
                        &[cstr_value(prefix), cstr_value(level_str), cstr_value(text)],
                    ));
                }
                Self::GetPropertyReply { userdata, data } => {
                    visitor.visit_named_fields(&NamedValues::new(
                        MPV_EVENT_GET_PROPERTY_REPLY_FIELDS,
                        &[Valuable::as_value(&userdata), Valuable::as_value(&data)],
                    ));
                }
                Self::SetPropertyReply { userdata } => {
                    visitor.visit_named_fields(&NamedValues::new(
                        MPV_EVENT_SET_PROPERTY_REPLY_FIELDS,
                        &[Valuable::as_value(&userdata)],
                    ));
                }
                Self::CommandReply { userdata, data } => {
                    visitor.visit_named_fields(&NamedValues::new(
                        MPV_EVENT_COMMAND_REPLY_FIELDS,
                        &[Valuable::as_value(&userdata), Valuable::as_value(&data)],
                    ));
                }
                Self::StartFile { playlist_entry_id } => {
                    visitor.visit_named_fields(&NamedValues::new(
                        MPV_EVENT_START_FILE_FIELDS,
                        &[Valuable::as_value(&playlist_entry_id)],
                    ));
                }
                Self::EndFile { entry_id, reason } => {
                    visitor.visit_named_fields(&NamedValues::new(
                        MPV_EVENT_END_FILE_FIELDS,
                        &[Valuable::as_value(&entry_id), Valuable::as_value(&reason)],
                    ));
                }
                Self::ClientMessage(client_message) => {
                    visitor.visit_unnamed_fields(&[Valuable::as_value(&client_message)]);
                }
                Self::PropertyChange { userdata, data } => {
                    visitor.visit_named_fields(&NamedValues::new(
                        MPV_EVENT_PROPERTY_CHANGE_FIELDS,
                        &[Valuable::as_value(&userdata), Valuable::as_value(&data)],
                    ));
                }
                Self::Hook {
                    name,
                    handle,
                    userdata,
                } => {
                    visitor.visit_named_fields(&NamedValues::new(
                        MPV_EVENT_HOOK_FIELDS,
                        &[cstr_value(name), Valuable::as_value(&userdata)],
                    ));
                }
                Self::Shutdown
                | Self::FileLoaded
                | Self::VideoReconfig
                | Self::AudioReconfig
                | Self::Seek
                | Self::PlaybackRestart
                | Self::QueueOverflow
                | Self::Unknown => {
                    visitor.visit_unnamed_fields(&[]);
                }
            }
        }
    }
};
