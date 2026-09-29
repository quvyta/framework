## Ne zaman kullanılır

Uygulamanız makinede olmayan bir programa ihtiyaç duyduğunda kullanın: `bsdtar` olmadan bir arşiv görüntüleyici, dil sunucusu olmadan bir düzenleyici, `git` olmadan bir araç. Kişiye bir komutu terminale kopyalatmak yerine işi burada, onun gözü önünde ve onayından sonra yapmayı önerin. Paket yöneticisi bulunamazsa bunun yerine hangi paketin gerektiğini söyleyin.

## Adım adım

1. Program eksikse paket yöneticisini bulun: `Install::package("libarchive")`. Makinede framework'ün bildiği bir yönetici yoksa `None` verir.
2. Paketin bir dağıtımda başka bir adı varsa bunu söyleyin: `.name_for(Manager::Apt, "libarchive-tools")`.
3. Komutu kişinin karar verdiği yerde, örneğin butonun yanında gösterin: `install.command_line()`.
4. Butona basılınca sorun: `Command::confirm(install.confirm(Msg::Install))`. Henüz hiçbir şey çalışmaz.
5. Terminali ancak cevap gelince devredin: `Msg::Install => Command::handoff(install.handoff(Msg::Installed))`.
6. `Msg::Installed(sonuç)` içinde programı yeniden arayın ve kişiye nasıl gittiğini söyleyin: `Finished { code: Some(0) }` kurdu, başka her sonuç kurmadı.

## Nasıl çalışır

- **Makinenin kendi paket yöneticisi.** `pacman`, `apt-get`, `dnf`, `zypper`, `apk` ve `brew` bu sırayla `PATH`'te aranır, ilk bulunan kullanılır. `paru` gibi AUR yardımcıları bilerek dışarıdadır: kaynaktan kullanıcı olarak derlerler, bu da "bu paketi kur" demek değildir.
- **Çalışmadan önce gösterilen komutun kendisi.** `sudo pacman -S --needed libarchive`: yöneticinin kendi kurma komutu; uygulama root değilse başında `sudo` olur (Homebrew hiç almaz, root iken de çalışmayı reddettiği için atlanır). Soru bu satırın aynısını gösterir.
- **Evet bayrağı yok.** Paket yöneticisi "Kuruluma devam edilsin mi?" gibi kendi sorusunu sorar, kişi onu terminalde cevaplar.
- **Parola paket yöneticisine gider.** Devir, terminali çalıştıkları sürece `sudo`'ya ve yöneticiye verir; uygulamanız parolayı hiç görmez.
- **Çıktı okunur kalır.** Devir, yönetici bitince bir tuşa basılmasını bekler; son satırları, bir hata da olsa, ekranınız geri gelmeden okunur.
- **Esc hiçbir şey kurmaz.** Vazgeç, Esc ve kapatma işareti hep reddeder; bunu duymak için soruya `.on_cancel(mesaj)` ekleyin.
- **Testler asla kurmaz.** `Install::package_with(ad, arama, root)` program aramasını ve root sorusunu alır; `Harness` devri çalıştırmak yerine kaydeder.

## Sık yapılan hatalar

- **Soruyla aynı adımda devretmek.** Devri yalnızca onay mesajından döndürün; cevaptan önce hiçbir şey çalışmamalı.
- **`-y` ya da `--noconfirm` eklemek.** Kişi tek bir pakete onay verdi; yöneticinin kendi sorusu başka neler geldiğini gösterir.
- **İşe yaradığını varsaymak.** Sıfır olmayan çıkış kodu ya da iptal edilen parola sorusu hiçbir şeyin kurulmadığı demektir; programı yeniden arayın.
- **Komutu kendiniz kurmak.** `install.command_line()` ve `install.handoff(..)` kullanın; böylece kişinin okuduğu satır çalışan satırdır.
