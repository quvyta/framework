## Metotlar

- `FilePicker::new(&tarayıcı, sar)` — `sar` bir `FilePickerMsg`'yi senin mesajına çevirir: `Msg::Picker` gibi bir fonksiyon ya da ihtiyacını yakalayan bir kapanış, örneğin bir ekranın kendi dönüşümü (`move |m| Msg::Tab(sıra, m)`). Herhangi bir `Fn(FilePickerMsg) -> Msg + 'static`.
- `.show(ui)` — seçiciyi bir sütun olarak ekler ve boyutlandırmak için düğümünü döndürür.
- `.open_on(Click)` — varsayılan `Click::Double`: tık seçer, çift tık klasörü açar ya da dosyayı seçer. `Click::Single`: tek tık yapar.
- `FileBrowser::new(klasör, PickMode)`, `.extensions(liste)`.
- `.open(klasör, sar)` ve `.update(mesaj, sar)` — ikisi de klasörleri okuyan komutu döndürür. `sar` klasörü okuyan iş parçacığında bir kez çalışır: herhangi bir `FnOnce(FilePickerMsg) -> Msg + Send + 'static`, seçicinin aldığı aynı fonksiyon ya da kapanış.
- `.folder()` (gösterilen klasör), `.loading()` (okunmakta olan klasör, varsa), `.shows_hidden()`, `.selected_path()`.
- `FilePickerMsg::Open(yol) | Loaded(yol, liste) | Select(Option<ad>) | Filter(metin) | ShowHidden(bool) | Refresh | Chosen(yol)`.
- `PickMode::Files | Folders`; `read_folder(yol) -> Listing`; `FileEntry` (`name`, `is_folder`, `size`, `is_hidden`); `ListingError::PermissionDenied | NotFound | NotAFolder | Other(metin)`.

## Davranış

- Liste, List'in tuşlarını izler: `up` `down`, `home` `end`, `pgup` `pgdn`; `enter` ya da çift tık (`Click::INTERVAL`, 400 ms içinde aynı girdiye iki basış) bir klasörü ya da üst klasör satırını açar, dosya modunda dosyayı seçer; tek tık yalnızca seçer. `.open_on(Click::Single)` ile tık açar ve seçer. Klasör modunda imlecin kendiliğinden durduğu girdiye tıklamak onu göstermek sayılır.
- `tab` süzgeç, liste, gizli dosyalar anahtarı ve seçme düğmesi arasında gezer.
- Yoldaki bir parçaya tıklamak o klasörü açar; açık klasörün parçası bir düğme değildir.
- Bir klasör okunurken gösterilen klasörün yolu, süzgeci, listesi, seçimi ve odağı olduğu gibi kalır ve çalışmaya devam eder; yeni klasör cevabı gelince tek karede onların yerine geçer. Bu sırada başka bir klasör açmak önceki cevabı geçersiz kılar.
- 300 ms'den uzun süren bir okuma yolun hemen ardında bir spinner gösterir (satırda yer varsa "Klasör okunuyor…" yazısıyla); göründükten sonra en az 500 ms kalır. Bir hata onu hemen gizler.
- Seçme düğmesi ilk klasör okunmadan önce, bir hatadan sonra ve dosya modunda bir dosya seçilene kadar pasiftir.
- Uzantılar büyük-küçük harfe bakmadan karşılaştırılır; klasörler her zaman görünür.

## Tema anahtarları

- `path-segment` — `fg`, `bg`; `hover` ve `selected` (açık klasör) ile; `path-separator` — `fg`.
- Seçici `list-item`, `text-input`, `switch-labeled`, `button.primary`, `spinner` ve `spinner-label` stillerini kullanır.
- İkonlar: `folder`, `file`, `arrow-up`, `path-separator`, `error`.
- Metinler: `quvyta.file-picker.*`.
