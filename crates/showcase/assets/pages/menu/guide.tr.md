## Ne zaman kullanılır

Menüyü uygulamanın her zaman orada duran yerleri için kullan: bir konsolun yan menüsü, bir ayarlar ekranının bölümleri. Veri satırları için `List`, kapatılabilen açık şeyler için `Tabs` ya da `TabRail` daha uygundur.

## Adım adım

1. Öğeleri uygulamanın tanıdığı bir anahtarla kur: `MenuItem::new("deploys", t!("menu.deploys"))`.
2. İşe yaradığı yerde ikon ve rozet ekle: `.icon("success", Some("success")).badge("3")`.
3. Öğeleri başlıklı gruplara koy: `MenuGroup::new("workspace", items).title(t!("menu.workspace"))`.
4. Menüyü açık sayfayla göster: `Menu::new(groups).selected(Some(&self.page)).on_select(|key| Msg::Go(key.to_owned()))`.
5. Bir `AppShell` yan menüsüne yerleştir: `.sidebar(|ui| { ui.add(menu).fill(); })`.
6. Grupların katlanabilmesi için `.collapsible(|group, open| Msg::Group(..))` ekle, kapalı grupları `.collapsed(..)` ile ver.

## Nasıl çalışır

- **Buton değil satır.** Başlıklar silik yazıdır. Açık öğe vurgu çubuğuyla yükselir; menü odaktayken çubuk nefes alır. Üstüne gelinen öğe hafifçe yükselir. İkon ve yazı bir hücre kayar; rozetler sağda sabit kalır.
- **Klavye bir imleç gezdirir.** ↑ ve ↓ hiçbir şey açmadan vurguyu taşır; menüde gezinirken yoldaki her sayfa yüklenmez. Bir harf yazmak o harfle başlayan sonraki öğeye atlar. Enter ya da Boşluk açar; tıklama hemen açar. Her zaman tek bir vurgu vardır: fareyi bir satıra götürmek imleci oraya taşır, tuşlar oradan devam eder.
- **Katlama bir seçenektir.** `collapsible` yoksa başlıklar sade yazıdır. Varsa başlıklı grupların başlığında bir ok belirir ve klavyeyle başlığa gelinebilir; Enter ya da tıklama açıp kapatır, → açar, ← kapatır, bir öğede ← grubun başlığına gider. Katlanabilen başlık bir öğe gibi yükselir ve kayar; oku sağda sabit kalır.
- **Uzun menüler kayar.** Tekerlek kaydırır, imleç ve açık öğe görünür kalır, gerektiğinde kaydırma çubuğu çıkar.
- **List ile aynı satırlar.** Menu, List, Tree, Accordion ve TabRail satırlarını ortak bir çizimle çizer; hover, seçim ve kayma her yerde aynıdır: önce sabit işaretler, sonra kayan ikon ve etiket, en sağda sabit kısım.

## Sık yapılan hatalar

- **Her ok tuşunda sayfa açmak.** Menü `on_select` mesajını yalnızca Enter, Boşluk ya da tıklamayla gönderir; sayfaları başka bir şeyle açma.
- **İşaretsiz durum rengi.** Yazıyı değil, ikonu renklendir.
- **Başlıksız grubu katlamak.** Katlanabilmesi için grubun başlığı olmalıdır.
