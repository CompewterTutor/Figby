#!/bin/sh
# make-examples - Generate a single Markdown file with FIGlet examples.
#
# Usage:
#   ./scripts/make-examples.sh
#   ./scripts/make-examples.sh --sample-text="FIGBY!"
#   ./scripts/make-examples.sh --fonts=standard,big
#   ./scripts/make-examples.sh --exclude=banner,block
#
# Options:
#   --sample-text <text>   Text to render (default: "hello figby")
#   --fonts <list>         Comma-separated font whitelist (stems only)
#   --exclude <list>       Comma-separated font blacklist (stems only)

set -e

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel 2>/dev/null)"

if [ -z "$REPO_ROOT" ]; then
  echo "Error: must be run from within the Figby repository." >&2
  exit 1
fi

cd "$REPO_ROOT"

SAMPLE_TEXT="hello figby"
FONTS_WHITELIST=""
FONTS_BLACKLIST=""

while [ $# -gt 0 ]; do
  case "$1" in
    --sample-text=*)
      SAMPLE_TEXT="${1#*=}"
      ;;
    --fonts=*)
      FONTS_WHITELIST="${1#*=}"
      ;;
    --exclude=*)
      FONTS_BLACKLIST="${1#*=}"
      ;;
    *)
      echo "Error: unknown option: $1" >&2
      echo "Usage: $0 [--sample-text=<text>] [--fonts=<list>] [--exclude=<list>]" >&2
      exit 1
      ;;
  esac
  shift
done

FIGBY=""
if command -v figby >/dev/null 2>&1; then
  FIGBY="figby"
elif [ -x "figby-rs/target/debug/figby" ]; then
  FIGBY="figby-rs/target/debug/figby"
elif [ -x "target/debug/figby" ]; then
  FIGBY="target/debug/figby"
else
  echo "Figby binary not found. Building..." >&2
  cargo build --manifest-path figby-rs/Cargo.toml -p figby 2>&1
  if [ -x "figby-rs/target/debug/figby" ]; then
    FIGBY="figby-rs/target/debug/figby"
  else
    echo "Error: failed to build figby binary." >&2
    exit 1
  fi
fi

OUTFILE="FIGLET_EXAMPLES.md"
OUTDIR="examples"
mkdir -p "$OUTDIR"

{
  echo "# FIGlet Font Examples"
  echo ""
  echo "Generated with Figby. Sample text: \`$SAMPLE_TEXT\`"
  echo ""
} > "$OUTDIR/$OUTFILE"

find fonts/ -maxdepth 1 -type f \( -name '*.flf' -o -name '*.tlf' \) | sort > /tmp/figby-fonts-$$.txt
trap 'rm -f /tmp/figby-fonts-$$.txt' EXIT

count=0
failed=0

while IFS= read -r font_path; do
  font_name="$(basename "$font_path")"
  font_stem="${font_name%.*}"

  if [ -n "$FONTS_WHITELIST" ]; then
    case ",$FONTS_WHITELIST," in
      *,"$font_stem",*) ;;
      *) continue ;;
    esac
  fi

  if [ -n "$FONTS_BLACKLIST" ]; then
    case ",$FONTS_BLACKLIST," in
      *,"$font_stem",*) continue ;;
    esac
  fi

  height="$(head -1 "$font_path" | cut -d' ' -f3)"
  [ -z "$height" ] && height="?"

  output="$("$FIGBY" -d fonts/ -f "$font_stem" "$SAMPLE_TEXT" 2>/dev/null)" || {
    echo "Warning: figby failed for font '$font_name'" >&2
    failed=$((failed + 1))
    continue
  }

  {
    echo ""
    echo "### $font_stem ($font_name, height=$height)"
    echo ""
    echo '```'
    echo "$output"
    echo '```'
    echo ""
  } >> "$OUTDIR/$OUTFILE"

  count=$((count + 1))
done < /tmp/figby-fonts-$$.txt

echo "Generated '$OUTDIR/$OUTFILE' with $count font examples."
if [ "$failed" -gt 0 ]; then
  echo "$font(s) produced errors." >&2
fi
