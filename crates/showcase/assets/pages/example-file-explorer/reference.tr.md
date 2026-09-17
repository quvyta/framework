## Metotlar

- `Tree::new(düğümler)`; `.selected`, `.on_select`, `.on_expand` ile; düğümlerde `.expandable(true)`, `.expanded(bool)`, `.loading(bool)` ve `.children(..)`; okunamayan klasör `.icon("error", Some("danger"))` taşıyan `.faint(true)` bir çocuk alır.
- `Table::new(sütunlar, satırlar)`; `.selected`, `.sort`, `.on_select`, `.on_activate`, `.on_sort`, `.empty_text` ile; hücrelerde `TableCell::new(..).icon("folder", Some("accent"))`.
- `Command::perform` içinde `read_folder(yol)`; `FileEntry` değerlerinden bir `Listing` verir.
- Bir `ScrollView` içinde `CodeView::new(metin, Language::Rust | Toml | Plain)` ve `Markdown::new(metin)`.
- Yol ve durum için `Span::role("faint" | "secondary" | "body")` ve `.bold()` ile `Text::rich(parçalar)`.

## Davranış

- Ağaçta bir klasör seçmek onu bulunulan klasör yapar; bir düğümü açmak onu ilk seferde okur.
- Tabloda bir satır seçmek dosyayı önizler; bir klasör satırını açmak onu bulunulan klasör yapar ve ağaçta üst klasörlerini açar.
- Hâlâ okunan bir klasör ya da dosya gösterileni temizlemez: tablo ve önizleme cevap gelince tek karede değişir. 300 ms'den uzun okunan bir klasörün ağaç satırı en az 500 ms döner.
- Sıralama klasörleri üstte tutar, kalanı ada ya da boyuta göre dizer.
- Önizleme en fazla 64 KiB ve 400 satır okur; sıfır baytı olan dosyalar ikili olarak bildirilir.
- Gizli girdiler (adı nokta ile başlayanlar) gösterilmez.

## Tema anahtarları

- Örnek yeni stil eklemez: satırlar `list-item`, başlık `table-header`, oklar `tree-chevron`, önizleme `code` ve `markdown-*`, yol `faint` ve `secondary` tipografisini kullanır.
