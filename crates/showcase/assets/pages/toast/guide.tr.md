## Ne zaman kullanılır

Arka planda bir şey olduğunu ve yanıt gerekmediğini söylemek için bildirim kullan: bir dağıtım bitti, bir disk doluyor, bir derleme başarısız oldu. Ekranı kilitlemez ve kendiliğinden gider. Kararlar için diyalog kullan; bir alanla ilgili hatayı o alanın yanında göster.

## Adım adım

1. `update` içinden bir bildirim döndür: `Command::toast(Toast::success(t!("deployed")))`.
2. İşe yarıyorsa ayrıntı ekle: `.body(t!("deployed-body"))`.
3. Tek bir sonraki adım sun: `.action(t!("retry"), Msg::Retry)`; mesaj diğerleri gibi gelir.
4. İlerleyen bir şey için anahtar ver: `.key("upload")`. Yeniden göstermek onu yerinde değiştirir; `Command::dismiss_toast("upload")` kaldırır.
5. İş sürerken ikon hareket etsin: `.icon_motion(SpinnerStyle::Pulse)`. İş bitince sonucu aynı anahtarla, animasyonsuz bir bildirimle göster: `Toast::success(t!("uploaded")).key("upload")`.
6. Haberin gidilecek bir yeri varsa bildirimi basılabilir yap: `.on_press(Msg::OpenDeployLog)`. Bildirime basmak mesajı gönderir, bildirim yerinde kalır; oyun alanındaki "Basılabilir dağıtım bildirimi" bunu dağıtım bildirimi için açar.
7. Köşeyi bir kez, örneğin açılışta seç: `Command::toast_corner(Corner::TopRight)`.

## Nasıl çalışır

- **Yığın çalışma motorunundur.** Bildirimler görünümünün parçası değildir: onları açan ekrandan uzun yaşar, diyaloglar dahil her katmanın üstüne çizilir. Uygulaman bildirim durumu tutmaz.
- **Durum bir işarettir, yalnızca renk değil.** Solda durum renginde bir sütun ve bir ikon (başarı, uyarı, tehlike, bilgi) durur; başlık ve gövde nötr renkte kalır.
- **Kayarak girer**: `motion.enter` süresince ekran kenarından hücre hücre gelir, renkleri de aynı hızla belirir; aynı yoldan kayarak çıkar. Hareket azaltılmışsa anında belirir ve kaybolur.
- **Bir köşede üst üste dizilir**; en yenisi köşeye en yakındır, aralarında bir satır boşluk vardır. Sığmayan bildirimler diğerleri gidene kadar bekler.
- **İkon hareket edebilir.** Spinner'ın tek hücrelik animasyonlarından herhangi biri (Yükleniyor animasyonu ve Animasyon stüdyosu sayfalarındaki stiller, Pulse dahil) ikon hücresinde, bildirimin türünün renginde oynar ve diğer renkler gibi bildirimle birlikte belirir. Başlık sütununu korur; animasyondan ikonuna geçen anahtarlı bir bildirim yerinden oynamaz. Hareket azaltılmışsa türün ikonu sabit durur. Oyun alanındaki "İkon animasyonu" yükleme bildiriminin stilini seçer.
- **Kendiliğinden gider**: 5 saniye sonra, eylem taşıyorsa 8 saniye sonra ya da `.duration(…)` ile verdiğin sürede. İmleç üstündeyken geri sayım durur.
- **Kapatma işareti sekmelerdeki ve diyaloglardakinin aynısıdır.** Başlık satırının sonunda üç hücre, boştayken fısıltı kadar silik. İmleç bildirimin üstüne gelince işaret değişmez; yalnızca imleç işaretin kendisine gelince üç hücre birlikte aydınlanır. Her bildirim kapatılabildiği için her zaman oradadır.
- **Yalnızca kapatma işareti kapatır.** İşarete tıklamak bildirimi kapatır; eyleme tıklamak mesajını gönderir ve kapatır. Sade bir bildirimin başka bir yerine tıklamak hiçbir şey yapmaz; bildirim yanlışlıkla gelen bir tıklamayla kaybolmaz.
- **Basılabilir bildirim bir yere götürür.** `.on_press(msg)` ile bildirime (eylemine ya da işaretine değil) tıklamak `msg` gönderir, örneğin haberin geldiği logu ya da sayfayı açmak için; bildirim, işareti ya da süresi kapatana kadar kalır. İmleç üstündeyken bildirim bir kademe yükselir (`$overlay`'den `$active`'e), eylem düğmesi de onunla birlikte yükselir. Sade bildirimde hover yoktur, çünkü basılacak bir şeyi yoktur.
- **Tıklama bildirimde kalır.** Bildirime yapılan hiçbir tıklama altındakine ulaşmaz.

## Sık yapılan hatalar

- **Kullanıcının mutlaka yanıtlaması gereken şey için bildirim.** Kaybolur; diyalog kullan.
- **Her ilerleme adımı için ayrı bildirim.** İlerlemeye tek bir anahtar ver; üst üste yığılmak yerine yerinde güncellensin.
- **Uzun metin.** Başlığı kısa tut; gövde satır kırar ama bildirim bir log değildir.
- **Hiç durmayan animasyon.** Hareket eden ikon "hâlâ sürüyor" der; iş bitince bildirimi değiştir, yoksa gidene kadar atmaya devam eder.
