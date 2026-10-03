"""Cuts JuliaMono down to the symbols the ecosystem's icon sets and texts use.

A FontForge script, not pyftsubset: the machine that made the file has FontForge but no fontTools,
and nothing is installed for a build. `SOURCE_DATE_EPOCH` fixes the timestamps FontForge writes,
so the same command gives the same bytes.

    SOURCE_DATE_EPOCH=0 python3 symbols-subset.py \\
        JuliaMono-Regular.ttf JuliaMono-Regular-subset.ttf

Kept: whole blocks of the symbols a terminal program draws — Arrows, Miscellaneous Technical,
Geometric Shapes (where the Unicode icon set draws its shapes from), Miscellaneous
Symbols, Dingbats and Miscellaneous
Symbols and Arrows — so a recorded terminal tab running a real coding tool, which marks its replies,
tool calls and spinners with these, draws every frame; the single characters the icon sets and the
French texts need; and the two sextants the small radio mark is drawn with. Everything else goes, including the
layout and hinting tables: the picture places every glyph on the grid itself.
"""

import sys

import fontforge

#: Whole blocks, as far as JuliaMono has them: Arrows (↯ ↵), Miscellaneous Technical (⏺ ⎿ ⏵ ⏸ ⎔),
#: Geometric Shapes (▣ ▤ ▥ ▩ ◐ ◑ ◒ ◓ …), Miscellaneous Symbols (⚙ ☐ ☑ ★), Dingbats (✢ ✱ ✳ ✻ ✽ ✘) and Miscellaneous Symbols and
#: Arrows (⬝). A character of a block the font lacks is skipped quietly; only a single listed below
#: is reported missing.
BLOCKS = [range(0x2190, 0x2200), range(0x2300, 0x2400), range(0x25A0, 0x2600), range(0x2600, 0x2700), range(0x2700, 0x27C0), range(0x2B00, 0x2C00)]

#: The rest, one by one: narrow no-break space (French typography), ℹ information, ⌕ search,
#: ☐ ☑ ballot boxes, ✔ check mark, ❖ the ecosystem's own mark, and the two sextants of the small
#: radio mark.
SINGLES = [0x202F, 0x2139, 0x2315, 0x2610, 0x2611, 0x2714, 0x2756, 0x1FB03, 0x1FB07]


def main(source, output):
    font = fontforge.open(source)
    font.selection.none()
    for code in list(SINGLES) + [code for block in BLOCKS for code in block]:
        try:
            font[code]
        except TypeError:
            if code in SINGLES:
                print(f"U+{code:04X} is not in {source}", file=sys.stderr)
            continue
        font.selection.select(("more", "unicode"), code)
    font.selection.invert()
    font.clear()
    font.generate(output, flags=("omit-instructions", "short-post"))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
