# Home Manager Module
usage: 
```nix

```

## Options

### enable
enable module
```nix
programs.jellyhaj.enable = true;
```

### package
JellyHaj derivation
Should have `checkKeybinds` and `checkEffects` helpers.
```nix
programs.jellyhaj.package = <derivation>;
```
By default this directly evaluates the expression with `callPackage`.

### mpv_profile
One of `"fast"`, `"default"` and `"high-quality"`
```nix
programs.jellyhaj.config.mpv_profile = "default";
```

### hwdec
Mpv hardware decoder
```nix
programs.jellyhaj.config.hwdec = "auto-safe";
```

### mpv_log_level
Log level for mpv
```nix
programs.jellyhaj.config.mpv_log_level = "info";
```

### login_file
Path to the login file
```nix
programs.jellyhaj.config.login_file = "${config.xdg.configHome}/jellyhaj/login.toml";
```
set by `programs.jellyhaj.login`

### keybinds_file
Path to keybinds file or null
```nix
programs.jellyhaj.config.keybinds_file = null;
```
`programs.jellyhaj.keybinds` sets this option.

### effects_file
Path to effects file or null
```nix
programs.jellyhaj.config.effects_file = null;
```
`programs.jellyhaj.effects` sets this option.

### mpv_config_file
Path to mpv config file sourced by the integrated player or null
```nix
programs.jellyhaj.config.mpv_config_file = null;
```

### entry_image_width
With of entry images in cells. Image height and entry size is scaled accordingly.
```nix
programs.jellyhaj.config.entry_image_width = 32;
```

### concurrent_jellyfin_connections
Concurrent connections to the jellyfin server
```nix
programs.jellyhaj.config.concurrent_jellyfin_connections = 2;
```

### fetch_timeout
Timeout of fetch operations in seconds
```nix
programs.jellyhaj.config.fetch_timeout = 15;
```

### store_access_token
Store the jellyfin access token in the cache. Disabled by default.
```nix
programs.jellyhaj.config.store_access_token = false;
```

### keybinds
[Keybinds definition](./config/keybinds.toml) or `null`
```nix
programs.jellyhaj.keybinds = null;
```

### effects
[Effects definition](./config/effects.toml) or `null`
```nix
programs.jellyhaj.effects = null;
```

### login
`{}` or `null`
```nix
programs.jellyhaj.login = null;
```

#### server_url
url of the Jellyfin server
```nix
programs.jellyhaj.login.server_url = "https://jellyfin.example";
```

#### username
username
```nix
programs.jellyhaj.login.username = "";
```

#### password
*If you set your password here, it will be in your nix config and world readable in the nix store*
```nix
programs.jellyhaj.login.password = "";
```

#### password_cmd
Command that supplies the password on stdout or `null`
```nix
programs.jellyhaj.login.password_cmd = ["cat", "secret_file"];
```

