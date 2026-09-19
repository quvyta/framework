# Fonts

The screenshots are drawn with **JetBrains Mono Nerd Font Mono**, Regular and Bold: JetBrains Mono
with the Nerd Font icons, every glyph one cell wide. Italic is not shipped; it is drawn by slanting
the upright glyphs.

- Source: <https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/JetBrainsMono.tar.xz>
- Release: Nerd Fonts v3.5.1
- Licence: SIL Open Font License 1.1, in `OFL.txt` (copied unchanged from the archive)

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
| `NotoSansMonoCJKsc-Regular-subset.otf` | `f85f254146646e9d713f8ff90bf7f2cf44ae962dfbcfc88eb9627f49fcec515f` |
| `OFL-NotoSansMonoCJK.txt` | `6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2` |

The JetBrains Mono files and both licences are unchanged; `sha256sum -c` against the table
confirms every file.
