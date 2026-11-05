#!/usr/bin/env bash
set -euo pipefail

systemctl --user disable --now sitbreak.service || true
rm -f "${HOME}/.config/systemd/user/sitbreak.service"
rm -f "${HOME}/.local/bin/sitbreak"
systemctl --user daemon-reload
case "${LC_ALL:-${LC_MESSAGES:-${LANG:-}}}" in
  zh*|ZH*)
    echo "已卸载。配置仍在 ~/.config/sitbreak ，计时状态仍在 ~/.local/state/sitbreak"
    ;;
  *)
    echo "Uninstalled. Config remains in ~/.config/sitbreak and timer state in ~/.local/state/sitbreak"
    ;;
esac
