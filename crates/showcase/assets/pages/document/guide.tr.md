## Nerede kullanılır

Dosya kullanıcının değil uygulamanın kendisininse belge kullan: bir proje dosyası, bir profil tanımı, bir oturum kaydı, bir kaynak listesi. Şeklini kendin tanımladığın bir TOML dosyasıdır, tablo dizisi (`[[profile]]`) taşıyabilir ve arkandan hiç onarılmaz.

`Settings` bunun diğer yarısıdır, aynısı değil. Ayarlar tercihtir: her anahtarın bir varsayılanı vardır, kendini onarma dosyayı yeniden yazabilir ve saklayamadığı değeri atar. Belgenin varsayılanı yoktur, tablo ve tablo dizisi taşır, okunması ise hiçbir şey yazmaz — ne dosyayı, ne de yanına bir yedek.

Kararı veren dört fark:

- **Dosya kimin.** `Settings` kullanıcının tercihlerini tutar; belge uygulamanın verisini.
- **Varsayılanlar.** Her ayar anahtarının bir varsayılanı vardır, bu yüzden eksik anahtar sorun değildir. Belge anahtarı zorunlu ya da isteğe bağlıdır ve yerine hiçbir şey konmaz.
- **Ne taşıyabilir.** `SettingValue`'da tablo yok, bu yüzden `[[profile]]` kayıtları atlanır. Belgede `Shape::table` ve `Shape::entries` var.
- **Bozuk dosyanın bedeli.** Kendini onaran ayarlar yeniden yazılabilir, eski dosya `settings.toml.bak` olarak kalır. Bozuk bir belge yalnızca bildirilir; kaydetmek `storage::atomic_write` ile kendi attığın bir adımdır.

## Adım adım

1. Dosyayı bir kez tanımla: `Shape::new().required("id", ValueKind::text())`, `.optional(…)`, içindeki tablo için `.table("engine", …)`, her `[[profile]]` için bir kayıt isteyen `.entries("profile", …)`.
2. Oku: `Document::open(yol, &shape)?`, ya da metni elinde tutuyorsan `Document::parse(ad, metin, &shape)`.
3. İhtiyacın olanı `document.root()` üzerinden al: `text`, `integer`, `flag`, `table` ve `entries`. Eksik ya da okunamayan her şey için `None` — ya da hiç kayıt — dönerler.
4. `document.diagnostics()`'i kullanıcıya göster. Her biri sorunun bulunduğu dosya, satır ve sütunu taşır.
5. Zorunlu anahtarları olmayan bir belgenin senin için ne anlama geldiğine karar ver. `Document` bildirir; geri kalanının açılmaya değip değmediğini yalnızca sen bilirsin.
6. Kaydederken metni kendin kur ve `storage::atomic_write` ile yaz. Okumak ile yazmak bilerek iki ayrı adımdır.

## Nasıl çalışır

- **Bütün denetim şekildir.** Tanımlı türde tanımlı bir anahtar saklanır. Kimsenin tanımlamadığı bir anahtar uyarıdır ve atılır. Başka türden bir değer de atılır: anahtar zorunluysa hata, isteğe bağlıysa uyarı olur. Dosyada hiç bulunmayan zorunlu bir anahtar hatadır ve bulunduğu tablonun başladığı yerde bildirilir — dizi kaydı için onu açan `[[profile]]` satırında.
- **Bozuk belge okunabilen kısmını yine verir.** Kullanıcının elle düzenlediği bir dosya genelde tek bir yerinden bozuktur. Okunamayan bir profili eksik de olsa projeyi yine adlandırabilen bir uygulama, hiçbir şey açamayandan işe yarardır; bu framework'teki her yükleyici de böyle okur. Neyin kaybedildiğini tanılar tam olarak söyler.
- **Okurken hiçbir şey yazılmaz.** Onarım yok, dosyaya yazılan varsayılan yok, `.bak` dosyası yok. Bu kapatılabilir bir seçenek de değil: tipin yazma yolu hiç yoktur.
- **Sözdizimi hatası okumayı bitirmez.** Ayrıştırıcı kendini toplar, hata satır ve sütunuyla bildirilir ve sonrasındaki anahtarlar yine okunur.
- **Anahtarlar addır, yol değil.** İç içelik `Shape::table` ile tanımlanır; `[engine]` altındaki `kind`'ı da `engine.kind` yazımını da aynı dosya olarak okur.
- **Yazmak sende kalır.** Dosyanın sırasını, gruplamasını ve yorumlarını senin yapın bilir; genel bir yazıcı bilmez. Güvenli yazmanın kendisi `storage::atomic_write`'tır; demo, uygulamanın ona vereceği dosyayı gösterir.

## Sık yapılan hatalar

- **Veri dosyası için `Settings` kullanmak.** Tablo değeri olmadığı için `[[profile]]` kayıtları tek kelime edilmeden atlanır; kendini onarma açıkken dosya onlar olmadan yeniden yazılır.
- **Her tanıyı başarısızlık saymak.** Çoğu "bu kayıt gitti" demektir, "bu dosya kullanılamaz" değil. Önce `root()`'u oku, sonra karar ver.
- **Olmayan dosyadan boş belge beklemek.** `Document::open` I/O hatasını döner, çünkü "henüz yazılmadı" ile "boş yazıldı" başka şeylerdir ve hangisini beklediğini yalnızca sen bilirsin.
- **Şekilde varsayılan tanımlamak.** Varsayılan yok. Geri düşüşlerini değerin kullanıldığı yerde tut, böylece dosya bizim değil kullanıcının sözü kalır.
- **Kendi sorununu yersiz bildirmek.** Bir değer okunuyor ama senin için anlamsızsa — dosya sisteminin kabul etmediği bir ad, takvimin bilmediği bir gün — `Table::value_location(key)` değerin kendisinin satır ve sütununu verir; belgenin kendi tür hataları da oraya işaret eder. `Table::location(key)` anahtarınkini verir.
