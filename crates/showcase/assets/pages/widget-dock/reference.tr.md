## Metotlar

- `WidgetDock::new(bölümler)` — gösterim sırasına göre başlıklar; her bileşen için `ui.add_with` ile bir gövde çocuğu ekle.
- `.open(bool dizisi)` — gösterim konumuna göre açık durumu.
- `.on_toggle(|konum, açık| msg)` — bir bileşen açıldı ya da kapandı.
- `.on_move(|from, to| msg)` — sıralamayı açar; `from` konumundan çıkar, `to` konumuna koy.
- `.empty_text(metin)` — hiç bileşen yokken gösterilir.
- `Section::new(başlık)`, `.icon(anahtar)`, `.detail(metin)` — `Accordion` ile ortak.

## Tuşlar

- Odaktayken: `up` `down` `home` `end` başlıklar arasında gezinir, `enter` ya da `space` açıp kapatır, `ctrl+shift+up` `ctrl+shift+down` bileşeni taşır (`on_move` ile).

## Fare

- Açıp kapatmak için başlığa tıkla; taşımak için başlığı sürükle (`on_move` ile).

## Davranış

- Alanını doldurur: önce başlıklar ve aralıklar, sonra açık gövdeler kalan satırları paylaşır; küçük gövdeler doğal yüksekliğini korur, kalanlar eşit böler.
- Açılıp kapanma alan içinde hareketlidir; hareket azaltılmışsa atlar.

## Tema anahtarları

- `Accordion`'ın hepsi: `section`, `section-title`, `section-chevron`, `section-detail`, `section-body`.
- `section-title.ghost` — sürüklenen başlık; `section-drop` — bırakılacak satırın `bg` rengi; `section-empty` — `fg`.
