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
9. Birden çok düğüm seçtirmek için seçili anahtarları bir `Vec<String>` içinde tut ve `.multi_select(&self.chosen, |anahtarlar| Msg::Choose(anahtarlar))` ekle; her mesaj seçimin yeni halinin tamamıdır, geldiği gibi sakla. `.selected(..)` imleç olarak kalır.
10. Düğümleri sürükleyerek klasörlere taşıtmak için `.droppable(|bırakma| Msg::Drop(bırakma), |anahtar| klasör_mü(anahtar))` ekle. `TreeDrop`, taşınan `keys` anahtarlarını ve girdikleri `into` klasörünü söyler (en üst seviye için `None`); o düğümleri verinden çıkar ve klasöre koy.

## Nasıl çalışır

- **Girinti boşluktur.** Her seviye iki hücre sağa kayar; kılavuz çizgisi çizilmez. Klasörlerde açıkken aşağı bakan küçük bir ok vardır.
- **Tuşlar şekli izler.** → klasörü açar ya da içine girer, ← kapatır ya da üst klasöre çıkar, Enter klasörleri açıp kapatır ve dosyaları açar.
- **Ok kendi başına bir hedeftir.** Oka tıklamak yalnızca açar ya da kapatır; ada tıklamak ayrıca seçer.
- **Yalnızca ikon ve ad kayar.** Hover edilen ya da seçili satır yükselir, ikonunu ve adını bir hücre sağa kaydırır. Girinti, ok ve detay yerinde kalır; ok fareden hiç kaçmaz.
- **Yalnızca açık kısım vardır.** Ağaç yalnızca açık düğümleri düzleştirir ve yalnızca ekrandaki satırları çizer; elli bin dosyalı bir klasör de akıcı kayar.
- **Sıralama kardeşler arasında kalır.** Sürüklenen düğüm yalnızca kardeşleri arasında yer değiştirir: o geçerken kardeşler yer açar, imleci bir hayalet satır izler ve sürüklenen düğümün kendi çocukları bırakılana kadar katlanır. Ctrl+Shift+↑/↓ klavyeden aynısını yapar. Üst ya da alt satırda tutulan sürükleme ağacı kaydırır. Sıralama açıkken tıklama tuş bırakılınca işler; sürüklemek için basılan klasör açılmaz.
- **Menü bir düğüme aittir.** Sağ tık imlecin altındaki satırın menüsünü açar ve o satırı yükseltilmiş tutar; menü tuşu ya da Shift+F10 seçili düğümün menüsünü onun altında açar. Birden çok düğüm seçiliyken bunlardan birine sağ tık hepsini seçili tutar, menü seçimin tamamı için açılır; başka bir satıra sağ tık önce seçimi o satıra indirir.
- **Tek imleç, çok seçim.** Çoklu seçimde Ctrl+tık bir satırı ekler ya da çıkarır, Shift+tık ve Shift+↑/↓ bir aralık seçer, Space imlecin satırını ekler ya da çıkarır, Esc teke döner. Seçili satırların hepsi seçim tonunu alır; çubuğu ise yalnızca imlecin satırı taşır ve yalnızca o kayar, böylece tuşların nereden yürüyeceği hep bellidir.
- **Sürükleme seçimi taşır.** Seçili bir satırdan başlayan sürükleme bütün seçili düğümleri birlikte götürür. İmlecin altındaki klasör vurgu rengini alır; sürüklenen düğümlerin kendisi, içlerindeki her şey ve zaten içinde durdukları klasör soluk kalır ve kabul etmez. Kapalı bir klasörün üzerinde 400 ms beklemek onu açar, böylece bırakma daha derine ulaşır; son satırın altındaki boşluk en üst seviyedir. Sıralama da açıksa klasör satırı düğümü içine alır, diğer satırlar kardeşler arasında bir yerdir.
- **Yüklenmenin de bir yüzü var.** Çocukları 300 ms'den uzun süren klasörün oku bir spinner'a dönüşür ve yanıp sönmesin diye en az 500 ms kalır; hızlı yüklemede hiç görünmez; okunamayan klasör hatasını tehlike işaretli silik bir çocuk olarak gösterebilir.

## Sık yapılan hatalar

- **Satır konumunu anahtar yapmak.** Klasörler açıldıkça konumlar değişir; anahtarlar değişmemeli.
- **Diski `view` içinde okumak.** Klasörleri bir `Command::perform` içinde oku, sonucu sakla.
- **Sıralamanın ebeveyni değiştirmesini beklemek.** Tek başına `reorderable` bir düğümü asla başka bir ebeveynin altına taşımaz: klasöre bırakmak kazayla kolayca olur. Bunu `droppable` ile bilerek aç, klavye için de bağlam menüsünde "Şuraya taşı…" sun.
- **Seçimleri kendin birleştirmek.** Ağaç her zaman seçimin yeni halinin tamamını gönderir; kendi seçimine eklemek yerine onunla değiştir.
- **Klasörün çocuklarını da ayrıca taşımak.** `TreeDrop`, klasörüyle birlikte taşınan düğümleri listeye koymaz; yalnızca adı geçen anahtarları taşı.
- **Kapatmayı unutmak.** `açık = false` durumunu da işle, yoksa klasörler bir daha kapanmaz.
