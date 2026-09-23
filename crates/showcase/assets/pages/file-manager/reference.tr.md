## Yöntemler

- `FileManager::new(&durum, sar)` — `sar` bir `FileManagerMsg`'i senin mesajına çevirir: `Msg::Files` gibi bir fonksiyon ya da gerekeni yakalayan bir kapanış. Herhangi bir `Fn(FileManagerMsg) -> Msg + 'static`.
- `.root_label(metin)` — en üstteki satırın yazısı; varsayılanı kök klasörün kendi adı.
- `.on_open(|yol| Msg)` — bir dosyaya tık ya da Enter. Bu olmadan dosyaya tık yalnızca seçer.
- `.on_open_terminal(|yol| Msg)` — klasör menüsüne "Burada terminal aç" ekler, o klasörün yoluyla.
- `.menu_items(|key, targets| Vec<ContextItem<Msg>>)` — kendi öğelerin, kendi grubunda; `targets` o satırdaki bir işlemin neye işlediğidir.
- `.row_mark(|key| RowMark)` — uygulamanın o satırın görünüşü hakkında söyledikleri; söyleyecek bir şeyi olmayan satır için `RowMark::new()`.
- `.view(FileView::Tree | List | Icons)` — klasörün çizildiği biçim; varsayılanı ağaç.
- `.disabled(bool)` — her satır solgun; tık, tuş, sürükleme ve menü yok.
- `.show(ui)` — yöneticiyi ekler ve ağacın düğümünü döner; `.fill()` ve `.id(ad)` için. Ad soran diyalog açıkken o da eklenir; bir katmandır ve yer tutmaz.
- `FileManagerState::new(kök)`, `.confined()`, `.following(bool)`, `.trashing()`, `.trashing_in(klasör)`, `.showing_hidden(bool)`; `set_following(bool)`, `set_showing_hidden(bool)`, `is_confined()`, `follows_changes()`, `is_trashing()`, `shows_hidden()`.
- `.load(sar)` — ilk seferinde kökü, sonrasında açık her klasörü yeniden okur. `.update(mesaj, sar)` — mesajı uygular ve istediği işi döner. `sar` arka plandaki iş parçacıklarına taşınır: herhangi bir `Fn(FileManagerMsg) -> Msg + Send + Sync + 'static`.
- Durumu okumak: `root()`, `path(key)`, `children(key)`, `work()`, `shown_children(key)`, `copied()`, `pending()`, `is_copying()`, `is_open(key)`, `is_loading(key)`, `is_folder(key)`, `folder_keys()`, `visible_folders()`, `selected()`, `chosen()`, `targets(key)`, `cut()`, `is_cut(key)`, `error()`, `naming()`, `naming_problem()`, `select(key)`.
- `FileManagerState::ROOT` — kökün anahtarı, boş dizge.
- `FolderEntry { name, folder }` ve `FolderEntry::read_folder(yol) -> Result<Vec<FolderEntry>, String>`; bir klasörü kendi yolundan okuyan uygulama için.
- `FileManagerMsg::Select | Choose | Expand | Read | NewFile | NewFolder | Rename | Cut | Paste | DropCut | Drop | Delete | DeleteConfirmed | Refresh | Name | Submit | CloseNaming | Done | Changed | Detail | Detailed | Enter | Leave`.
- Ayrıntılar: `details(key) -> Option<Option<&FileDetails>>` (hiç istenmedi / istendi ve orada bir şey yok), `has_details(key)`, tam bir aralık için `detail(anahtarlar, sar)`, imlecin çevresindeki sayfa için `detail_page(klasör, sar)`, sayfada hâlâ eksik olanlar için `detail_gaps(klasör)`. `FileDetails { size, modified, mode, readonly }`; `size_text(klasör_mü)`, `modified_text()`, `permissions_text(klasör_mü)` ve `FileDetails::read(yol)`.
- Düz görünümler: `folder()` — liste ve simgelerin gösterdiği klasör; `FileManagerMsg::Enter(key)` bir klasöre girer, `FileManagerMsg::Leave` ondan çıkar.
- `FileChange::Created(key) | Moved(from, to) | Deleted(key)`; `FileError::Name | Outside | IntoItself | Taken(ad) | CrossDevice | Denied | System(metin)`, her birinde `.message()`.
- `FileChange::Copied(key) | Trashed(key)` ve `FileError::NotReadable | Missing | NoTrash | NoRoom | Stopped` ikinci adımla geldi; hepsi `.message()` ile konuşur.
- `copy_into(kök, key, into, confined)` — bir yolu kendi yöntemiyle kopyalayan uygulama için.
- `FileWork` — şu an süren uzun işlem: `id()` (kendi `Tasks` modelinde göstermek ya da kendin durdurmak için `TaskId`), `done()`, `note()`, `entries()`.
- `FolderEntry::is_hidden()` — adı noktayla başlayan girdi.
- `RowMark::new()`, `.sign(ikon, ton)` (ikisi her zaman birlikte), `.faint(bool)`; okumak için: `icon()`, `tone()`, `is_faint()`, `is_empty()`.
- `NameProblem::Empty | Slash | Nul | Dots | Taken` ve `.message()`; `Naming { purpose, folder, value, tried }`; `NameFor::File | Folder | Rename(key)`.
- Kimlik olarak anahtarlar: `child_key(üst, ad)`, `parent_key(key)`, `name_of(key)`, `is_within(key, klasör)`, `is_inside(key)`.

## Davranış

- Listenin dört sütunu vardır — ad, boyut, değişti, izinler — ve sığmadıklarında yana kayarlar. İki düz görünüm de her satırın yanına çoklu seçim için bir işaret koyar; altlarındaki satır klasörün kaç öğe tuttuğunu söyler ya da okuma 300 ms'yi aşarsa döner.

- Satırlar: önce kök, sonra klasörler ve dosyalar, her grup ad sırasında. Adı platformun metin olarak yazamadığı girdi dışarıda bırakılmaz, kayıplı gösterilir.
- Klasör açıldığında bir kez okunur; tekrar açmak bilineni gösterir. Okunurken kapatılırsa cevap atılır, böylece aynı adda yeni bir klasör kapalı ve okunmamış başlar.
- `up` `down` `home` `end` imleci taşır, `left` `right` ve `enter` klasörü açıp kapatır, dosyada `enter` onu açar, `space` ve `ctrl`+tık çoklu seçer, `shift+f10` satırın menüsünü açar. Bir klasörün üstüne sürüklenen satırlar oraya taşınır; satırların altındaki boş alana bırakılırsa köke gider.
- Sağ tık, seçili satırlardan birinin üstündeyse seçimi korur, değilse satırı seçim yapar; böylece menü tıklanan şeye işler.
- Klasör menüsü: Yeni dosya, Yeni klasör, sonra Yeniden adlandır ve Kes (kökte yok), bir şey kesilmişse Buraya yapıştır ve Taşımaktan vazgeç, sonra senin öğelerin, sonra kökte Yenile ya da altında Sil. Dosya menüsü yalnızca klasörün yapabildiklerini içermez. Birkaç seçili satırdan biri hepsi için Kes ve Sil sunar, Yeniden adlandır sunmaz: ad tek girdiye verilir.
- Kesilen klasörün kendisine ya da içindeki bir klasöre yapıştırmak görünür ama seçilemez.
- Yeniden adlandırma, uzantıdan önceki kısım seçili açılır; böylece yazmak dosyanın türünü korur. Klasör ve baştan noktalı dosya bütünüyle seçilir.
- Ad yazıldıkça denetlenir: boş (yalnızca ilk denemeden sonra), `/`, NUL, `.` ve `..`, ve klasörde zaten olan bir ad — girdinin kendi adı bu sayılmaz.
- Silme önce sorar, tehlike renginde, ve klasörün içindeki her şeyi götürdüğünü söyler. Birkaç girdi bir kez sorulur; beşe kadar adla, sonrası sayıyla.
- İşlemler verilen sırada, her biri kendi başına çalışır. Ret girdiyi ve sebebini söyler; yapılabilen yapılır. Değişiklikten sonra dokunulan klasörler yeniden okunur ve imleç oluşana ya da taşınana gider.
- `confined()` kökten çıkan anahtarı ve yolun üzerindeki sembolik bağ olan klasörü reddeder. Onsuz da o parçalar reddedilir; yalnızca bağ izlenir.
- Taşıma bir yeniden adlandırmadır: başka dosya sistemindeki hedef reddedilir ve söylenir, hiçbir şey kopyalanmaz, zaten orada olan hiçbir şeyin üstüne yazılmaz.
- İşaret satırın klasör ya da dosya ikonunun yerine kendi işaretini, kendi tonunda koyar ve satırı solgun çizebilir. Satırı asla daha gür yapamaz: kesilmiş bir girdi ve pasif bir yönetici, işaret ne derse desin solgun kalır. Ton işaretsiz verilemez, böylece işaretli satır on altı renk ve ASCII kipinde de ayırt edilir.
- Kopyalama tutulan bir iş parçacığı değil bir `Task` olarak çalışır: nereye geldiğini söyler, satırların üstündeki bir satır işi adlandırır, ilerleme çubuğunu çizer ve Durdur sunar. Durdurmak yarı yazılmış girdiyi geri alır ve klasörleri yeniden okur, çünkü kopyalanmış olan diskte kalır. Aynı anda tek kopyalama çalışır; sürerken istenen ikincisi hiçbir şey yapmaz.
- Kopyalama taşımanın yanındaki işlemdir: `Copy` girdileri `Cut` gibi kenara koyar, `Paste` onları taşımak yerine kopyalar; klasör içindeki her şeyle. Kopyalanan yerinde kalır, bu yüzden onda solgun bir şey yoktur. Bağ, bağ olarak kopyalanır. Hedef klasörde aynı ad varsa reddedilir; hiçbir şeyin üstüne yazılmaz.
- `trashing()` ve `trashing_in(klasör)` silmeyi çöpe atmaya çevirir: menülerde Sil'in yerini alır ve hiçbir şey sorulmaz, çünkü çöpe yeniden bakılabilir. Çöp, freedesktop belirtiminin dediği gibi yazılır: girdi `files/` altına, nereden geldiğini ve ne zaman gittiğini söyleyen `info/<ad>.trashinfo` notu ise önce ve `create_new` ile yazılır, böylece adı o ayırır; aynı ad varsa sıradaki numara alınır. Çöpün alamadığı girdi — başka bir dosya sisteminde ya da hiç çöp yok — sessizce silinmez: yönetici kalıcı silmeyi kendi sorusuyla, tehlike renginde sorar.
- `showing_hidden(bool)` adı noktayla başlayan girdileri gösterir. Onlar her hâlükârda okunur, bu yüzden açmak diske hiç gitmez; yeni bir ad, gizli girdi gösteriliyor olsun olmasın ona karşı da denetlenir.
- Reddetme sistemin değil kişinin diliyle söylenir: içine bakılamayan klasör, artık orada olmayan şey, dolu disk ve durdurulmuş kopyalama, her birinin kendi cümlesi var.
- `following(true)` ekrandaki klasörleri izler. Oluşma, silinme ve ad değişimi o klasörü yeniden okutur; içerik değişimi hiçbir şey okutmaz; kaybolan klasör ya da taşma ekrandaki her şeyi okutur. Bırakılmış bir izlemenin partisi göz ardı edilir.
- `.following_within(süre)` `following(true)` gibi izler, her bekleyiş en fazla `süre` sürer ve sonra yeniden bekler (`FileManagerMsg::Quiet`); böylece bir ekran testi başka bir programın değişikliğinin gelişini harness'i adımlatarak görür.

## Tema anahtarları

- Yönetici ağacın biçimlerini kullanır: `list-item` (`faint` ile), `tree-chevron`, `tree-drop`, `list-detail`, `spinner` ve kaydırma çubuğunun biçimleri.
- Menü ve diyalog `context-menu`, `context-item` (`danger`, `disabled` ile), `modal`, `modal-title`, `text-input`, `field-error`, `button.primary` kullanır.
- İkonlar: `folder`, `file`, `tree-expanded`, `tree-collapsed`.
- Metinler: `quvyta.file-manager.*`.
