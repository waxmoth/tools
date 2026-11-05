#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
cd "$root"

cargo build --release

bin_dir="${HOME}/.local/bin"
unit_dir="${HOME}/.config/systemd/user"
mkdir -p "$bin_dir" "$unit_dir"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
install -m 755 "$target_dir/release/sitbreak" "$bin_dir/sitbreak"
sed "s|@BIN@|$bin_dir/sitbreak|g" "$root/systemd/sitbreak.service" > "$unit_dir/sitbreak.service"

systemctl --user daemon-reload
systemctl --user enable --now sitbreak.service
systemctl --user restart sitbreak.service
systemctl --user --no-pager --full status sitbreak.service
