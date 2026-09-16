#!/usr/bin/env sh
set -e

echo "generating bindings"

rust_version="1.97.1"

bindgen --no-layout-tests --rust-target "$rust_version" --generate-cstr \
	--wrap-unsafe-ops --newtype-enum 'mpv.*' \
	--with-derive-ord --with-derive-eq \
	--allowlist-item 'mpv.*' --allowlist-item 'MPV.*' \
	--allowlist-item HEADER_MPV_CLIENT_API_VERSION \
	"client-helper.h" -o src/client.rs -- -DMPV_ENABLE_DEPRECATED=0 -fretain-comments-from-system-headers
bindgen --no-layout-tests --rust-target "$rust_version" --generate-cstr \
	--wrap-unsafe-ops --newtype-enum 'mpv.*' \
	--allowlist-item 'mpv_render.*' --allowlist-item 'MPV_RENDER.*' \
	--allowlist-item 'mpv_opengl.*' --allowlist-item 'MPV_OPENGL.*' \
	--allowlist-item '_drmModeAtomicReq' --no-recursive-allowlist \
	"render-helper.h" -o src/render.rs -- -DMPV_ENABLE_DEPRECATED=0 -fretain-comments-from-system-headers
bindgen --no-layout-tests --rust-target "$rust_version" --generate-cstr \
	--wrap-unsafe-ops --newtype-enum 'mpv.*' \
	--allowlist-item 'mpv_stream.*' --allowlist-item 'MPV_STREAM.*' \
	--no-recursive-allowlist \
	"stream_cb-helper.h" -o src/stream_cb.rs \
	-- -DMPV_ENABLE_DEPRECATED=0 -fretain-comments-from-system-headers
