## Ne zaman kullanılır

Bir şey kuran bir akış yazarken bu örneği oku: yeni bir proje, bir hesap, bir dağıtım. Yalnızca framework bileşenleriyle yazılmış eksiksiz bir uygulama ekranıdır: sihirbaz, formlar, radyo grubu, ayar listesi, onay kutuları, kod önizlemesi, adım göstergesi ve ilerleme çubuğu.

## Adım adım

1. **Önce durum.** Tek bir yapı her yanıtı, güncel adımı, hataları ve oluşturmanın sürüp sürmediğini ya da bitip bitmediğini tutar.
2. **Her adıma bir doğrulama.** `validate(state, step)`, kontrol kimlikleriyle adlandırılmış `FormErrors` döndürür; kuralları başka hiçbir yer bilmez.
3. **İleri doğrular, ilerletir ve odaklar.** Başarılıysa adım ilerler ve `Command::focus` imleci yeni sayfanın ilk kontrolüne koyar; başarısızsa `focus_first` soruna gider.
4. **Her sayfa ona uyan bileşeni kullanır.** Ad ve klasör bir `Form`; üç seçenekten biri `RadioGroup`; tek tek uygulanan tercihler `SettingsList`; isteğe bağlı özellikler silik açıklamalı `Checkbox`'lar.
5. **Özet ne olacağını gösterir.** Bir `CodeView`, yanıtlardan kurulan proje dosyasını önizler.
6. **Bitir arka plan işini başlatır.** `Command::perform` her oluşturma aşamasını çizimden ayrı çalıştırır; biten her aşama bir sonrakini başlatan bir mesaj gönderir. Bu sırada dikey `Steps` ve bir `ProgressBar` ilerlemeyi gösterir.
7. **Son ekran sıradaki adımı söyler** ve baştan başlamayı önerir.

## Nasıl çalışır

- **Örnekte hiçbir şey elle çizilmez.** Her yüzey, işaret ve hareket bileşenlerden ve temadan gelir; akış her temada, dilde ve karakter modunda doğru görünür.
- **Sihirbaz butonlarını `page_height` ile yerinde tutar**; kısa ve uzun sayfalar onları zıplatmaz.
- **Geri dönmek yanıtları korur.** Değerler durumda yaşar; Geri ve bitmiş adımı seçmek yalnızca `step` değerini değiştirir.
- **Esc ve Vazgeç baştan başlatır**, çünkü sihirbazın `on_cancel` mesajı vardır.
- **İlk denemeden sonra hatalar düzenlemeyi izler.** Bir adımda sorun çıktıktan sonra her değişiklik onu yeniden doğrular; değer düzelince mesaj hemen kaybolur.

## Sık yapılan hatalar

- **Yavaş işi `update` içinde yapmak.** Dosya oluşturmak, motoru çağırmak ya da depo klonlamak `Command::perform` işidir; çizim asla beklememeli.
- **Her şeyi tek sayfada sormak.** Soruları konuya göre grupla; her sayfa tek bir şeyi yanıtlasın.
- **Bitir'in ne yaptığını gizlemek.** Geri alınamayan adımdan önce bir özet göster.
