## Ne zaman kullanılır

Kullanıcı bir şey olmadan önce birlikte denetlenen birkaç değer giriyorsa form kullan: container oluşturmak, dağıtım hedefi eklemek, oturum açmak. Tek tek ve hemen uygulanan ayarlar için ayar listesi daha uygundur.

## Adım adım

1. Değerleri ve bir `FormErrors` değerini durumunda tut: `name: String`, `errors: FormErrors`.
2. Her değeri form sırasıyla denetleyen tek bir fonksiyon yaz ve her sorun için bir mesaj kaydet: `errors.check("name", name.len() >= 3, t!("name-short"))`.
3. Alanları yerleştir: `Form::new().show(ui, |form| { form.field(Field::new(t!("name")).required(true), |ui| { … }); })`.
4. Her kontrole hatanın adını kimlik olarak ver, `.id("name")`; mesajı alanına `.error(errors.get("name"))` ile geçir ve kontrolün kendisini işaretle: `TextInput::invalid(errors.has("name"))`.
5. Gönderirken doğrula; sorun varsa `errors.focus_first()` döndür, odak ilk hatalı kontrole gider.
6. Değerin neye mal olacağını, söylemeye değdiğinde alanın `.warning(…)` seçeneğiyle söyle. Hatanın çizildiği yerde çizilir ve bir sorun değildir: yalnızca uyarı taşıyan alanlardan oluşan bir form yine gönderilir.
7. Yetenekleri yalnızca işe yaradığı yerde aç: alana `.hint(…)`, etiket sütunu için forma `.label_width(16)`, alanların üstünde sorun listesi için `.summary(&errors)`.

## Nasıl çalışır

- **Önemli olan her şey uygulamanındır.** Değerler, hatalar ve ne zaman doğrulanacağı sende; form etiketleri, ipuçlarını, uyarıları ve hataları çizer, odağı taşır.
- **Alan; etiket, kontrol ve tek mesaj satırıdır.** O yere üç mesajdan yalnızca biri gelir: önce hata, sonra uyarı, en son ipucu. Hata uyarıdan o yeri alır; böylece alan bir yere sığan iki şeyi birden söylemez.
- **Bir durum rengi asla tek başına gelmez.** Hata ve uyarı işaretlidir: hata `error` işaretiyle tehlike renginde, uyarı `warning` işaretiyle uyarı renginde — o türdeki bir bildirimin taşıdığı iki işaret; böylece alan ile aynı türden bir bildirim benzer görünür.
- **Uyarı sorun değildir.** `FormErrors` içine girmez, `Form::summary` içinde görünmez, odağı taşımaz ve gönderimi durdurmaz. Uygulamanın kendi cümlesidir, yanındaki değerden kurulur: "Paylaşılan depodan değil, çekme yavaş olabilir".
- **Zorunluluk bir sözcüktür.** Etiketin ardından silik bir "zorunlu" gelir; yıldız konmaz.
- **Kontrol odaktayken etiketi aydınlanır**; göz, çerçeve olmadan etkin alanı bulur.
- **Enter ilerletir.** Kendi gönderimi olmayan metin alanında Enter sonraki alana, son alandan da formdan sonraki butona geçer.
- **Etiket sütunu geri çekilir.** `label_width` ile etiketler yer oldukça kontrolün yanında durur, dar ekranda üstüne taşınır. Etiketinin yanındaki yere sığmayan bir kontrol, örneğin uzun yer tutuculu bir giriş, kendi etiketini üstüne alır ve bütün satırı kullanır; hiçbir şey kesilmez, öteki alanlar sütunda kalır.
- **`Command::focus` güncellemeler arasında çalışır**; aynı güncellemeyle beliren bir kontrol de odağı alabilir.

## Sık yapılan hatalar

- **Kullanıcı bir şey yapmadan hata göstermek.** Önce gönderirken doğrula; ondan sonra her düzenlemede doğrulamak gürültü değil yardım gibi gelir.
- **Hata adıyla kontrol kimliğinin farklı olması.** `focus_first` yalnızca hatasıyla aynı adı taşıyan kontrole ulaşır.
- **Uyarıyı hata olarak kaydetmek.** Değer kullanılabiliyorsa uyarı maliyetini söyler; `FormErrors` içine yazmak, kullanıcının gideremediği bir sebeple formu gönderilemez hâle getirir.
- **Kontrolü geçersiz işaretlemeyi unutmak.** Mesajı alan gösterir; kontrolün renklenmesi kendi seçeneğidir.
- **Yalnızca "geçersiz" diyen mesajlar.** Ne yapılacağını söyle: "1024 ile 65535 arasında bir port seç".
