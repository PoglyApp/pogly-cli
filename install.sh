#!/usr/bin/env bash

set -euo pipefail

repo="PoglyApp/pogly-cli"
root="${XDG_DATA_HOME:-$HOME/.local/share}/Pogly/cli"

echo "Installing pogly-cli..."

release_url="https://api.github.com/repos/$repo/releases/latest"

release="$(curl -fsSL \
    -H "Accept: application/vnd.github+json" \
    -H "User-Agent: pogly-cli-installer" \
    "$release_url")"

tag="$(printf '%s' "$release" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | cut -d'"' -f4)"

if [[ -z "$tag" ]]; then
    echo "Error: Could not determine the latest release tag."
    echo ""
    echo "GitHub API response:"
    printf '%s\n' "$release"
    exit 1
fi

version="${tag#v}"
bin_dir="$root/bin/$version"

mkdir -p "$bin_dir"

echo "Downloading pogly-cli $tag..."

curl -fL \
    -H "User-Agent: pogly-cli-installer" \
    -o "$bin_dir/pogly-cli" \
    "https://github.com/$repo/releases/download/$tag/pogly-cli"

chmod +x "$bin_dir/pogly-cli"

launcher="$root/pogly"
launcher_tmp="$root/.pogly.tmp.$$"

cleanup() {
    rm -f "$launcher_tmp"
}

trap cleanup EXIT

echo "Downloading launcher..."

curl -fL \
    -H "User-Agent: pogly-cli-installer" \
    -o "$launcher_tmp" \
    "https://github.com/$repo/releases/download/$tag/pogly"

chmod +x "$launcher_tmp"
mv -f "$launcher_tmp" "$launcher"

printf '%s' "$version" > "$root/version"

shell_name="$(basename "${SHELL:-bash}")"

case "$shell_name" in
    fish)
        shell_rc="$HOME/.config/fish/config.fish"

        mkdir -p "$(dirname "$shell_rc")"

        if ! grep -Fqs "$root" "$shell_rc" 2>/dev/null; then
            printf '\nset -gx PATH "%s" $PATH\n' "$root" >> "$shell_rc"
            echo "Added $root to your PATH in $shell_rc."
        fi
        ;;

    zsh)
        shell_rc="$HOME/.zshrc"

        if ! grep -Fqs "$root" "$shell_rc" 2>/dev/null; then
            printf '\nexport PATH="%s:$PATH"\n' "$root" >> "$shell_rc"
            echo "Added $root to your PATH in $shell_rc."
        fi
        ;;

    bash)
        shell_rc="$HOME/.bashrc"

        if ! grep -Fqs "$root" "$shell_rc" 2>/dev/null; then
            printf '\nexport PATH="%s:$PATH"\n' "$root" >> "$shell_rc"
            echo "Added $root to your PATH in $shell_rc."
        fi
        ;;

    *)
        shell_rc="$HOME/.profile"

        if ! grep -Fqs "$root" "$shell_rc" 2>/dev/null; then
            printf '\nexport PATH="%s:$PATH"\n' "$root" >> "$shell_rc"
            echo "Added $root to your PATH in $shell_rc."
        fi
        ;;
esac

echo ""
echo "pogly-cli $tag installed."
echo "Get started:"
echo "  pogly overlay add <overlay-url-or-identity> --token pgly_..."
echo "  pogly help"

