# E2E Testing Checklist — `--create-font` & FIGfont Configurations

Build: `cargo build -p figby`  
Binary: `figby-rs/target/debug/figby`  
Font dir: `-d fonts` (from repo root)

---

## Section 1: Basic Font Creation

Test generating FIGfonts from system TTF/OTF fonts. Available monospace fonts on this system: `Courier 10 Pitch`, `DejaVu Sans Mono`, `FreeMono`, `Liberation Mono`, `Ubuntu Mono`, `Ubuntu Sans Mono`.

### 1.1 Generate a FIGfont from a system font
```bash
./figby-rs/target/debug/figby --create-font "DejaVu Sans Mono" --create-font-charset smooth > /tmp/test_font.flf
```
- [x] Check exit code 0
- [x] File is non-empty
- [x] File starts with `flf2a$`
- [x] First header line parses: `flf2a$ <height> <baseline> <max_length> 0 0 -1 64 0`
- [x] Notes: Header: `flf2a$ 15 12 8 0 0 -1 64 0`. Uses `--create-font-charset smooth` for antialiased glyphs.

### 1.2 Generate with different font size
```bash
./figby-rs/target/debug/figby --create-font "DejaVu Sans Mono" --font-size 8 --create-font-charset smooth > /tmp/test_font_8.flf
./figby-rs/target/debug/figby --create-font "DejaVu Sans Mono" --font-size 24 --create-font-charset smooth > /tmp/test_font_24.flf
```
- [x] Size 8 is smaller than size 24 (check `charheight` in header)
- [x] Size 24 has visible glyph detail
- [x] Notes: 8pt: h=10 b=8 ml=5. 24pt: h=29 b=23 ml=15.

### 1.3 Generate from different system fonts
```bash
./figby-rs/target/debug/figby --create-font "Courier 10 Pitch" --create-font-charset smooth > /tmp/courier.flf
./figby-rs/target/debug/figby --create-font "Liberation Mono" --create-font-charset smooth > /tmp/liberation.flf
./figby-rs/target/debug/figby --create-font "FreeMono" --create-font-charset smooth > /tmp/freemono.flf
```
- [x] Courier generates successfully
- [x] Liberation Mono generates successfully
- [x] FreeMono generates successfully
- [x] All have correct header format
- [x] Notes: All generate with proper headers. Courier: flf2a$ 15 11 8. Liberation: flf2a$ 14 10 8. FreeMono: flf2a$ 13 10 8.

### 1.4 Generate with `--output` flag
```bash
./figby-rs/target/debug/figby --create-font "DejaVu Sans Mono" --create-font-charset smooth --output /tmp/created_font.flf
```
- [x] File written to specified path
- [x] Contents identical to stdout version (diff check)
- [x] Notes: stdout == file: YES

### 1.5 Error on nonexistent font
```bash
./figby-rs/target/debug/figby --create-font "TotallyFakeFontXYZ"
```
- [x] Exits with non-zero code
- [x] Prints error message: `Error creating font: font not found: TotallyFakeFontXYZ`
- [x] Notes:

---

## Section 2: Generated Font Quality

### 2.1 All 102 required chars present
```bash
rg "^[0-9]+$" /tmp/test_font.flf | wc -l      # count codetagged sections
head -1 /tmp/test_font.flf                      # check header
```
- [x] Chars 32-126 (95 ASCII printable) are all present
- [x] 7 Deutsch chars (196, 214, 220, 228, 246, 252, 223) present
- [x] Total chars = 102 (no codetagged unless added)
- [x] Notes: 1531 lines = 1 header + 102×15 rows. Codetagged count: 0.

### 2.2 Render text with generated font
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -W
```
- [x] Output renders recognizable "Hello Figby"
- [x] All characters are correctly shaped
- [x] No garbled or misaligned glyphs
- [x] Notes: Use `-W` (full width) for clearest view. Default kerning mode also works but chars overlap at small sizes.

### 2.3 Round-trip: generate → parse → re-generate → compare
```bash
./figby-rs/target/debug/figby --create-font "DejaVu Sans Mono" > /tmp/roundtrip_a.flf
./figby-rs/target/debug/figby -f /tmp/roundtrip_a.flf "TEST" > /tmp/roundtrip_output.txt
```
- [x] Font loads without errors
- [x] Output looks correct
- [x] Notes: Round-trip works, "TEST" renders.

### 2.4 Check hardblank placement
```bash
head -1 /tmp/test_font.flf | awk '{print substr($1,6,1)}'
```
- [x] Hardblank `$` defined in header
- [x] Hardblank NOT used in glyph rows (only 1 occurrence total = header)
- [x] Notes: Hardblank `$` in glyphs would display as space in output (renderer replaces). SMOOTH_CHARSET deliberately avoids `$` and `@` (endmark) to prevent output corruption.

### 2.5 Check baseline alignment
```bash
# Header shows h=15 b=12. Check p/g/y at lines 1202-1351 for descenders
```
- [x] Baseline value is correct (check header)
- [x] Characters with descenders (g, j, p, q, y) extend below baseline
- [x] Capital letters sit on baseline
- [x] Notes: h=15 b=12. Chars with descenders extend into rows 12-14 (below baseline). Verified p/g/y.

---

## Section 3: Generated Font Header Parameters

Check the header of the generated font and verify against spec.

### 3.1 Header format verification
```bash
head -1 /tmp/test_font.flf
```
Expected: `flf2a$ <h> <b> <ml> 0 0 -1 64 0`
- [ ] `flf2a` signature present
- [ ] Hardblank = `$`
- [ ] `old_layout` = 0 (fitting/kerning backward compat)
- [ ] `comment_lines` = 0 (no comments generated)
- [ ] `print_direction` = -1 (unset, uses driver default)
- [ ] `full_layout` = 64 (horizontal fitting/kerning by default)
- [ ] `codetag_count` = 0
- [ ] Notes:

### 3.2 Verify header values make sense
```bash
# Parse header
head -1 /tmp/test_font.flf | awk '{print "sig:",$1, "hb:",substr($1,5,1), "h:",$2, "b:",$3, "ml:",$4, "ol:",$5, "cl:",$6, "pd:",$7, "fl:",$8, "cc:",$9}'
```
- [ ] `charheight` >= `baseline` >= 1
- [ ] `baseline` <= `charheight`
- [ ] `max_length` >= widest character width
- [ ] Notes:

---

## Section 4: Render with Different Layout Modes

Test the generated font with all layout/smushing flags against FIGlet spec.

### 4.1 Font default (no layout flags) — uses font's full_layout
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf
```
- [ ] Uses kerning (full_layout=64) by default
- [ ] Output looks reasonable
- [ ] Notes:

### 4.2 Full width mode (-W = no kerning or smushing)
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -W
```
- [ ] Characters are fully spaced (full width)
- [ ] Each char occupies its full design width
- [ ] Notes:

### 4.3 Kerning only (-k)
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -k
```
- [ ] Characters moved closer together (touching)
- [ ] No smushing (characters don't overlap)
- [ ] Notes:

### 4.4 Smushing forced (-o)
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -o
```
- [ ] Characters overlap/merge at contact points
- [ ] Visible smushing effect
- [ ] Notes:

### 4.5 Smushing combined with font layout (-S)
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -S
```
- [ ] Combines font's layout bits with smushing mode
- [ ] Notes:

### 4.6 Font default layout (-s)
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -s
```
- [ ] Uses font's `full_layout` as-is
- [ ] Same as no flag (SmushOverride::No)
- [ ] Notes:

### 4.7 Manual smush mode (-m)
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -m -1   # kerning
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -m 0    # fitting
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -m 1    # smush rule 1 only
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -m 63   # all smush rules
```
- [ ] `-m -1` = kerning (like `-k`)
- [ ] `-m 0` = fitting (like `-W` basically)
- [ ] `-m 1` applies only equal-char smushing
- [ ] `-m 63` applies all 6 horizontal smushing rules
- [ ] Notes:

---

## Section 5: Render Width & Justification

### 5.1 Output width control
```bash
echo "Hello Figby This is a longer test string to see wrapping behavior" | \
  ./figby-rs/target/debug/figby -f /tmp/test_font.flf -w 40
```
- [ ] Output wraps at 40 columns
- [ ] Lines break cleanly at word boundaries
- [ ] Notes:

### 5.2 Terminal width (-t)
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -t
```
- [ ] Output uses terminal width
- [ ] Notes:

### 5.3 Justification: left (-l)
```bash
echo "Hello" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -w 80 -l
```
- [ ] Text left-aligned
- [ ] Notes:

### 5.4 Justification: center (-c)
```bash
echo "Hello" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -w 80 -c
```
- [ ] Text centered in 80 cols
- [ ] Notes:

### 5.5 Justification: right (-r)
```bash
echo "Hello" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -w 80 -r
```
- [ ] Text right-aligned
- [ ] Notes:

### 5.6 Justification: font default (-x)
```bash
echo "Hello" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -x
```
- [ ] Uses font's default (for generated font with print_direction=-1, this is LTR → left)
- [ ] Notes:

---

## Section 6: Writing Direction

### 6.1 Left-to-right (-L)
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -L
```
- [ ] Renders LTR correctly
- [ ] Notes:

### 6.2 Right-to-left (-R)
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -R
```
- [ ] Renders RTL (FIGlet flips the order — last char first)
- [ ] Justification defaults to right for RTL
- [ ] Notes:

### 6.3 Font default direction (-X)
```bash
echo "Hello Figby" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -X
```
- [ ] Uses font's print_direction (LTR since -1 → 0)
- [ ] Notes:

---

## Section 7: Generated Font vs System FIGlet (if installed)

### 7.1 Compare output against C FIGlet for identical input
```bash
echo "Hello World" | figlet -f /tmp/test_font.flf 2>/dev/null || echo "figlet not installed"
echo "Hello World" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf
```
- [ ] If figlet installed, outputs match (may differ if system figlet can't load the font)
- [ ] Notes:

### 7.2 Compare against a bundled FIGlet font for sanity
```bash
echo "Hello" | ./figby-rs/target/debug/figby -d fonts -f standard
echo "Hello" | figlet -f standard 2>/dev/null || true
```
- [ ] Figby+standard font renders recognizably
- [ ] Notes:

---

## Section 8: Deutsch (German) Character Handling

### 8.1 Deutsch flag enabled (-D)
```bash
echo "Ä Ö Ü ä ö ü ß" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -D
```
- [ ] German chars render (may be placeholders if glyphs missing)
- [ ] Notes:

### 8.2 Deutsch flag disabled (-E)
```bash
echo "Ä Ö Ü ä ö ü ß" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -E
```
- [ ] German chars may not render (depends on font's char table)
- [ ] Notes:

---

## Section 9: Paragraph Mode

### 9.1 Paragraph mode on (-p)
```bash
printf "Hello\nworld\n" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -p
```
- [ ] Single newlines treated as spaces within paragraph
- [ ] Double newlines start new paragraph
- [ ] Notes:

### 9.2 Paragraph mode off (-n)
```bash
printf "Hello\nworld\n" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf -n
```
- [ ] Newlines break lines as expected
- [ ] Notes:

---

## Section 10: FIGfont Spec Configuration Tests

Test whether the generated font header values conform to spec expectations.

### 10.1 Old_Layout = 0, Full_Layout = 64

From figfont.txt: Old_Layout=0 + Full_Layout includes 64 = horizontal fitting (kerning) default.
- [ ] Default rendering uses kerning (chars touch but don't overlap)
- [ ] This matches "Step-by-Step: horizontal fitting as default"
- [ ] Notes:

### 10.2 Verify no codetag corruption
```bash
# Count actual chars vs header's codetag_count
total_chars=$(grep -c "^.\+@$" /tmp/test_font.flf)  # rough count
```
- [ ] `codetag_count` in header = 0 (only 102 required chars)
- [ ] Notes:

### 10.3 Check comment lines handling
```bash
comment_count=$(head -1 /tmp/test_font.flf | awk '{print $6}')
echo "Comment lines: $comment_count"
```
- [ ] `comment_lines` = 0
- [ ] No stray lines between header and char data
- [ ] Notes:

---

## Section 11: Edge Cases

### 11.1 Render very short input
```bash
echo "A" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf
```
- [ ] Single character renders
- [ ] No crash
- [ ] Notes:

### 11.2 Render very long input (wrap)
```bash
echo "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" | \
  ./figby-rs/target/debug/figby -f /tmp/test_font.flf -w 40
```
- [ ] Long string wraps correctly
- [ ] No crash or memory issues
- [ ] Notes:

### 11.3 Render with special characters
```bash
echo "!@#$%^&*()_+-=[]{}|;:',.<>?/~\`" | ./figby-rs/target/debug/figby -f /tmp/test_font.flf
```
- [ ] All special chars render without error
- [ ] Notes:

### 11.4 Generate font with different font families (not just monospace)
```bash
./figby-rs/target/debug/figby --create-font "Ubuntu" 2>&1 || true
```
- [ ] Non-monospace fonts may still generate (check behavior)
- [ ] Notes:

### 11.5 Pipe stdin to --create-font (should read from args, not stdin)
```bash
echo "hello" | ./figby-rs/target/debug/figby --create-font "DejaVu Sans Mono"
```
- [ ] `--create-font` ignores stdin (just generates font from system font name)
- [ ] Notes:

---

## Notes / Bugs Found

Use this space during testing:

| # | Section | Issue | Severity |
|---|---------|-------|----------|
|   |         |       |          |
|   |         |       |          |
|   |         |       |          |

Severity: `CRITICAL` (crash/data loss) / `MAJOR` (wrong output) / `MINOR` (cosmetic) / `ENHANCEMENT`

---

## Test Completion

- [ ] All sections completed
- [ ] Notes compiled for each section
- [ ] Bugs reported (list above)
- [ ] Date tested:
