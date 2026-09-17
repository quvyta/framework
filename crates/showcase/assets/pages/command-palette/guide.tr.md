## Ne zaman kullanılır

Uygulamada akılda tutulabilecek tuşlardan fazla komut varsa komut paleti ekle: sayfalar arasında gezmek, çok sayıda öğe üzerinde işlem yapmak, tema değiştirmek. Her şeye birkaç harfle klavyeden ulaşılır; her komutun tuşunu da gösterdiği için tuşları öğretir.

## Adım adım

1. Açık olup olmadığını tut: `palette_open: bool`; `App::action` içinde genel `palette` eyleminden (`ctrl p`) aç.
2. Komutları `view` içinde kur: `PaletteCommand::new("restart-web", t!("restart", name = "web"), Msg::Restart(id))`.
3. Komutu başka yerde çalıştıran tuşu göster: `.chord("ctrl r")`.
4. Açıkken ekle: `ui.add(CommandPalette::new(commands, Msg::ClosePalette))`.
5. Kısayol haritasının eylemlerini de `.keymap(true)` ile listele; birini çalıştırmak tuşuna basmakla aynıdır.
6. Son komutları sunmak için `.on_run(|id| Msg::Ran(id.to_owned()))` ile gelen kimlikleri durumunda tut ve `.recent(ids)` ile geri ver.

## Nasıl çalışır

- **Kelimelere göre bulanık eşleşme.** Harfler etiketin herhangi bir yerinde sırayla eşleşir; ardışık harfler ve kelime başları öne çıkar, böylece "log red" yazmak "redis loglarını göster" komutunu bulur. Eşleşen karakterler vurgu rengini alır.
- **Önce klavye, fare de aynı ölçüde.** Tuşlar hep süzgeçtedir: ↑/↓ ya da Ctrl+P/Ctrl+N gezer, PgUp/PgDn sayfa atlar, Enter çalıştırır, Esc kapatır. Fareyle bir satırın üstüne gelmek aynı vurguyu taşır, tıklamak çalıştırır, tekerlek ve kaydırma çubuğu kaydırır, × işareti ya da dışarı tıklamak kapatır.
- **Tek vurgu.** Klavye de fare de aynı vurguyu taşır; iki satır aynı anda yanmaz. Palet açılırken tesadüfen bir satırın üstünde duran fare, hareket edene kadar hiçbir şeyi değiştirmez.
- **Kapatılabilir demek Esc, × ve dışarı tıklama birlikte demek.** `.dismissable(false)` üçünü birden kapatır ve işareti gizler; bir komutu çalıştırmak paleti yine kapatır.
- **Çalıştırmak önce kapatır.** Kapatma mesajı komutun mesajından önce gelir; komut başka bir katman açabilir.
- **Sanallaştırılmıştır.** Yalnızca görünen satırlar çizilir, vurgulanan satır görünüme kayar; binlerce komut hızlı kalır.
- **Satırlar liste satırı gibi davranır.** Vurgulanan satır yükselir, yüzeyin sol kenarındaki çubuğu parlatır ve etiketi bir hücre kayar; sağdaki tuş yerinde kalır.
- **Son kullanılanlar önde.** Süzgeç boşken son komutlar "Son kullanılanlar" altında, ardından "Tüm komutlar" gelir.
- **Yukarıda durur.** Palet ekranın üst kısmına sabitlenir; süzdükçe liste süzgecin altında zıplamadan büyüyüp küçülür.

## Sık yapılan hatalar

- **Fiilsiz etiketler.** "Loglar" belirsizdir; "redis loglarını göster" bir eylem gibi okunur.
- **Değişken kimlikler.** Son komutlar kimliğe göre eşleşir; kimliği listedeki sıradan değil, komutun yaptığı işten türet.
- **Komutları kurarken iş yapmak.** `view` her karede çalışır; komutlar yalnızca mesaj taşır.
