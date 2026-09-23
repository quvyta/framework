# Fonts

The screenshots are drawn with **JetBrains Mono Nerd Font Mono**, Regular and Bold: JetBrains Mono
with the Nerd Font icons, every glyph one cell wide. Italic is not shipped; it is drawn by slanting
the upright glyphs.

- Source: <https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/JetBrainsMono.tar.xz>
- Release: Nerd Fonts v3.5.1
- Licence: SIL Open Font License 1.1, in `OFL.txt` (copied unchanged from the archive)

The symbols JetBrains Mono has no glyph for come from **JuliaMono**, Regular, cut down to the ones
the ecosystem uses: the search (`⌕`) and settings (`▤`) icons of the Unicode icon set, which the setup
wizard shows as samples on its appearance step, the rest of that set's shapes, and the narrow
no-break space of French typography. JuliaMono's advance is the same share of its em as JetBrains
Mono's (1200 of 2000, 600 of 1000), so a symbol is exactly one cell wide and needs no scaling of
its own. It has no bold; a bold cell draws it regular, as the CJK face does.

- Source: <https://github.com/cormullion/juliamono/releases/download/v0.63.2/JuliaMono-ttf.tar.gz>
  (SHA-256 `be6517295198ec5c92bdbaad42f4f6f8d83f921d80512b79f54fe036add95c0c`), file
  `JuliaMono-Regular.ttf` (SHA-256 `40a07da0d1601215eb6b89312eb44128a3e2f36675d3e1f518264bd391fc7023`)
- Release: JuliaMono v0.63.2
- Licence: SIL Open Font License 1.1, in `OFL-JuliaMono.txt` (the archive's `LICENSE`, unchanged)
- Kept: the whole Geometric Shapes block (U+25A0–25FF), which is where the Unicode icon set draws
  its shapes from, and the single characters the icon sets and texts need: U+202F, U+2139, U+2315,
  U+2610, U+2611, U+2714, U+2756 and the two sextants U+1FB03 and U+1FB07 of the small radio mark.
  108 glyphs, 17,400 bytes — the whole font would be 671,568, and this file ships inside every
  application of the ecosystem.

This machine has FontForge but not fontTools, and nothing is installed for a build, so the cut was
made with FontForge 20251009 (`symbols-subset.py`, which also holds the list of characters).
`SOURCE_DATE_EPOCH` fixes the timestamps FontForge writes, and with it the same command gives the
same bytes:

```sh
SOURCE_DATE_EPOCH=0 python3 symbols-subset.py \
  JuliaMono-Regular.ttf JuliaMono-Regular-subset.ttf
```

Chinese and Japanese come from **Noto Sans Mono CJK SC**, Regular, cut down to what everyday text
needs. Its glyphs are drawn at the size of the Latin text and centred in their two cells; bold
cells draw them regular.

- Source: <https://github.com/notofonts/noto-cjk/releases/download/Sans2.004/13_NotoSansMonoCJKsc.zip>
  (SHA-256 `e252c39994f8a278676507600a955663c23c24a7827dc63a4300b2f7b427cd5d`), file
  `NotoSansMonoCJKsc-Regular.otf` (SHA-256 `ec04cc376b34887cedbdf84074e2e226ed2761eeabdcb9173fc1dd7bfd153ef7`)
- Release: Noto Sans CJK Sans2.004, font version 2.004
- Licence: SIL Open Font License 1.1, in `OFL-NotoSansMonoCJK.txt` (the archive's `LICENSE`, unchanged)
- Kept: CJK symbols and punctuation, hiragana and katakana (U+3000–30FF, U+31F0–31FF), full- and
  half-width forms (U+FF01–FF9F, U+FFE0–FFE6), and the 9,788 ideographs of GB 2312 and JIS X 0208,
  which `cjk-ideographs.py` lists from Python's own codecs. Layout tables, vertical metrics and
  hinting are dropped: the picture places every glyph on the grid itself.

The subset was made with fontTools 4.65.0, and the same command gives the same file:

```sh
python3 cjk-ideographs.py > ideographs.txt
pyftsubset NotoSansMonoCJKsc-Regular.otf --unicodes-file=ideographs.txt \
  --unicodes='U+3000-30FF,U+31F0-31FF,U+FF01-FF9F,U+FFE0-FFE6' --layout-features='' \
  --drop-tables+=GSUB,GPOS,BASE,VORG,vhea,vmtx --no-hinting \
  --output-file=NotoSansMonoCJKsc-Regular-subset.otf
```

| File | SHA-256 |
|---|---|
| `JetBrainsMonoNerdFontMono-Regular.ttf` | `f2a5ea6cfab397445ffab00c0370927b66d61e560a05db5db271b42006381c1a` |
| `JetBrainsMonoNerdFontMono-Bold.ttf` | `bfcf9a917276ffc058867d87cbc8a5b2f1ab0f4b710e9170dc02763ccb80bd4b` |
| `OFL.txt` | `30f0c136e3c88e422d0791acd97238870f9054a9729bc34cf2ff0d4ed8cac4ad` |
| `JuliaMono-Regular-subset.ttf` | `ae35ca54501e5dfffd57906a768d96ca104ce088b4080eb9b22553b702a3589e` |
| `OFL-JuliaMono.txt` | `bae3beff9e15f1a680c2764c4bbdb25b995b9bf539aeaf8467482a495c75ad0c` |
| `NotoSansMonoCJKsc-Regular-subset.otf` | `f85f254146646e9d713f8ff90bf7f2cf44ae962dfbcfc88eb9627f49fcec515f` |
| `OFL-NotoSansMonoCJK.txt` | `6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2` |

The JetBrains Mono files and all three licences are unchanged; `sha256sum -c` against the table
confirms every file.

Which characters the four faces have a glyph for is listed in `coverage.txt`, one section per face.
A test writes that list and compares it, so a font file that is added or cut differently shows as a
diff there instead of as a box in a picture someone happens to draw.
