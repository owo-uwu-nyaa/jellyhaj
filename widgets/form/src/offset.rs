pub fn calc_offset(len: u16, window: u16, cur: u16) -> u16 {
    let split = window / 2;
    if cur < split {
        0
    } else if cur >= (len.strict_sub(split)) {
        len.strict_sub(window)
    } else {
        cur.strict_sub(split)
    }
}

#[cfg(test)]
mod offset_tests {
    use crate::macro_impl::offset::calc_offset;

    fn check(len: u16, window: u16, cur: u16, offset: u16) {
        assert!(offset + window <= len, "window is out of bounds");
        assert!(
            (offset..offset + window).contains(&cur),
            "current is not in window"
        );
    }

    fn check_for(len: u16, window: u16) {
        for cur in 0..len {
            check(len, window, cur, calc_offset(len, window, cur));
        }
    }

    #[test]
    fn offsets() {
        check_for(1, 1);
        check_for(4, 4);
        check_for(4, 2);
        check_for(5, 5);
        check_for(5, 3);
        check_for(5, 2);
        check_for(5, 1);
    }
}
