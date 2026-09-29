# Raw libmpv bindings

[Mpv](https://mpv.io) is a scriptable and embeddable media player.
This provides up to date bindings for embedding libmpv with slightly fixed doc comments.

## Building

This crate currently does not support building mpv itself (it has a large list of dependencies). Therefore you must provide an already existing installation of mpv by setting `MPV_PATH` to the directory containing libmpv.

Libmpv is linked dynamically by default, you can overrride this with `MPV_STATIC=true`.

On unix systems pkg-config is used to locate the library if `MPV_PATH` is unset.

## Features

By defalt this crate uses pregenerated bindings. 
If you need
- Symbols introduced in later versions
- Deprecated symbols

you can use bindgen to generate bindings dynamically. Enable the `bindgen` feature and either set `MPV_INCLUDE_PATH` or use pkg-config to provide the headers.

