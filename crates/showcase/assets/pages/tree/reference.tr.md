## Metotlar

- `Tree::new(kökler)` — en üst seviyedeki `TreeNode`'lar.
- `.selected(Option<&str>)` — seçili düğümün anahtarı.
- `.on_select(|anahtar| mesaj)`, `.on_activate(|anahtar| mesaj)`, `.on_expand(|anahtar, açık| mesaj)`.
- `.empty_text(metin)` — düğüm yokken görünür.
- `TreeNode::new(anahtar, etiket)`, `.children(düğümler)`, `.expanded(bool)`, `.expandable(bool)`, `.loading(bool)`.
- `.icon(anahtar, Some(token))`, `.detail(metin)`, `.faint(bool)`.

## Davranış

- Odaktayken tuşlar: `up` `down` ya da `k` `j`, `pgup` `pgdn`, `home` `end` gezinir; `right` ya da `l` açar veya ilk çocuğa geçer; `left` ya da `h` kapatır veya üst düğüme çıkar; `enter` klasörü açıp kapatır, yaprağı etkinleştirir; `space` etkinleştirir.
- Fare: oka tıklama açar ya da kapatır; satıra tıklama önce seçer, sonra `enter` gibi davranır; tekerlek ve kaydırma çubuğu kaydırır.
- Her seviye iki hücre girintilidir. Yapraklar okun sütununu boş bırakır; aynı seviyedeki adlar alt alta gelir.
- Hover edilen ya da seçili satırda yalnızca ikon ve etiket bir hücre sağa kayar; girinti, ok (ya da yükleme göstergesi) ve detay yerinde kalır. Etiket bir boş hücre payı ayırır; dururken de kayarken de aynı yerden kesilir.

## Tema anahtarları

- `list-item` (`hover`, `selected`, `focus`, `pressed`); `list-item.faint`; `list-detail`; boş metin için `list-header`.
- `tree-chevron` — `fg`; `hover` ve `selected` ile.
- `spinner` — yüklenen düğümün oku; `scrollbar` — `track`, `thumb`.
- İkonlar: `tree-collapsed`, `tree-expanded`, `spinner`.
