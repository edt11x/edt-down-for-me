# edt-down-for-me

A compact Linux desktop app (Rust + Slint) that shows whether configured web
properties are reachable.

## Status dots

| Color | Meaning |
| --- | --- |
| Grey | Starting up; no successful reply yet |
| Green | Replied at least once in the last **12 seconds** |
| Yellow | No reply in the last 12 seconds, but a reply within the last **36–60 seconds** |
| Red | No reply in at least **60 seconds** |

## How reachability is checked

Each listed host is probed about every **6 seconds** (a low rate):

1. **ICMP ping** (unprivileged datagram sockets on modern Linux)
2. If ping is blocked or fails: **TCP connect** to port 443, then 80 (or a custom port)
3. If that still fails: a short **HTTP HEAD** on port 80

Any ICMP echo, TCP accept, or HTTP response (including 4xx) counts as reachable.

Hostnames and IPv4/IPv6 addresses are both allowed. You can paste a URL; the
scheme and path are stripped.

## Default list

- google.com
- github.com
- gitlab.com
- microsoft.com
- okta.com

Add and remove entries in the UI. The list is saved to
`~/.config/edt-down-for-me/sites.json`.

## Build and run

```bash
./build.sh
./run.sh
```

`run.sh` builds a release binary if one is not already present.

```bash
edt-down-for-me --version
edt-down-for-me --help
```

### Build-time packages

Fedora:

```bash
sudo dnf install gcc pkg-config fontconfig-devel libxkbcommon-devel
```

Debian / Ubuntu:

```bash
sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
```

Rust 1.74+ with Cargo is required (Slint 1.17 wants a recent stable compiler).

## Install

User-local (no root), current Linux account:

```bash
./local_install.sh
```

Fedora / system-wide (`/usr/local` by default):

```bash
sudo ./install.sh
```

Override the prefix with `PREFIX=/usr sudo ./install.sh`.

### Debian / Ubuntu `.deb`

```bash
./package_deb.sh
sudo apt install ./dist/edt-down-for-me_0.1.0_amd64.deb
```

The exact filename follows `edt-down-for-me_<version>_<arch>.deb`.
