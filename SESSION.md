# Session notes

## Request

Build a compact local Linux GUI (Rust + Slint) that shows whether configured
web properties are accessible. Ping at a low rate, with a fallback if ping
fails. Allow adding and deleting hosts/IPs, persist the list, and ship
`build.sh`, `run.sh`, `local_install.sh`, Fedora `install.sh`, and a `.deb`.

## What was done

- Slint window listing hosts with grey/green/yellow/red dots.
- Probe loop (~6s): ICMP, then TCP 443/80, then HTTP HEAD.
- Config at `~/.config/edt-down-for-me/sites.json` (defaults written on first run).
- Packaging scripts and a `.deb` builder (`package_deb.sh`).

## Decisions

- Version **0.1.0** for the first landing.
- Yellow covers 12s–60s (the 36s stale point sits in that window); red at 60s.
- Any ICMP / TCP accept / HTTP response (including 4xx) counts as reachable.
- Generated `target/` and `dist/` (including `.deb`) stay gitignored; rebuild with `package_deb.sh`.

## Known issues

- Many CDNs block ICMP; TCP/HTTP fallback is what usually turns the dot green.
- `cargo clippy`, `rustfmt`, `cargo audit`, and Snyk were not installed on this host; `cargo test` plus debug/release builds were the substitute.

## Next steps

- Optional Fedora RPM spec if a packaged install is wanted beyond `install.sh`.
