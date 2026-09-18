## Önce klavye

quvyta-framework ile yazılmış bir uygulamada her şey klavyeyle çalışır. Fare aynı eylemlere bir kısayoldur, asla tek yol değildir.

## Odak

- Girdi alan bileşenler odaklanabilir: butonlar, alanlar, açılır listeler, listeler, sekmeler, kaydırma alanları.
- **Tab** bir sonrakine, **shift tab** bir öncekine geçer; sıra ekranda göründükleri sıradır.
- Bir bileşene tıklamak ona odaklanır.
- Tuşlar önce odaklı bileşene gider. Onun kullanmadığı tuşlar ebeveynlerine, sonra kısayol haritasına ilerler.
- Açık bir açılır liste kapanana kadar tuşları tutar; oklar içinde gezinir, Esc kapatır.
- `Command::focus("isim")`, `.id("isim")` ile adlandırılmış bileşene odaklanır; örneğin bir formun ilk alanına geri dönmek için.

Odağı tema gösterir: odaklı bileşen, çubuğuyla ya da kendi renkleriyle iki vurgu tonu arasında nefes alır; göz onu hemen bulur. Butonlar, kartlar, sekmeler ve basılabilen diğer kontroller yalnızca odak klavyeyle geldiğinde nefes alır; az önce tıkladığın kontrol farenin altında sakin kalır.

## Kısayol haritası

Kısayol haritası **eylem adlarını** tuşlara bağlar. Framework eylemleri `[global]` içinde, uygulamanın eylemleri `[app]` içinde durur:

```toml
[app]
save = "ctrl+s"
search = ["/", "ctrl+f"]
```

`App::action` bir adı mesaja çevirir. Kullanıcılar tuşları bir dosyayla yeniden bağlayabilir; kodda hiçbir şey değişmez. Aynı tuş bir tablonun iki eylemine bağlanırsa bir uyarı bunu söyler.

Uygulamanın kendi bağlamalarının diskte bir dosya olması şart değil. `Runtime::keymap_source(dosya, metin)` TOML metninin kendisini alır; bu metin genelde deponuzdaki kısayol dosyasının `include_str!`'ıdır, böylece kurulan ikili tuşlarını kendi taşır. `CARGO_MANIFEST_DIR`'den kurulan bir yol, ikili başka bir yere kurulunca kırılır. Ayrıca verilen bir `keymap_file` artık zorunlu değildir: okunamadığında metin onun yerine geçer ve sebep, programı durdurmak yerine bir tanılamaya dönüşür. Bozuk bir satır dosya, satır ve sütunuyla bildirilip atlanır; gömülü bağlamalar çalışmaya devam eder.

## İpucu etiketleri dilden gelir

İpucu çubuğu etiketleri dil anahtarlarından okur: global eylemler için `quvyta.keys.<eylem>`, seninkiler için `keys.<eylem>`. Dili değiştirince her ipucu da değişir.

## Basılı tutulan tuşlar

Enter ya da Boşluk'u basılı tutmak bir etkinleştirmeyi asla tekrarlamaz. Tuş bırakmayı bildiremeyen terminaller basılı tuşu hızlı basışlar olarak gönderir; quvyta-framework 100 ms'den yakın basışları tek sayar.

## Hata ayıklama katmanı

Herhangi bir uygulamada **f12**'ye bas. Tıklanabilir alanlar renklenir, her odaklanabilir bileşen odak sırasındaki yerini gösterir, bir panel kare numarasını, çizim süresini, tıklanabilir alan sayısını ve odaklı bileşeni gösterir. Gizlemek için tekrar f12'ye bas.

## Sık yapılan hatalar

- **Yalnızca fareyle yapılabilen eylemler.** Her tıklama hedefinin bir tuş yolu olmalı: odaklanabilir bileşen ya da kısayol eylemi.
- **Bileşenlere gömülü tuşlar.** Kullanıcılar değiştirebilsin diye uygulama kısayollarını kısayol haritasına bağla.
- **Tab'ı almak.** Bileşen Tab'a ihtiyaç duyan bir metin editörü değilse Tab'ı odak gezintisine bırak.
