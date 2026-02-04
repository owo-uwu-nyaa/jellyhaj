use crate::FormItem;

pub const fn add_form_item<T,F: FormItem<T>>(height: u16) -> u16 {
    height.strict_add(F::HEIGHT).strict_add(1)
}

pub const fn add_form_item_buf<T,F: FormItem<T>>(height_buf: u16) -> u16 {
    height_buf
        .saturating_sub(1)
        .saturating_sub(F::HEIGHT)
        .strict_add(F::HEIGHT_BUF)
}

pub const fn add(v1: u16, v2: u16) -> u16 {
    v1.strict_add(v2)
}

pub const fn sub(v1: u16, v2: u16) -> u16 {
    v1.strict_sub(v2)
}
