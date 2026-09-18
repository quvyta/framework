## Metotlar

- `Tree::new(kökler)` — en üst seviyedeki `TreeNode`'lar.
- `.selected(Option<&str>)` — seçili düğümün anahtarı.
- `.on_select(|anahtar| mesaj)`, `.on_activate(|anahtar| mesaj)`, `.on_expand(|anahtar, açık| mesaj)`.
- `.empty_text(metin)` — düğüm yokken görünür.
- `.reorderable(|adım| mesaj)` — düğümler kardeşleri arasında taşınabilir; `adım`, `key`, `parent` (en üst seviyede `None`), `from` ve `to` taşıyan bir `TreeMove`'dur. `adım.apply(&mut kardeşler)` öğeyi senin listende taşır.
- `.context_menu(|anahtar| öğeler)` — o anahtarlı düğümün `ContextItem`'ları.
- `TreeNode::new(anahtar, etiket)`, `.children(düğümler)`, `.expanded(bool)`, `.expandable(bool)`, `.loading(bool)`.
- `.icon(anahtar, Some(token))`, `.detail(metin)`, `.faint(bool)`.

## Davranış

- Odaktayken tuşlar: `up` `down` ya da `k` `j`, `pgup` `pgdn`, `home` `end` gezinir; `right` ya da `l` açar veya ilk çocuğa geçer; `left` ya da `h` kapatır veya üst düğüme çıkar; `enter` klasörü açıp kapatır, yaprağı etkinleştirir; `space` etkinleştirir.
- Fare: oka tıklama açar ya da kapatır; satıra tıklama önce seçer, sonra `enter` gibi davranır; tekerlek ve kaydırma çubuğu kaydırır.
- Sıralama: bir satıra (okuna değil) basmak onu seçer; imleci bir satır oynatmak sürüklemeye çevirir. İnilecek yer imlecin altında duran kardeş, yoksa ekrandaki en yakın kardeştir; kardeşler bırakmanın vereceği sırayla gösterilir, inilecek yer renklenir ve imleci bir hayalet satır izler. Sürüklenen düğümün çocukları sürükleme boyunca katlanır. Bırakılınca ağaç tek bir `TreeMove` gönderir; başladığı yere bırakılırsa hiçbir şey. Sürükleme olmadan bırakmak satırı `enter` gibi açar, kapatır ya da etkinleştirir. `ctrl+shift+up` ve `ctrl+shift+down` seçili düğümü bir yer taşır, uçlarda durur. Düğüm hiçbir zaman ebeveyn değiştirmez.
- Üst ya da alt satırda veya ötesinde tutulan sürükleme 400 ms sonra bir satır, sonra her 150 ms'de bir kaydırır; kenardan ne kadar uzaksa o kadar sık.
- Bağlam menüsü: bir satıra sağ tık, düğümünün menüsünü imleçte açar ve satırı yükseltilmiş tutar; `shift+f10` ya da menü tuşu seçili düğümün menüsünü, onu görünüme kaydırarak satırının altında açar. Bir öğeyi seçmek mesajını gönderir. Satırların altına sağ tık hiçbir şey yapmaz.
- Her seviye iki hücre girintilidir. Yapraklar okun sütununu boş bırakır; aynı seviyedeki adlar alt alta gelir.
- Hover edilen ya da seçili satırda yalnızca ikon ve etiket bir hücre sağa kayar; girinti, ok (ya da yükleme göstergesi) ve detay yerinde kalır. Etiket bir boş hücre payı ayırır; dururken de kayarken de aynı yerden kesilir.

## Tema anahtarları

- `list-item` (`hover`, `selected`, `focus`, `pressed`); `list-item.faint`; `list-detail`; boş metin için `list-header`.
- `tree-chevron` — `fg`; `hover` ve `selected` ile.
- `spinner` — yüklenen düğümün oku; `scrollbar` — `track`, `thumb`.
- `tab-drop` — sürüklenen düğümün ineceği yer; `tab-ghost` — imleci izleyen satır. Menü `ContextItem` anahtarlarını kullanır.
- İkonlar: `tree-collapsed`, `tree-expanded`, `spinner`.
