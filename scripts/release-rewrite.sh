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
  awk -v root="path+file://$1#" '
    NR > 1 { bad = 1; exit }
    {
      text = $0
      if (substr(text, 1, 13) != "{\"packages\":[") { bad = 1; exit }
      size = length(text)
      depth = 2
      for (i = 14; i <= size; i++) {
        c = substr(text, i, 1)
        if (quoted) {
          if (escaped) escaped = 0
          else if (c == "\\") escaped = 1
          else if (c == "\"") quoted = 0
          continue
        }
        if (c == "\"") { quoted = 1; continue }
        if (c == "]" || c == "}") {
          depth--
          if (depth == 1) break
          continue
        }
        if (c != "[" && c != "{") continue
        depth++
        if (depth != 3) continue
        if (c != "{" || !match(substr(text, i, 4096), /^\{"name":"[^"\\]+","version":"[^"\\]+","id":"[^"\\]*"/)) { bad = 1; exit }
        split(substr(text, i, RLENGTH), field, "\"")
        if (index(field[12], root) == 1) { found = found field[4] " " field[8]; roots++ }
      }
      if (depth != 1 || roots > 1 || substr(text, i, 23) != "],\"workspace_members\":[") { bad = 1; exit }
      read = 1
    }
    END {
      if (bad || !read) print "unreadable"
      else print found
    }
  '
}
