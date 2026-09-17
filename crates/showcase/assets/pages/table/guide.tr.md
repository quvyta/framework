## Ne zaman kullanılır

Her satırın insanların karşılaştırdığı aynı birkaç bilgiyi taşıdığı yerde tablo kullan: durumu, CPU'su ve belleğiyle container'lar, süresiyle derlemeler, boyutuyla dosyalar. Tek sütunluk adlar için liste, iç içe şeyler için ağaç daha uygundur.

## Adım adım

1. Sütunları tarif et: `Column::new(t!("name"))` boş alanı doldurur; `.width(ColumnWidth::Fit)` sütunu en geniş hücresine göre boyutlar; `.width(ColumnWidth::Fixed(9))` sabitler; `.min(12)` dolduran sütunun okunur kalmasını sağlar.
2. Sayıları `.align(Align::End)` ile sağa hizala; basamaklar alt alta gelir.
3. Satırları kur: `TableRow::new([ad, durum, cpu])`; bir hücre durum ikonu taşıyabilir: `TableCell::new(kelime).icon("dot", Some("success"))`.
4. Satır çoksa durumunda `Arc<[TableRow]>` olarak tut ve klonunu ver; tablo onları hiç kopyalamaz.
5. Seçimi `.selected(..)` ile göster, `.on_select(..)` ve `.on_activate(..)` ile al.
6. Sıralama için sütunları `.sortable(true)` yap, `.on_sort(|sütun, yön| ..)` mesajında verini yeniden sırala ve sonucu `.sort(sütun, yön)` ile göster.
7. Çoklu seçim için `.checked(bool_listesi)` ve `.on_toggle(..)` ekle; boş tablonun ne demek olduğunu `.empty_text(..)` ile söyle.

## Nasıl çalışır

- **Başlık bir çizgi değil, bir yüzeydir.** Başlıklar yükseltilmiş tonun üstünde silik durur; sıralı sütunun başlığı parlar ve vurgu renginde bir ok taşır.
- **Sütunları boşluk ayırır.** Sütunlar arasında iki boş hücre vardır; aralarına hiçbir şey çizilmez.
- **Yalnızca ilk hücre kayar.** Hover edilen ya da seçili satır yükselir, çubuğu gösterir ve görünen ilk hücresini bir hücre sağa kaydırır. Çoklu seçimin işareti ile sağdaki sayılar ve durumlar yerinde kalır; göz gezinirken onları karşılaştırabilir, işaret de hep tıkladığın yerdedir.
- **Sığmayan sütunlar yana kayar.** En küçük genişlikler sığmazsa ← ve → bütün sütunları kaydırır. Başlığın uçlarındaki oklar o yanda gizli sütun olduğunu söyler; oklar birer düğmedir: tıklamak bir sütun kaydırır, fare üstüne gelince aydınlanırlar.
- **Satır sayısı sınırsız.** Yalnızca ekrandaki satırlar çizilir; `Fit` genişlikleri her satır kümesi için bir kez ölçülür.
- **Veri senin, sıra senin.** Tablo satırları asla kendisi sıralamaz; sıralama ister ve senin geri verdiğin oku gösterir.

## Sık yapılan hatalar

- **`view` içinde sıralamak.** Sıralama mesajı gelince `update` içinde sırala; `view` her karede çalışır.
- **Her karede 100 000 satır kurmak.** Veri değişince kur ve `Arc`'ı sakla.
- **Yalnızca renkle durum.** Renkli noktayı demodaki gibi bir kelimeyle eşleştir.
- **Sütunları çizilmiş bir çubukla ayırmak.** Boşluk ve hizalama onları zaten ayırır.
