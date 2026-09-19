"""Prints the ideographs of the embedded CJK font as a pyftsubset --unicodes list.

Every ideograph of GB 2312 (simplified Chinese, both levels) and of JIS X 0208 (Japanese, both
levels): together they hold the characters of everyday Chinese and Japanese text, the Joyo kanji
among them, and read from Python's own codecs they need no list kept by hand.
"""


def decode_all(codec, rows):
    found = set()
    for high in rows:
        for low in range(0xA1, 0xFF):
            try:
                found.update(bytes([high, low]).decode(codec))
            except UnicodeDecodeError:
                pass
    return found


hanzi = decode_all("gb2312", range(0xB0, 0xF8))
kanji = decode_all("euc_jp", range(0xB0, 0xF5))
ideographs = sorted(c for c in hanzi | kanji if 0x4E00 <= ord(c) <= 0x9FFF)
print(",".join(f"U+{ord(c):04X}" for c in ideographs))
