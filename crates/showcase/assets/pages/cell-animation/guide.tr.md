## Ne zaman kullanılır

Tek bir hücrenin hareket etmesi gerektiğinde hücre animasyonu kullan: bir spinner stili, biten işin onay işareti, yanıp sönen bir durum noktası, kayıt ışığı. Spinner stillerinin hepsi birer hücre animasyonudur; aynı biçimle hem gömülüleri yeniden tasarlarsın hem kendi animasyonlarını eklersin. Bir dosyaya yazmadan önce karakterleri ve renkleri bu sayfadaki stüdyoda üç karakter modunda dene.

## Adım adım

1. **Bir başlangıç seç.** Stüdyoda *Başlangıç* listesinden gömülü bir animasyon ya da *Yeni animasyon* seç. Gömülüler listesindeki bir ada basmak da onu açar.
2. **Kareleri yaz.** Her karenin üç karakter alanı vardır. Önce ASCII'yi doldur; her kare onu ister. Unicode boş kalırsa orada ASCII karakteri, Nerd Font boş kalırsa Unicode karakteri kullanılır. Alanlar `U+F0995` ya da `5` gibi bir kod noktası da alır; klavyeden yazılamayan Nerd Font karakterleri için işe yarar.
3. **Rengi anlamı olan yere koy.** *Renk* boşsa bileşenin rengi kullanılır. Tema rengi için `$success`, başarı rengiyle bileşen renginin tam ortası için `mix($success, $fg, 50%)`, nefes alması için `pulse($muted, $fg)`, sabit bir renk için `#38BDF8` yaz.
4. **Hızı ayarla.** *Kare süresi* ya bir tema hareket anahtarını (`spinner`, `step`, …) izler, böylece hızı her tema kendisi belirler, ya da sabit bir milisaniye değeridir. Bir karenin *Süre* alanı yalnızca o kare için bunu geçersiz kılar.
5. **Oynatmayı ve renkleri seç.** `loop` tekrar eder, `once` son karede durur, `bounce` ileri gidip geri döner. `step` her kareyi kendi renginde gösterir; `blend` rengi bir sonraki kareye doğru kaydırır.
6. **TOML'u kopyala** ve `[animations.<ad>]` bloğunu bir ikon seti ya da tema dosyasına yapıştır, ya da dosyaları kim yönetiyorsa ona gönder.
7. **Kullan.** `Spinner::new().animation("kayit-isigi")`, `Toast::info("Kaydediliyor").icon_motion("kayit-isigi")` ya da kendi bileşeninde `cx.animation("kayit-isigi", stil, Some(Duration::ZERO))`.

## Nasıl çalışır

- **Biçim.** Bir `[animations.<ad>]` tablosunda `frame` (bir hareket anahtarı ya da `"80ms"` gibi bir süre, varsayılanı `"spinner"`), `playback` (`loop`, `once`, `bounce`), `colors` (`step`, `blend`), isteğe bağlı ve 1'den sayılan `rest` kare numarası ve `{ nerd, unicode, ascii, color, duration }` listesi olan `frames` bulunur.
- **Yedekler.** Nerd Font karakteri yoksa Unicode, Unicode karakteri yoksa ASCII kullanılır. Her karakter tam bir hücre olmalıdır: bir CJK karakteri ya da emoji iki hücredir ve dosya, satır ve sütun bilgisiyle reddedilir. Parantezler hiçbir modda kabul edilmez.
- **Animasyonlar nerede durur.** Gömülüler varsayılan ikon setinde, ikonların yanındadır. Bir temanın `[animations.<ad>]` tablosu tek bir animasyonu değiştirir; tıpkı `[icons]` tablosunun tek bir ikonu değiştirmesi gibi. Kendi ikon setin yenilerini ekleyebilir. Bozuk bir animasyon bildirilir ve atlanır; yerine geçmeye çalıştığı animasyon çalışmaya devam eder.
- **Eski temalar.** Hâlâ `spinner-arc = { nerd = "…", unicode = "…", ascii = "…" }` gibi bir spinner ikonu yazan tema çalışmaya devam eder: karakterler o animasyonun karelerinin yerine geçer, her yeni kare aynı konumdaki karenin rengini alır.
- **`$fg`**, animasyonu çizen bileşenin rengidir: spinner'ın tonu, butonun yazısı. Bitiş animasyonu `mix($success, $fg, N%)` ile karıştığı için spinner hangi renkteyse oradan başlar.
- **Hareket azaltılınca** duruş karesi sabit görünür: ilk kare, `once` için son kare. Bir `pulse()` bu durumda ikinci renginde durur; böylece duran nokta güçlü rengini korur.
- **Stüdyo hatırlar.** Değiştirdiğin ya da oluşturduğun animasyonlar showcase ayarlarında `studio.animations` altında saklanır. *Gömülüye dön* bir gömülüde yaptığın değişiklikleri atar.

## Sık yapılan hatalar

- **Geniş karakterler.** Emoji ve CJK karakterleri iki hücre kaplar; stüdyo bunu karenin altında söyler ve karakteri dışarıda bırakır.
- **Her yerde sabit renk.** `#hex` temayı yok sayar; animasyon her temaya uysun diye `$token` ve `mix()` tercih et.
- **Yalnızca Nerd Font.** Her zaman anlamlı bir ASCII karakteri ver; birçok terminalde Nerd Font yoktur.
- **Birbiriyle çatışan kare sayıları.** Tek bir kare listesi bütün modlara hizmet eder. ASCII dizisi daha kısaysa her mod kendi ritmini korusun diye onu tekrarla; gömülü Slices animasyonu bunu 40 kareyle yapar.
