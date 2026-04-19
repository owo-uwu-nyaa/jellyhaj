# Jellyfin-TUI-rs

This is a (not so little anymore) terminal based Jellyfin client. 
It uses the great [ratatui](https://ratatui.rs/) library to bring you a nice tui and thanks to [ratatui-images](https://github.com/benjajaja/ratatui-image) even with nice preview images (needs terminal support).
All Media is played through the [mpv player](https://mpv.io).

This repository also contains hard forks of the [jellyfin-rs](https://github.com/sargon64/jellyfin-rs) and [libmpv-rs](https://github.com/ParadoxSpiral/libmpv-rs) which I modified heavily.

While I try to gradually implement more and more features from the web client this is a fun hobby project I sometimes sink my time into, therefor development progress is highly erratic (I try to keep the main branch working (at least the current head, don't expect to be able to grab any commit and get something 100% working), and make releases when I think enough features have accumulated). This is also reflected in me sometimes making weird / suboptimal choices because I think doing it that way is more fun. 

# Configuration
You can find some notes in the default config files in 'config'. Just copy it to '~/.config/jellyhaj' and modify it.

# Building
You can just build the project with cargo (requires libmpv, sqlite and clang). The resulting binary should just work (TM).

Alternatively there is a nix flake that builds the binary and provides a basic home manager profile.
There is a binary cache, to use it add the following to your configuration:
```nix
nix.settings = {
    substituters = ["https://owo-uwu-nyaa.cachix.org"];
    trusted-public-keys = [
        "owo-uwu-nyaa.cachix.org-1:cDhuSEgFr9z86WmR2+SE/58/AvM4WXEFYT/nAUBjcok="
    ];
};
```

While I don't have intentionally introduced incompatibilities I only test manually on my own Linux device, therefore it might not work out of the box and needs fixes. Help welcome!

# Contributing

Feel free to contribute to this artisanal, hand crafted piece of pasta! But please keep in mid that I will probably not be particularly responsive (I don't tend to look at this account regularly and only work on this in some of my free time).





