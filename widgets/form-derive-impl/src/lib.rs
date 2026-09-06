#![allow(clippy::too_many_arguments)]

pub mod form;
pub mod selection;

#[cfg(test)]
pub mod test_helper {

    use prettydiff::diff_lines;
    use prettyplease::unparse;
    use proc_macro2::TokenStream;
    use std::{env, fs, path::Path};
    use syn::{File, parse_file, parse2};

    #[track_caller]
    pub fn assert_tokens_eq(expected: impl AsRef<Path>, actual: TokenStream) {
        let expected = expected.as_ref();
        let actual: File = parse2(actual).expect("parsing actual failed!");
        let actual = unparse(&actual);
        if env::var("FORM_DERIVE_OVERRIDE_TOKENS").is_ok_and(|v| "true" == v) {
            fs::write(expected, actual).expect("failed to write out new tokens");
        } else {
            let expected = fs::read_to_string(expected)
                .expect("failed to open file containing expected tokens");
            let expected: File = parse_file(&expected).expect("parsing expected failed!");
            let expected = unparse(&expected);
            if expected != actual {
                let diff = diff_lines(&expected, &actual);
                panic!("tokens diverged from expected value:\n{diff}")
            }
        }
    }
}
