## Ne zaman kullanılır

Metni, kullanıcının alıp götürmek isteyebileceği içerik olduğu yerde seçilebilir yap: bir log, bir mesaj, bir yol, bir kimlik, bir belge, kod. quvyta-framework ile yazılmış bir uygulamada varsayılan olarak hiçbir şey seçilemez; böylece menüler, başlıklar, butonlar ve boş alanlar yanlışlıkla vurgulanmaz. Seçimi bileşen bileşen açarsın ve seçim başladığı bileşenin içinde kalır.

## Adım adım

1. İçeriği işaretle: `ui.add_with(ScrollView::new(), |ui| { … }).selectable(true)`. `CodeView`, `Markdown` ve `Terminal` kendiliğinden seçilebilir; `Text` değildir, kopyalanmaya değer bir mesaj ya da adres için `ui.add(Text::new(adres)).selectable(true)` yaz.
2. Seçmek için üzerinde sürükle. İki kez basmak bir kelimeyi, üç kez basmak satırı alır.
3. Bırakınca seçim kalır; henüz bir şey kopyalanmaz. `ctrl c` kopyalar, seçime sağ tıklamak **Kopyala** ve **Ham kopyala** seçeneklerini açar.
4. Gizli bilgileri dışarıda tut: bir düğümde `.selectable(false)` seçilebilir bir alanın içinde de çalışır; örneğin bir logdaki anahtar için.
5. Kendi bileşeninde içerik için çizerken `cx.selectable(alan)`, içerik olmayan hücreler için `cx.decoration(alan)` çağır; örneğin kendin çizdiğin bir kaydırma çubuğu ya da satır numarası sütunu.
6. Kopyalara tepki vermek için `App::clipboard` yaz ve `ClipboardEvent::Copied(metin)` olayını işle.

## Nasıl çalışır

- **Yalnızca isteyen yer.** Basış yalnızca seçilebilir bir alanın içinde ve hiçbir bileşen onu kullanmadıysa seçim başlatır: butonlar, alanlar, listeler, sekmeler ve kaydırma çubukları eskisi gibi çalışır.
- **Bölge bileşenin kendisidir.** Seçim, basışın altındaki en içteki seçilebilir bileşende kalır; kaydırma alanında kaydırma çubuğu hariç içeriğe daralır. Kenarı geçen sürükleme kenarda durur. Bir pencerenin altındaki hiçbir şey seçilemez.
- **Kelimeler ve satırlar.** Kelime; harf, rakam ve `_ - . / : @ ~ # % + =` dizisidir; böylece yollar, adresler, özetler ve sürümler bütün olarak gelir.
- **Kaydırmayı izler.** İçerik kayınca vurgu metniyle birlikte hareket eder; bileşen kaybolursa seçim de kaybolur.
- **Kopyala temizdir.** Kopyala ve `ctrl c` yalnızca süs olanı almaz: `▌` çubuğunu ve başlık çubuğundan sonraki hücreyi, kaydırma çubuklarını, kod bloklarının satır numaralarını ve iç boşluğunu, satırı yalnızca kenara kadar dolduran boşlukları. Kelime aralarındaki boşluklar ve satır sonları kalır.
- **Ham kopyala birebirdir.** Seçili her hücreyi ekranda nasılsa öyle alır, dolgu ve süsler dahil; yerleşimin kendisi önemli olduğunda işe yarar.
- **Sessiz onay.** Kopyalamak vurguyu kısa bir an parlatır. Başka bir tuş ya da başka bir yere basış seçimi kaldırır; menü açıkken tuşlar menüye aittir.

## Sık yapılan hatalar

- **Bütün sayfayı seçilebilir yapmak.** Sayfayı değil içeriği işaretle: menüde ya da başlıkta başlayan bir seçim gürültüdür.
- **Bir basışı yok saymak için yakalamak.** Fare basışına `true` döndüren bileşen orada seçimi engeller; kullanmıyorsan `false` döndür.
- **Süsü metin olarak çizip işaretlememek.** `cx.decoration(alan)` çağır, yoksa temiz kopya onu da alır.
- **Bırakınca kopyalanmasını beklemek.** Kopyalanmaz. Bir ekran kopyalamaya dayanıyorsa bir `CopyValue` ya da `ctrl c` ve sağ tık hakkında bir ipucu yardımcı olur.
