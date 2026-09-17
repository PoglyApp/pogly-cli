#!/usr/bin/env bash

set -euo pipefail

repo="PoglyApp/pogly-cli"
root="${XDG_DATA_HOME:-$HOME/.local/share}/Pogly/cli"

echo "Installing pogly-cli..."

release=$(curl -fsSL -H "User-Agent: pogly-cli-installer" "https://api.github.com/repos/$repo/releases/latest")
tag=$(echo "$release" | grep -m1 '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
version="${tag#v}"
bin_dir="$root/bin/$version"

mkdir -p "$bin_dir"

echo "Downloading pogly-cli $tag..."
curl -fsSL -o "$bin_dir/pogly-cli" "https://github.com/$repo/releases/download/$tag/pogly-cli"
chmod +x "$bin_dir/pogly-cli"

launcher="$root/pogly"
echo "Downloading launcher..."
if ! curl -fsSL -o "$launcher" "https://github.com/$repo/releases/download/$tag/pogly"; then
    mv -f "$launcher" "$launcher.old" 2>/dev/null || true
    curl -fsSL -o "$launcher" "https://github.com/$repo/releases/download/$tag/pogly"
fi
chmod +x "$launcher"

printf '%s' "$version" > "$root/version"

case "$(basename "${SHELL:-bash}")" in
    zsh)
        shell_rc="$HOME/.zshrc"
        ;;
    bash)
        shell_rc="$HOME/.bashrc"
        ;;
    *)
        shell_rc="$HOME/.profile"
        ;;
esac

if ! grep -qs "$root" "$shell_rc" 2>/dev/null; then
    echo "export PATH=\"$root:\$PATH\"" >> "$shell_rc"
    echo "Added $root to your PATH in $shell_rc (open a new terminal if 'pogly' is not found)."
fi

echo ""
echo "pogly-cli $tag installed."
echo "Get started:"
echo "  pogly overlay add <overlay-url-or-identity> --token pgly_..."
echo "  pogly help"

