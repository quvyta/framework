## Ne zaman kullanılır

Ekranda başka programların değiştirdiği bir klasör gösteriyorsanız klasör izlemeyi kullanın: bir derleme içine yazarken dosya ağacı, başka bir sürecin kaydettiği kayıtların listesi, kullanıcının elle düzenlediği bir ayar klasörü. Bir girdinin belirdiği ya da kaybolduğu anı işletim sistemi zaten bilir; izleme bunu söylemesini ister ve hiçbir şey olmazken hiçbir şeye mal olmaz.

- Bütün ağacı değil, kullanıcının açık gördüğü klasörleri **izleyin**. İzleme bilerek özyinelemeli değildir: bir ağaç yalnızca gösterdiğini bilmek zorundadır.
- **Yoklama yapmayın.** Klasörü her saniye okumak her tıkta iş çıkarır ve değişikliği yine geç gösterir.
- **Kendi yeniden okumalarınızı koruyun.** Uygulamanız bir klasörü kendisi değiştirdiyse onu hemen yeniden okuyun; izleme bunu bir an sonra doğrular, bunun zararı yoktur.

## Adım adım

1. Açılışta bir izleme oluşturup durumunuzda tutun: `let watch = FolderWatch::new()?;`. İzlemesi olmayan bir platformda bu bir `Unsupported` hatasıdır: o zaman seçtiğiniz anlarda yeniden okumaya devam edin.
2. Her klasörü açıldığında izleyin: `watch.watch(&folder)?`; kapandığında `watch.unwatch(&folder)`. Hata, yalnızca o klasörün canlı değişiklik almayacağı anlamına gelir; diğerleri alır.
3. Arka planda bekleyin: `changes = watch.changes()` ile `Command::perform(move || Msg::Changed(changes.next()))`.
4. `update` içinde toplulukta adı geçen her klasörü yeniden okuyun, sonra yeni bir `watch.changes()` ile yeniden bekleyin.
5. Boş bir topluluk izlemenin bırakıldığını söyler: beklemeyi bırakın.

## Nasıl çalışır

- **İş parçacığı çekirdekte uyur.** `next()` sistemin olay kuyruğunda bekler, yalnızca bir şey değişince ya da izleme bırakılınca uyanır. Boştaki bir izleme işlemci harcamaz.
- **Değişiklikler topluluk halinde gelir.** İlk olaydan sonra `next()` saniyenin onda biri kadar toplamayı sürdürür, sonra hepsini birden, en eskisi önde ve her değişikliği bir kez döndürür. Binlerce dosyalık bir `git checkout` birkaç topluluktur; ağacınız da birkaç kez kurulur.
- **Ekran testinin çalıştırabileceği bekleyiş.** Ekran testi arka plan işini olduğu yerde yapar; açtığı sayfa sonsuza dek beklememeli. Bu sayfa `changes.next_within(süre)` ile bekler, süre içinde bir şey değişmediyse yeniden bekler: ekranda sınanan bir uygulama da böyle yapar, hiç sınanmayan `next()` ile kalabilir.
- **Değişiklik girdisini adlandırır.** `FolderChange { folder, name, kind }`: `Created`, `Removed`, `Modified`, ya da bir klasör içindeki yeniden adlandırma için `Renamed { from }`. İzlenen bir klasörden ötekine taşıma, birinde silinme, ötekinde oluşmadır.
- **Taşma "yeniden oku" demektir.** Değişiklikler okunduğundan hızlı gelirse sistem bir kısmını atar ve bunu söyler. O zaman izlenen her klasöre bir `Overflow` gelir; onları yeniden okumak cevabın tamamıdır.
- **Giden klasör bir kez bildirilir.** Silinen, başka yere taşınan ya da ayrılan izlenen klasör bir kez `Gone` olur ve artık izlenmez. Hâlâ önemliyse yeni yolundan yeniden izleyin.
- **Sınırlar çökme değil hatadır.** Sistemin izleme sınırı dolunca (Linux'ta `fs.inotify.max_user_watches`) `watch` bir `QuotaExceeded` hatası döner.
- **İzlemeyi bırakmak bekleyeni uyandırır.** `next()` içindeki iş parçacığı boş bir topluluk döner, sonraki her çağrı da öyle; kapanan bir ekran geride iş parçacığı bırakmaz.
- **Şimdilik yalnızca Linux.** macOS ve Windows'un da olay kaynakları var, ama bu framework hiçbirine `unsafe` ya da büyük bir bağımlılık olmadan ulaşamıyor; orada `new()` `Unsupported` der.

Bu sayfada izlemeyi başlatıp düğmelere basın. Oluşturulan bir dosya, bir yeniden adlandırma ve bir silme, her biri tek değişiklikli bir topluluk olarak gelir. "Oluştur ve sil" iki yüz dosya oluşturup yeniden siler: dört yüz değişiklik bir ya da iki topluluk olarak gelir.

## Sık yapılan hatalar

- **`update` içinde beklemek.** `next()` bekletir; onu yalnızca `Command::perform` içinde çağırın.
- **Yeniden beklemeyi unutmak.** Her `perform` bir topluluk döndürür. Mesajı gelince sonraki beklemeyi başlatın, yoksa ikinci değişiklik hiç duyulmaz.
- **Ağacın her klasörünü izlemek.** Her izleme kullanıcının sınırından düşer. Açık olanı izleyin, kapananı bırakın.
- **Ağacı yalnızca adlardan güncellemek.** Bir toplulukta oluşup yeniden silinen bir girdi olabilir. Klasörü yeniden okuyun; adlar tek bir dosyayı izleyen seyrek ekranlar içindir.
- **`Gone`'u hata saymak.** Kullanıcı klasörü taşıdı ya da sildi; bunu gösterin ve içeriğini göstermeyi bırakın.
- **Kendi değişiklikleriniz için izlemeye güvenmek.** Kendi yazmanızdan hemen sonra yeniden okuyun; ekran zaten bildiği bir haber için saniyenin onda birini beklemesin.
