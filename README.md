# nixdelta

Human-readable diffs between NixOS systems.

NixOS can tell you what derivations changed. nixdelta tells you what
actually changed: which services were added, which ports opened, which users
were created, which packages landed.

## Features

- preview what a rebuild would change before you switch
- review what the last rebuild changed after you switch
- compare any two generations, flake configs, or store paths
- reads directly from the nix store, no evaluation needed
- JSON output for scripting
- usable as a Rust library

## Installation

```console
$ nix run github:adeci/nixdelta
```

Or add to your flake inputs and include in `environment.systemPackages`.

## Usage

### Preview a rebuild

Build first, then preview what would change:

```console
$ nixos-rebuild build
$ nixdelta preview
  praxis (26.05.20260303) → praxis (26.05.20260307) (pending)  (12 changes across 4 sections)

  systemd services

    + nginx             Nginx Web Server
    - cups

  users

    + nginx  system, uid=60, group=nginx

  firewall

    + tcp/443
    + tcp/80
    - tcp/631

  environment packages

    + nginx-1.27.4
    - cups-2.4.12
```

Or skip the `./result` step and pass a flake ref:

```console
$ nixdelta preview .#nixosConfigurations.myhost
```

Or point at any store path:

```console
$ nixdelta preview /nix/store/...-nixos-system-myhost-26.05
```

### Review the last rebuild

```console
$ nixdelta report
  gen 219 → gen 220 (current)  (2 changes across 2 sections)

  systemd services

    + iio-sensor-proxy  IIO Sensor Proxy service

  environment packages

    + iio-sensor-proxy-3.8
```

### Compare generations

```console
$ nixdelta generations 215 220
  gen 215 → gen 220  (3 changes across 2 sections)

  systemd services

    ~ disable-usb-wakeup  "Disable unused USB/TB controller wakeup" → "Disable XHC0 USB controller wakeup"
    - suspend-wwan

  environment packages

    + modem-manager-gui-0.0.20
```

Omit the second generation to compare against current:

```console
$ nixdelta generations 215
```

### Compare flake configs

```console
$ nixdelta diff .#nixosConfigurations.praxis .#nixosConfigurations.leviathan
  praxis (26.05.20260303) → leviathan (26.05.20260303)  (159 changes across 8 sections)

  systemd services

    + buildbot-master    Buildbot Continuous Integration Server.
    + buildbot-worker    Buildbot Worker.
    + harmonia-daemon    Harmonia Nix daemon protocol server
    + nginx              Nginx Web Server                         - bluetooth
    + postgresql         PostgreSQL Server                        - cups
    + postgresql-setup   PostgreSQL Setup Scripts                 - greetd
                                                                  - libvirtd
                                                                  - pipewire

  users

    + buildbot         system, group=buildbot                     - avahi
    + buildbot-worker  system, group=buildbot-worker              - cups
    + nginx            system, uid=60, group=nginx                - geoclue
    + postgres         service, uid=71, group=postgres            - rtkit

  firewall

    + tcp/443
    + tcp/80
    - tcp/631

  nginx vhosts

    + buildbot.decio.us                                           - localhost

  environment packages

    + envfs-1.1.0                                                 - blender-5.0.1
    + postgresql-and-plugins-17.8                                 - obs-studio-32.0.4
                                                                  - steam-1.0.0.85
                                                                  - vesktop-1.6.5

  postgresql

    ~ postgresql  disabled → enabled
    + database: buildbot
    + user: buildbot
```

### JSON output

All commands support `--json`:

```console
$ nixdelta generations 215 220 --json
{
  "before": "gen 215",
  "after": "gen 220",
  "total_changes": 3,
  "sections": [
    {
      "name": "systemd services",
      "changes": [
        {
          "kind": "modified",
          "name": "disable-usb-wakeup",
          "detail": "\"Disable unused USB/TB controller wakeup\" → \"Disable XHC0 USB controller wakeup\""
        },
        { "kind": "removed", "name": "suspend-wwan" }
      ]
    },
    {
      "name": "environment packages",
      "changes": [
        { "kind": "added", "name": "modem-manager-gui-0.0.20" }
      ]
    }
  ]
}
```

## How it reads the store

For generation comparisons, previews, and store paths, nixdelta reads the
same artifacts NixOS uses during activation. No evaluation needed. The `diff`
command and `preview` with a flake ref will evaluate/build as needed.

<details>
<summary><b>Systemd units</b></summary>

Read from `/run/current-system/etc/systemd/system/`. Each `.service` file is a
standard systemd unit in the nix store:

```ini
[Unit]
Description=Bluetooth service
Documentation=man:bluetoothd(8)

[Service]
Type=dbus
BusName=org.bluez
ExecStart=/nix/store/ypxd...-bluez-5.84/libexec/bluetooth/bluetoothd

[Install]
WantedBy=bluetooth.target
```

nixdelta extracts the description, `WantedBy`, and `After` from each unit.

</details>

<details>
<summary><b>Users and groups</b></summary>

NixOS stores the full user/group spec as `users-groups.json` in the store,
referenced from the `activate` script. This is the same file NixOS reads
during activation to create users:

```json
{
  "users": [
    {
      "name": "alex",
      "uid": 3801,
      "group": "users",
      "isSystemUser": false,
      "shell": "/run/current-system/sw/bin/fish"
    },
    {
      "name": "avahi",
      "uid": null,
      "group": "avahi",
      "isSystemUser": true
    }
  ],
  "groups": [
    { "name": "audio", "gid": 17, "members": ["alex", "dima"] }
  ]
}
```

</details>

<details>
<summary><b>Firewall ports</b></summary>

NixOS generates a `firewall-start` script in the store. nixdelta parses the
iptables rules directly:

```bash
ip46tables -A nixos-fw -p tcp --dport 22 -j nixos-fw-accept
ip46tables -A nixos-fw -p udp --dport 5353 -j nixos-fw-accept
```

From these lines nixdelta extracts `tcp/22` and `udp/5353`.

</details>

<details>
<summary><b>Packages</b></summary>

Runs `nix-store --query --references` on `/run/current-system/sw` to get
direct package references, what you declared, not the transitive closure:

```
/nix/store/kg0w...-avahi-0.8
/nix/store/ypxd...-bluez-5.84
/nix/store/fcqh...-pipewire-1.4.10
```

</details>

<details>
<summary><b>Etc files</b></summary>

Walks `/run/current-system/etc/` and collects file paths, skipping NixOS
metadata (`.uid`, `.gid`, `.mode` files).

</details>

<details>
<summary><b>Nginx vhosts</b></summary>

Parses `server_name` directives from the store-generated
`/run/current-system/etc/nginx/nginx.conf`.

</details>

Generation comparison uses the same approach on
`/nix/var/nix/profiles/system-N-link`. Preview reads `./result` or any store
path. The `diff` command is the only one that evaluates nix (via `extract.nix`).

## Credits

Built with help from Claude. Made just in time for the
[Numtide Planet Nix Hackathon 2026](https://github.com/numtide/planetnix-hackathon).

## License

MIT
