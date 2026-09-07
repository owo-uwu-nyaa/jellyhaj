use std::{env::var, sync::LazyLock};

fn main() {
    link_lib();
    #[cfg(feature = "bindgen")]
    run_bindgen();
}

static LINK_STATIC: LazyLock<bool> = LazyLock::new(|| {
    println!("cargo::rerun-if-env-changed=MPV_STATIC");
    var("MPV_STATIC").is_ok_and(|s| "true" == s)
});

#[cfg(unix)]
static PKG_CONFIG_RES: LazyLock<Option<pkg_config::Library>> = LazyLock::new(|| {
    match pkg_config::Config::new()
        .atleast_version("0.41.0")
        .statik(*LINK_STATIC)
        .probe("mpv")
    {
        Err(e) => {
            println!("cargo::warning={e:?}");
            println!(
                "cargo::error=Unable to find mpv. Try setting MPV_PATH and MPV_INCLUDE_PATH or PKG_CONFIG_PATH."
            );
            None
        }
        Ok(v) => Some(v),
    }
});

fn link_lib() {
    println!("cargo::rerun-if-env-changed=MPV_PATH");
    if let Ok(path) = std::env::var("MPV_PATH") {
        println!("cargo::rustc-link-search=native={path}");
        if *LINK_STATIC {
            println!("cargo::rustc-link-lib=static=mpv");
        } else {
            println!("cargo::rustc-link-lib=dylib=mpv");
        }
        return;
    }
    #[cfg(unix)]
    {
        if PKG_CONFIG_RES.is_none() {
            println!(
                "cargo::error=Unable to find mpv through pkg-config. Try setting MPV_PATH or PKG_CONFIG_PATH."
            );
        } else if *LINK_STATIC {
            println!("cargo::rustc-link-lib=static=mpv");
        } else {
            println!("cargo::rustc-link-lib=dylib=mpv");
        }
    }
    #[cfg(not(unix))]
    {
        println!("cargo::error=Unable to find mpv. Try setting MPV_PATH.");
    }
}

#[cfg(feature = "bindgen")]
fn run_bindgen() {
    use std::env::var_os;

    println!("cargo::rerun-if-env-changed=MPV_INCLUDE_PATH");
    let header_paths = if let Ok(path) = std::env::var("MPV_INCLUDE_PATH") {
        vec![path]
    } else {
        #[cfg(unix)]
        {
            if let Some(res) = PKG_CONFIG_RES.as_ref() {
                res.include_paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect()
            } else {
                println!(
                    "cargo::warning=Unable to find mpv. Try setting MPV_INCLUDE_PATH or PKG_CONFIG_PATH."
                );
                vec![]
            }
        }
        #[cfg(not(unix))]
        {
            println!(
                "cargo::warning=Unable to find mpv include directory. Try setting MPV_INCLUDE_PATH."
            );
            vec![]
        }
    };
    let builder = bindgen::builder()
        .clang_args(
            header_paths
                .iter()
                .inspect(|p| println!("adding header path {p}"))
                .map(|p| format!("-I{p}")),
        )
        .generate_cstr(true)
        .wrap_unsafe_ops(true)
        .rust_edition(bindgen::RustEdition::Edition2024)
        .allowlist_recursively(false);
    let out = std::path::PathBuf::from(var_os("OUT_DIR").expect("no out dir set"));
    let mut client = out.clone();
    client.push("client.rs");
    builder
        .clone()
        .header_contents("base.h", "#include <mpv/client.h>")
        .allowlist_item("mpv.*")
        .generate()
        .expect("generating bindings failed")
        .write_to_file(client)
        .expect("writing bindings");
    let mut render = out.clone();
    render.push("render.rs");
    builder
        .clone()
        .header_contents("base.h", "#include <mpv/render.h>")
        .allowlist_item("mpv_render.*")
        .generate()
        .expect("generating bindings failed")
        .write_to_file(render)
        .expect("writing bindings");
    let mut render_gl = out.clone();
    render_gl.push("render_gl.rs");
    builder
        .clone()
        .header_contents("base.h", "#include <mpv/render_gl.h>")
        .allowlist_item("mpv_opengl.*")
        .allowlist_type("_drmModeAtomicReq")
        .generate()
        .expect("generating bindings failed")
        .write_to_file(render_gl)
        .expect("writing bindings");
    let mut stream_cb = out.clone();
    stream_cb.push("stream_cb.rs");
    builder
        .clone()
        .header_contents("base.h", "#include <mpv/stream_cb.h>")
        .allowlist_item("mpv_stream.*")
        .generate()
        .expect("generating bindings failed")
        .write_to_file(stream_cb)
        .expect("writing bindings");

    println!("cargo::rerun-if-env-changed=MPV_PRINT_OUT_DIR");
    if var("MPV_PRINT_OUT_DIR").is_ok_and(|s| "true" == s) {
        println!("cargo::warning=written output to {}", out.display());
    }
}
