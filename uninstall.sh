#!/usr/bin/env bash

set -euo pipefail

purge_config=false
[ "${1:-}" = "--purge-config" ] && purge_config=true

root="${XDG_DATA_HOME:-$HOME/.local/share}/Pogly/cli"
config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/Pogly/cli"

if [ -d "$root" ]; then
    rm -rf "$root"
    echo "Removed $root"
else
    echo "Nothing installed at $root"
fi

for shell_rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
    [ -f "$shell_rc" ] || continue
    if grep -qs "$root" "$shell_rc"; then
        sed -i.bak "\#export PATH=\"$root:\$PATH\"#d" "$shell_rc"
        rm -f "$shell_rc.bak"
        echo "Removed $root from PATH in $shell_rc"
    fi
done

if $purge_config; then
    if [ -d "$config_dir" ]; then
        rm -rf "$config_dir"
        echo "Removed overlay profiles at $config_dir"
    fi
elif [ -d "$config_dir" ]; then
    echo "Overlay profiles kept at $config_dir (rerun with --purge-config to remove them)"
fi

echo "pogly-cli uninstalled."