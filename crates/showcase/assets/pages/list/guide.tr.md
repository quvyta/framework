## Ne zaman kullanılır

Kullanıcının içinde gezindiği ve açtığı öğelerden oluşan bir sütun için liste kullan: container'lar, dosyalar, menü girdileri, arama sonuçları. Yalnızca gösterdiği satırları çizdiği için her uzunlukta hızlı kalır.

## Adım adım

1. Satırları kur: `ListItem::new(isim)`; durum işareti için `.icon("dot", Some("success"))`, sessiz sağ sütun için `.detail(durum)`.
2. Bölümleri `ListItem::header(başlık)` ve `ListItem::gap()` ile ayır; bunlar asla seçilmez.
3. Durumunda tuttuğun seçimi göster: `.selected(self.selected)`.
4. Hareketi `.on_select(Msg::Select)`, açmayı `.on_activate(Msg::Open)` ile al.
5. Çoklu seçim için `.checked(bool_listesi)` ve `.on_toggle(Msg::Toggle)` ekle.
6. Boş listenin ne anlama geldiğini söyle: `.empty_text(t!("containers.none"))`.

## Nasıl çalışır

- **Dokunulan satır yükselir.** Hover satırın yüzeyini yumuşak bir çubukla yükseltir; seçili satır daha da yükselir. Çubuk yalnızca liste odaktayken nefes alır; göz klavyenin nerede olduğunu hep bilir.
- **Yalnızca ikon ve etiket kayar.** Hover edilen ya da seçili satır ikonunu ve etiketini bir hücre sağa kaydırır. Çubuk, çoklu seçimin işareti, detay sütunu ve kaydırma çubuğu tam oldukları yerde kalır; işaret her zaman tıkladığın yerdedir. Etiket her zaman bir boş hücre payı ayırır; satır dursa da kaysa da aynı yerden `…` ile kesilir.
- **Fare, tuşların yaptığını yapar.** Tıklamak satırı açar; çoklu seçimde işarete (ya da hemen sonraki hücreye) tıklamak satırı açmadan işaretler.
- **Seçim görünür kalır.** Klavyeyle gezinmek listeyi kaydırır; tekerlek seçimi değiştirmeden kaydırır.
- **Kaydırma çubuğu yalnızca gerektiğinde.** Satırlar taşarsa sağda temanın seçtiği stilde bir kaydırma çubuğu sütunu belirir; sürükle ya da tıkla.
- **Silik satırlar da satırdır.** `.faint(true)` var olan ama hazır olmayan öğeleri çizer, bu menüdeki planlanmış bileşenler gibi.

## Temayla özelleştirme

```toml
[style."list-item:hover"]
bg = "$raised"
pillar = "mix($accent, $surface, 45%)"

[style."list-item:selected:focus"]
pillar = "pulse($accent, $accent-2)"
```

Bir temada kaymayı kapatmak için `[motion]` altında `slide = false` yaz.

## Sık yapılan hatalar

- **Kaydırma konumunu saklamak.** Motor onu tutar; durumun yalnızca seçimi tutar.
- **Kimlikleri yeniden kurmak.** Listeye kalıcı bir `.id` ver, özellikle belirip kaybolabiliyorsa.
- **Yalnızca renkle durum.** Durum noktasını detay sütununda bir kelimeyle eşleştir.
