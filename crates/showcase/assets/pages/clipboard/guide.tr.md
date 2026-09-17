## Ne zaman kullanılır

Kullanıcının uygulamandan bir şey alması ya da içeri metin getirmesi gerektiğinde panoyu kullan: kurulum komutu, container imaj etiketi, API anahtarı, bir log satırı.

- **CopyValue**, kullanıcının bütün olarak kopyaladığı bir değer için hazır çözümdür. Kopyalandığını yerinde gösterir; "kopyalandı" için bildirim göstermen gerekmez.
- **Seçilebilir metin**, bir logun, belgenin ya da kodun bir kısmını fareyle almayı sağlar; Fareyle metin seçme sayfasına bak.
- **Command::copy**, metne kendi kodunun karar verdiği durumlar içindir; örneğin bir tablonun seçili satırı.
- **Yapıştırma** metin alanlarında senden hiçbir şey istemez. `App::clipboard`'u yalnızca ekranın odaklı bir alan olmadan yapıştırılan metni kabul ettiği durumlarda dinle; örneğin sayfanın herhangi bir yerine bir adres yapıştırmak.

## Adım adım

1. Bir değer göster: `ui.add(CopyValue::new("cargo add quvyta-framework").on_copy(Msg::Copied))`. Enter, Space, `c` ya da tıklama onu kopyalar.
2. Gizli değerleri `.masked(true)` ile sakla: ekranda noktalar çizilir, kopyalanan gerçek değerdir.
3. Metin ekranda bir değer olarak durmuyorsa `update` içinden `Command::copy(metin)` ile kopyala.
4. Bir yapıştır butonu için panoyu `Command::read_clipboard(|metin| Msg::Yapistirildi(metin))` ile oku.
5. Bileşenlerin, menülerin ve kopyalama tuşunun yaptığı kopyaları ve hiçbir bileşenin almadığı yapıştırmaları duymak için `App` üzerinde `fn clipboard(&self, event: &ClipboardEvent) -> Option<Msg>` yaz.

## Nasıl çalışır

- **Kopya iki yere gider.** Her kopya hem OSC 52 ile terminal panosuna (yerelde de SSH üzerinden de çalışır) hem de çalışma zamanının tuttuğu uygulama içi panoya gider.
- **Yapıştırma üç kaynağı sırayla dener.** `ctrl v`, bir alanın menüsündeki Yapıştır ve `Command::read_clipboard` şunları okur:
  1. sistem panosunu aracıyla: Wayland'de `wl-paste`, X11'de `xclip` ya da `xsel`, macOS'ta `pbpaste`. Araç kabuk olmadan, kendi iş parçacığında ve en fazla yarım saniye çalışır; ekran onu hiç beklemez;
  2. araç bir şey vermezse terminalin panosunu bir OSC 52 sorusuyla. Birçok terminal bu soruyu reddeder ya da yok sayar; bu yüzden kısa beklenir, geç gelen cevap tuş olarak düşmek yerine atılır;
  3. uygulama içinde en son kopyalanan metni.
- **Yapıştıracak bir şey yoksa.** Üçünde de metin yoksa Yapıştır seçenekleri pasif olur ve `read_clipboard` `None` getirir.
- **Terminalin kendi yapıştırması da çalışır.** Terminalin yapıştırmasıyla gelen metin bir yapıştırma olayı olarak odaklı bileşene gider.
- **Yapıştırma önce bileşenlere gider.** Odaklı bir metin alanı yapıştırmayı alır; hiçbiri almazsa `App::clipboard` `ClipboardEvent::Pasted` duyar.
- **Seçili metin temiz ya da ham kopyalanır.** Fareyle seçip bırakmak bir şey kopyalamaz. `ctrl c` ve sağ tık menüsündeki Kopyala onu temiz kopyalar: `▌` çubuğu, kaydırma çubukları ve satır numaraları gibi süsler olmadan, satırı yalnızca kenara kadar dolduran boşluklar olmadan. Ham kopyala her hücreyi ekrandaki gibi alır. Demodaki başlıkla dene: ikisini de nota yapıştırıp karşılaştır.
- **Onay sessizdir.** Sondaki `kopyala` kelimesi başarı işaretiyle `kopyalandı` olur, bir süre durur ve boştaki rengine karışarak döner; zemin bir kez parlar.

## Sık yapılan hatalar

- **Her kopya için bildirim göstermek.** CopyValue onayı zaten kullanıcının baktığı yerde gösterir.
- **`ClipboardEvent::Copied`'a `Command::copy` ile cevap vermek.** Senin istediğin kopyalar bildirilmez; yine de her bildirimde başka bir metni kopyalamak kullanıcıyı şaşırtır.
- **`read_clipboard`'un hemen cevap vermesini beklemek.** Mesajı, sistem aracı ya da terminal cevap verdikten sonra, sonraki bir güncellemede gelir.
- **Gizlemek için değeri değiştirmek.** Gerçek değerin kopyalanması için `.masked(true)` kullan.
