## Ne zaman kullanılır

Başka şeyleri içeren ve seviye seviye gezilen şeyler için ağaç kullan: klasörler ve dosyalar, servisler ve container'ları, bir belgenin ana hatları. Her şey tek seviyedeyse liste yeterlidir.

## Adım adım

1. Düğümleri kalıcı bir anahtar ve etiketle kur: `TreeNode::new("src/lib.rs", "lib.rs")`. Anahtar düğümü her mesajda adlandırır; konum değil, yol ya da kimlik kullan.
2. Klasörlere `.children([...])` ile çocuklarını ver, hangilerinin açık olduğunu `.expanded(self.open.contains(anahtar))` ile söyle. Durumunda açık anahtarların bir kümesini tutmak en kolay yoldur.
3. Seçimi `.selected(self.selected.as_deref())` ile göster, `.on_select(|anahtar| ..)` ile al.
4. Açma ve kapamayı `.on_expand(|anahtar, açık| ..)` ile al: anahtarı açık kümesine ekle ya da çıkar.
5. Yapraklar `.on_activate(|anahtar| ..)` ile açılır.
6. Getirilmesi gereken çocuklar için düğümü `.expandable(true)` yap; açılınca onları okuyan bir `Command::perform` döndür ve cevap gelene kadar `.loading(true)` göster.
7. Düğümleri kişinin kendi sırasına koymak için `.reorderable(|adım| Msg::Move(adım))` ekle ve `TreeMove`'u kardeş listene `adım.apply(&mut kardeşler)` ile uygula.
8. Bir düğüm üzerindeki eylemler için — yeniden adlandır, arşivle, başka yere taşı — `.context_menu(|anahtar| vec![ContextItem::new(..), ..])` ekle.

## Nasıl çalışır

- **Girinti boşluktur.** Her seviye iki hücre sağa kayar; kılavuz çizgisi çizilmez. Klasörlerde açıkken aşağı bakan küçük bir ok vardır.
- **Tuşlar şekli izler.** → klasörü açar ya da içine girer, ← kapatır ya da üst klasöre çıkar, Enter klasörleri açıp kapatır ve dosyaları açar.
- **Ok kendi başına bir hedeftir.** Oka tıklamak yalnızca açar ya da kapatır; ada tıklamak ayrıca seçer.
- **Yalnızca ikon ve ad kayar.** Hover edilen ya da seçili satır yükselir, ikonunu ve adını bir hücre sağa kaydırır. Girinti, ok ve detay yerinde kalır; ok fareden hiç kaçmaz.
- **Yalnızca açık kısım vardır.** Ağaç yalnızca açık düğümleri düzleştirir ve yalnızca ekrandaki satırları çizer; elli bin dosyalı bir klasör de akıcı kayar.
- **Sıralama kardeşler arasında kalır.** Sürüklenen düğüm yalnızca kardeşleri arasında yer değiştirir: o geçerken kardeşler yer açar, imleci bir hayalet satır izler ve sürüklenen düğümün kendi çocukları bırakılana kadar katlanır. Ctrl+Shift+↑/↓ klavyeden aynısını yapar. Üst ya da alt satırda tutulan sürükleme ağacı kaydırır. Sıralama açıkken tıklama tuş bırakılınca işler; sürüklemek için basılan klasör açılmaz.
- **Menü bir düğüme aittir.** Sağ tık imlecin altındaki satırın menüsünü açar ve o satırı yükseltilmiş tutar; menü tuşu ya da Shift+F10 seçili düğümün menüsünü onun altında açar.
- **Yüklenmenin de bir yüzü var.** Çocukları 300 ms'den uzun süren klasörün oku bir spinner'a dönüşür ve yanıp sönmesin diye en az 500 ms kalır; hızlı yüklemede hiç görünmez; okunamayan klasör hatasını tehlike işaretli silik bir çocuk olarak gösterebilir.

## Sık yapılan hatalar

- **Satır konumunu anahtar yapmak.** Klasörler açıldıkça konumlar değişir; anahtarlar değişmemeli.
- **Diski `view` içinde okumak.** Klasörleri bir `Command::perform` içinde oku, sonucu sakla.
- **Düğümü sürükleyerek başka bir ebeveynin altına taşımak.** Sürükleme bir düğümün ebeveynini asla değiştirmez: bir klasörün içine bırakmak kazayla kolayca olur, geri almak zordur. Bunun yerine bağlam menüsünde "Şuraya taşı…" sun.
- **Kapatmayı unutmak.** `açık = false` durumunu da işle, yoksa klasörler bir daha kapanmaz.
