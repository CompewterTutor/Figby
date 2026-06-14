#!/bin/sh
# regenerate-expected.sh — Regenerate expected test outputs from C FIGlet 2.2.5
#
# Usage: ./scripts/regenerate-expected.sh
#   Builds C figlet from c-figlet/, runs every test scenario, overwrites tests/res*.txt
#   with byte-exact output from C figlet.
#
# Environment:
#   CC — C compiler (default: gcc)
#   FIGLET_BINARY — path to C figlet (default: builds from c-figlet/)
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FONTS_DIR="$REPO_ROOT/fonts"
TESTS_DIR="$REPO_ROOT/tests"
C_FIGLET_DIR="$REPO_ROOT/c-figlet"

# === Build C FIGlet ===
FIGLET_BINARY="${FIGLET_BINARY:-"$C_FIGLET_DIR/figlet"}"
if [ ! -x "$FIGLET_BINARY" ]; then
    echo "Building C figlet from $C_FIGLET_DIR ..."
    CC="${CC:-gcc}"
    # Pass DEFAULTFONTDIR so the binary can resolve fonts without FIGLET_FONTDIR
    make -C "$C_FIGLET_DIR" \
        CC="$CC" \
        XCFLAGS="-DTLF_FONTS -DDEFAULTFONTDIR='\"$FONTS_DIR\"' -DDEFAULTFONTFILE='\"standard\"'" \
        figlet 2>&1 | grep -v "^make:" | grep -v redefined || true
    if [ ! -x "$FIGLET_BINARY" ]; then
        echo "ERROR: Failed to build C figlet" >&2
        exit 1
    fi
fi

# === Helper: run C figlet ===
run_figlet() {
    FIGLET_FONTDIR="$FONTS_DIR" "$FIGLET_BINARY" "$@"
}

# === Generate expected outputs ===

echo "=== Generating expected outputs for tests 1-50 ==="

# --- Test 01: showfigfonts ---
echo "  Test 01: showfigfonts"
{
    for stem in $(ls "$FONTS_DIR"/*.flf | xargs -n1 basename | sed 's/\.flf$//' | sort); do
        echo "$stem :"
        run_figlet -f "$stem" "$stem"
        echo
        echo
    done
} > "$TESTS_DIR/res001.txt"

# --- Test 02: all fonts with default input ---
echo "  Test 02: all fonts"
INPUT=$(cat "$TESTS_DIR/input.txt")
{
    for stem in $(ls "$FONTS_DIR"/*.flf | xargs -n1 basename | sed 's/\.flf$//' | sort); do
        printf "%s" "$INPUT" | run_figlet -f "fonts/$stem"
    done
} > "$TESTS_DIR/res002.txt"

# --- Tests 03-27: existing test scenarios ---
echo "  Test 03: long text"
printf "%s" "$(cat "$TESTS_DIR/longtext.txt")" | run_figlet > "$TESTS_DIR/res003.txt"

echo "  Test 04: left to right"
printf "%s" "$INPUT" | run_figlet -L > "$TESTS_DIR/res004.txt"

echo "  Test 05: right to left"
printf "%s" "$INPUT" | run_figlet -R > "$TESTS_DIR/res005.txt"

echo "  Test 06: flush left"
printf "%s" "$INPUT" | run_figlet -l > "$TESTS_DIR/res006.txt"

echo "  Test 07: flush right"
printf "%s" "$INPUT" | run_figlet -r > "$TESTS_DIR/res007.txt"

echo "  Test 08: center"
printf "%s" "$INPUT" | run_figlet -c > "$TESTS_DIR/res008.txt"

echo "  Test 09: kerning"
printf "%s" "$INPUT" | run_figlet -k > "$TESTS_DIR/res009.txt"

echo "  Test 10: full width"
printf "%s" "$INPUT" | run_figlet -W > "$TESTS_DIR/res010.txt"

echo "  Test 11: overlap"
printf "%s" "$INPUT" | run_figlet -o > "$TESTS_DIR/res011.txt"

echo "  Test 12: TLF font"
printf "%s" "$INPUT" | run_figlet -f tests/emboss > "$TESTS_DIR/res012.txt"

echo "  Test 13: kerning flush left RTL"
printf "%s" "$INPUT" | run_figlet -klR > "$TESTS_DIR/res013.txt"

echo "  Test 14: kerning center RTL slant"
printf "%s" "$INPUT" | run_figlet -kcR -f slant > "$TESTS_DIR/res014.txt"

echo "  Test 15: full width flush right RTL"
printf "%s" "$INPUT" | run_figlet -WrR > "$TESTS_DIR/res015.txt"

echo "  Test 16: overlap flush right big"
printf "%s" "$INPUT" | run_figlet -or -f big > "$TESTS_DIR/res016.txt"

echo "  Test 17: TLF kerning flush right"
printf "%s" "$INPUT" | run_figlet -kr -f tests/emboss > "$TESTS_DIR/res017.txt"

echo "  Test 18: TLF overlap center"
printf "%s" "$INPUT" | run_figlet -oc -f tests/emboss > "$TESTS_DIR/res018.txt"

echo "  Test 19: TLF full width flush left RTL"
printf "%s" "$INPUT" | run_figlet -WRl -f tests/emboss > "$TESTS_DIR/res019.txt"

echo "  Test 20: specify font directory"
# We can't easily replicate test 20 (temp dir) in shell. Skip — kept from existing output.

echo "  Test 21: paragraph mode"
printf "%s" "$INPUT" | run_figlet -p -w250 > "$TESTS_DIR/res021.txt"

echo "  Test 22: short line"
printf "%s" "$INPUT" | run_figlet -w5 > "$TESTS_DIR/res022.txt"

echo "  Test 23: kerning paragraph center small"
printf "%s" "$INPUT" | run_figlet -kpc -f small > "$TESTS_DIR/res023.txt"

echo "  Test 24: list control files"
{
    for flc in $(ls "$FONTS_DIR"/*.flc | sort); do
        echo "$flc" | sed "s|$REPO_ROOT/||"
    done
} > "$TESTS_DIR/res024.txt"

echo "  Test 25: uskata control"
printf "ABCDE" | run_figlet -f banner -C fonts/uskata.flc > "$TESTS_DIR/res025.txt"

echo "  Test 26: jis0201 control"
printf "\xb1\xb2\xb3\xb4\xb5" | run_figlet -f banner -C fonts/jis0201.flc > "$TESTS_DIR/res026.txt"

echo "  Test 27: RTL smushing flowerpower"
printf "%s" "$INPUT" | run_figlet -f tests/flowerpower -R > "$TESTS_DIR/res027.txt"

# --- NEW TESTS 28-50 ---

echo "  Test 28: empty input"
printf "" | run_figlet -f standard > "$TESTS_DIR/res028.txt"

echo "  Test 29: single char"
printf "X" | run_figlet -f standard > "$TESTS_DIR/res029.txt"

echo "  Test 30: explicit smush mode (-m0 = kerning)"
printf "Hello" | run_figlet -f standard -m0 > "$TESTS_DIR/res030.txt"

echo "  Test 31: deutsch flag"
printf "[\\]" | run_figlet -f standard -D > "$TESTS_DIR/res031.txt"

echo "  Test 32: deutsch disabled"
printf "[\\]" | run_figlet -f standard -E > "$TESTS_DIR/res032.txt"

echo "  Test 33: default direction"
printf "Hello" | run_figlet -f standard -X > "$TESTS_DIR/res033.txt"

echo "  Test 34: multibyte disable"
printf "test" | run_figlet -f standard -N > "$TESTS_DIR/res034.txt"

echo "  Test 35: control chars (1-31 skipped)"
printf "a\x01b\x02c\n" | run_figlet -f standard > "$TESTS_DIR/res035.txt"

echo "  Test 36: various widths"
{
    printf "Hello World\n" | run_figlet -f standard -w20
    printf "Hello World\n" | run_figlet -f standard -w40
    printf "Hello World\n" | run_figlet -f standard -w60
    printf "Hello World\n" | run_figlet -f standard -w120
} > "$TESTS_DIR/res036.txt"

echo "  Test 37: smush all rules (full_layout=191)"
# Use standard font with -m191 to explicitly set all 6 smush rules
printf "/\\\\" | run_figlet -f standard -m191 > "$TESTS_DIR/res037.txt"

echo "  Test 38: kern with small font"
printf "Hello World" | run_figlet -f small -k > "$TESTS_DIR/res038.txt"

echo "  Test 39: overlap with standard"
printf "Hi" | run_figlet -f standard -o > "$TESTS_DIR/res039.txt"

echo "  Test 40: full width RTL smush"
printf "abc" | run_figlet -f standard -WR > "$TESTS_DIR/res040.txt"

echo "  Test 41: TLF long text"
printf "%s" "$(cat "$TESTS_DIR/longtext.txt")" | run_figlet -f tests/emboss > "$TESTS_DIR/res041.txt"

echo "  Test 42: cmdinput flag -A"
run_figlet -f standard -A Hello > "$TESTS_DIR/res042.txt"

echo "  Test 43: font dir env"
FIGLET_FONTDIR="$FONTS_DIR" "$FIGLET_BINARY" -f standard "Hello" > "$TESTS_DIR/res043.txt"

echo "  Test 44: ASCII control file (upper.flc)"
printf "abc" | run_figlet -f banner -C fonts/upper.flc > "$TESTS_DIR/res044.txt"

echo "  Test 45: paragraph narrow"
printf "Hello World Foo Bar Baz Qux\n" | run_figlet -f standard -p -w30 > "$TESTS_DIR/res045.txt"

echo "  Test 46: smush mode 0 vs kern combo"
printf "Hello" | run_figlet -f standard -m0 > "$TESTS_DIR/res046.txt"

echo "  Test 47: all fonts kerning"
{
    for stem in $(ls "$FONTS_DIR"/*.flf | xargs -n1 basename | sed 's/\.flf$//' | sort); do
        printf "%s" "$INPUT" | run_figlet -f "fonts/$stem" -k
    done
} > "$TESTS_DIR/res047.txt"

echo "  Test 48: all fonts overlap"
{
    for stem in $(ls "$FONTS_DIR"/*.flf | xargs -n1 basename | sed 's/\.flf$//' | sort); do
        printf "%s" "$INPUT" | run_figlet -f "fonts/$stem" -o
    done
} > "$TESTS_DIR/res048.txt"

echo "  Test 49: long text center"
printf "%s" "$(cat "$TESTS_DIR/longtext.txt")" | run_figlet -f standard -c > "$TESTS_DIR/res049.txt"

echo "  Test 50: big font RTL"
printf "Hello" | run_figlet -f big -R > "$TESTS_DIR/res050.txt"

echo "=== All expected outputs regenerated ==="
