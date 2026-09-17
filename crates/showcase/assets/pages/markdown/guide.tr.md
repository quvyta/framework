## Ne zaman kullanılır

Kod yerine birinin yazdığı metinler için Markdown kullan: rehberler, yardım ekranları, sürüm notları, dosyadan yüklenen açıklamalar. Bu showcase'teki her rehber ve referans Markdown'dır.

## Adım adım

1. Belgeyi yükle ya da göm: `include_str!("help.md")` ya da bir dosyadan metin.
2. Ekle: `ui.add(Markdown::new(metin)).fill_width()`.
3. Ekrandan uzun olabiliyorsa bir `ScrollView` içine koy.

## Neler desteklenir

- Başlıklar: birinci ve ikinci seviye vurgu çubuğunu taşır, üçüncü seviye daha sessizdir.
- Kelimelerde sarılan, **kalın**, *vurgulu*, `satır içi kod` ve bağlantı içeren paragraflar. Bir parçanın hemen ardındaki noktalama onunla kalır: satır içi koddan sonraki `.` ya da `,` satıra hiçbir zaman tek başına başlamaz; satırdan geniş bir kod parçası bölünmek zorunda kaldığında bile.
- Madde işaretli ve numaralı listeler, iç içe listeler.
- Silik bir çubukla çizilen alıntılar.
- `rust` ve `toml` için renklendirilmiş, satır numaralı kod blokları.

Yatay çizgi boş alana dönüşür. Markdown asla karakterlerden çizgi çizmez.

## Nasıl çalışır

Belge, bileşen oluşturulurken bir kez ayrıştırılır. Çizim blokları mevcut genişliğe göre yerleştirir, görünür alan dışındaki blokları atlar ve her parçayı temadan boyar; belge tema değişikliklerini diğer bileşenler gibi takip eder.

- **Kendiliğinden seçilebilir.** Sürüklemek belgenin içinde seçer. Kopyala başlık ve alıntı çubuklarını ve ardındaki hücreyi, kod bloklarının iç boşluğunu ve satır numaralarını almaz; Ham kopyala her şeyi alır. Düğümde `.selectable(false)` seçimi kapatır.

## Sık yapılan hatalar

- **Geniş tablolar.** Tablolar çizilmez; `isim — açıklama` listeleri kullan.
- **Çok uzun kod satırları.** Girintiyle sarılırlar; örnekleri dar tut.
