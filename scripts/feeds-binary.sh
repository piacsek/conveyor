#!/usr/bin/env bash
set -euo pipefail
outside_the_binary='^(tests|docs|plans|scripts|changes|\.github)/|(^|/)AGENTS\.md$|^(PLAN|README)\.md$|^(LICENSE|\.gitignore)$'
grep -vE "$outside_the_binary|^\$" || true
