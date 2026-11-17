manifest_with_version() {
  awk -v version="$1" '
    /^[ \t]*\[/ { package = ($0 == "[package]") }
    package && /^version = / { $0 = "version = \"" version "\"" }
    { print }
  ' "$2"
}

lock_with_version() {
  awk -v version="$1" '
    entry && /^version = / { $0 = "version = \"" version "\"" }
    { entry = ($0 == "name = \"conveyor\""); print }
  ' "$2"
}

manifest_reads() {
  printf '%s\n' "$2" | awk -v line="version = \"$1\"" '
    /^[ \t]*\[/ { package = ($0 == "[package]") }
    package && $0 == line { found = 1 }
    END { exit !found }
  '
}

lock_reads() {
  printf '%s\n' "$2" | awk -v line="version = \"$1\"" '
    entry && $0 == line { found = 1 }
    { entry = ($0 == "name = \"conveyor\"") }
    END { exit !found }
  '
}

root_package() {
  jq -rs --arg manifest "$1/Cargo.toml" '
    try (
      if length != 1 or (.[0].packages | type) != "array" then "unreadable"
      else [.[0].packages[] | select(.manifest_path == $manifest)]
        | if length == 0 then ""
          elif length == 1 and (.[0].name | type) == "string" and (.[0].version | type) == "string" then "\(.[0].name) \(.[0].version)"
          else "unreadable" end
      end
    ) catch "unreadable"
  ' 2>/dev/null || echo unreadable
}
