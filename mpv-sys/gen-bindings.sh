#!/usr/bin/env sh
set -e

echo "generating bindings from headers in $1"

rust_version="1.97.1"

bindgen --no-layout-tests --rust-target "$rust_version" --generate-cstr --wrap-unsafe-ops "$1/mpv/client.h" --allowlist-item 'mpv.*' -o src/client.rs -- -DMPV_ENABLE_DEPRECATED=0
bindgen --no-layout-tests --rust-target "$rust_version" --generate-cstr --wrap-unsafe-ops "$1/mpv/render.h" --no-recursive-allowlist --allowlist-item 'mpv_render.*' -o src/render.rs -- -DMPV_ENABLE_DEPRECATED=0
bindgen --no-layout-tests --rust-target "$rust_version" --generate-cstr --wrap-unsafe-ops "$1/mpv/render_gl.h" --no-recursive-allowlist --allowlist-item 'mpv_opengl.*' --allowlist-item "_drmModeAtomicReq" -o src/render_gl.rs -- -DMPV_ENABLE_DEPRECATED=0
bindgen --no-layout-tests --rust-target "$rust_version" --generate-cstr --wrap-unsafe-ops "$1/mpv/stream_cb.h" --no-recursive-allowlist --allowlist-item 'mpv_stream.*' -o src/stream_cb.rs -- -DMPV_ENABLE_DEPRECATED=0
