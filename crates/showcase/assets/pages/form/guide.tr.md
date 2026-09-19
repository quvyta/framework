## Ne zaman kullanılır

Kullanıcı bir şey olmadan önce birlikte denetlenen birkaç değer giriyorsa form kullan: container oluşturmak, dağıtım hedefi eklemek, oturum açmak. Tek tek ve hemen uygulanan ayarlar için ayar listesi daha uygundur.

## Adım adım

1. Değerleri ve bir `FormErrors` değerini durumunda tut: `name: String`, `errors: FormErrors`.
2. Her değeri form sırasıyla denetleyen tek bir fonksiyon yaz ve her sorun için bir mesaj kaydet: `errors.check("name", name.len() >= 3, t!("name-short"))`.
3. Alanları yerleştir: `Form::new().show(ui, |form| { form.field(Field::new(t!("name")).required(true), |ui| { … }); })`.
4. Her kontrole hatanın adını kimlik olarak ver, `.id("name")`; mesajı alanına `.error(errors.get("name"))` ile geçir ve kontrolün kendisini işaretle: `TextInput::invalid(errors.has("name"))`.
5. Gönderirken doğrula; sorun varsa `errors.focus_first()` döndür, odak ilk hatalı kontrole gider.
6. Yetenekleri yalnızca işe yaradığı yerde aç: alana `.hint(…)`, etiket sütunu için forma `.label_width(16)`, alanların üstünde sorun listesi için `.summary(&errors)`.

## Nasıl çalışır

- **Önemli olan her şey uygulamanındır.** Değerler, hatalar ve ne zaman doğrulanacağı sende; form etiketleri, ipuçlarını ve hataları çizer, odağı taşır.
- **Alan; etiket, kontrol ve tek mesaj satırıdır.** Mesaj ipucudur, hata varsa hatadır; hata tehlike renginde ve bir işaretle gelir, asla yalnızca renkle.
- **Zorunluluk bir sözcüktür.** Etiketin ardından silik bir "zorunlu" gelir; yıldız konmaz.
- **Kontrol odaktayken etiketi aydınlanır**; göz, çerçeve olmadan etkin alanı bulur.
- **Enter ilerletir.** Kendi gönderimi olmayan metin alanında Enter sonraki alana, son alandan da formdan sonraki butona geçer.
- **Etiket sütunu geri çekilir.** `label_width` ile etiketler yer oldukça kontrolün yanında durur, dar ekranda üstüne taşınır. Etiketinin yanındaki yere sığmayan bir kontrol, örneğin uzun yer tutuculu bir giriş, kendi etiketini üstüne alır ve bütün satırı kullanır; hiçbir şey kesilmez, öteki alanlar sütunda kalır.
- **`Command::focus` güncellemeler arasında çalışır**; aynı güncellemeyle beliren bir kontrol de odağı alabilir.

## Sık yapılan hatalar

- **Kullanıcı bir şey yapmadan hata göstermek.** Önce gönderirken doğrula; ondan sonra her düzenlemede doğrulamak gürültü değil yardım gibi gelir.
- **Hata adıyla kontrol kimliğinin farklı olması.** `focus_first` yalnızca hatasıyla aynı adı taşıyan kontrole ulaşır.
- **Kontrolü geçersiz işaretlemeyi unutmak.** Mesajı alan gösterir; kontrolün renklenmesi kendi seçeneğidir.
- **Yalnızca "geçersiz" diyen mesajlar.** Ne yapılacağını söyle: "1024 ile 65535 arasında bir port seç".
