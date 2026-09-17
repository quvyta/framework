## Ne zaman kullanılır

Bir iş sabit sırada birkaç grup yanıt istiyorsa ve sonraki sorular öncekilere bağlıysa sihirbaz kullan: dağıtım hedefi eklemek, proje kurmak, hesap bağlamak. Bütün değerler tek ekrana sığıyorsa form daha hızlıdır.

## Adım adım

1. Güncel adımı, bütün değerleri ve bir `FormErrors` değerini durumunda tut.
2. Sihirbazı göster: `Wizard::new(etiketler).current(state.step).on_back(Msg::Back).on_next(Msg::Next).on_finish(Msg::Finish).show(ui, |ui| { … })`.
3. Sayfa kapanışında `state.step` adımının sayfasını çiz; çoğunlukla bir `Form`.
4. Her adım için bir doğrulama yaz. `update` içinde `Next` gelince güncel adımı doğrula; sorun varsa `errors.focus_first()` döndür ve yerinde kal, yoksa sonraki adıma geç.
5. `Finish` gelince işi yap; bu sırada `.busy(true)` Bitir'i çalışır gösterir.
6. Akışın ihtiyacını aç: Vazgeç butonu ve Esc için `.on_cancel(Msg::Cancel)`, bitmiş adımlara dönmek için `.on_step(Msg::GoTo)`, butonlar yerinden oynamasın diye `.page_height(satır)`.

## Nasıl çalışır

- **Sihirbaz; adım göstergesi, bir sayfa ve bir buton satırıdır.** Adımlar kullanıcının nerede olduğunu gösterir; Geri ikinci adımdan itibaren görünür; İleri son adımda Bitir olur ve ana butondur.
- **Doğrulama, sihirbaz bilmeden İleri'yi durdurur.** İleri yalnızca senin mesajını gönderir. İlerlemeye sen karar verirsin; `focus_first` imleci soruna koyar.
- **Odak akışı izler.** `Command::focus` aynı güncellemeyle beliren kontrollerde de çalışır; yeni adımın ilk alanına odaklanabilirsin.
- **Esc yalnızca istenirse vazgeçer.** `on_cancel` yoksa Esc uygulamadaki anlamını korur.
- **Enter ilerletir.** Form içinde Enter alandan alana, son alandan İleri'ye gider.

## Sık yapılan hatalar

- **Her şeyi sonda doğrulamak.** Her adımı, alanları ekrandayken İleri'de denetle.
- **Geri'de yanıtları kaybetmek.** Değerler durumunda yaşar, geri dönmek onları korur; `Back` içinde sıfırlama.
- **Çok fazla adım.** Üç ile beş adım rahat okunur; kısa adımları birleştir.
- **İş sürerken bitmiş görünen Bitir.** İş sonucunu bildirene kadar `busy` kullan.
