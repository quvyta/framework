## Metotlar

- `Install::package(ad) -> Option<Install>` — paket, bu makinenin paket yöneticisiyle; yönetici `PATH`'te `pacman`, `apt-get`, `dnf`, `zypper`, `apk`, `brew` sırasıyla aranır. Hiçbiri yoksa ya da `ad` tek bir paket adı değilse (boş, `-` ile başlıyor, boşluk ya da denetim karakteri içeriyor) `None`. Root iken Homebrew atlanır.
- `Install::package_with(ad, arama, root) -> Option<Install>` — aynısı, arama verilerek: `arama: impl Fn(&str) -> Option<PathBuf>` bir programın nerede olduğunu, `root: bool` uygulamanın root olarak çalışıp çalışmadığını söyler.
- `.name_for(yönetici, ad) -> Install` — yönetici `yönetici` ise `ad`'ı kullanır; diğerlerinde ya da tek bir paket adı olmayan bir adda hiçbir şey değişmez.
- `.manager() -> Manager`, `.name() -> &str` — bulunan yönetici ve paketin onun için adı.
- `.program() -> Vec<OsString>` — komutun sözcükleri, önce program.
- `.command_line() -> String` — komut, kişinin okuduğu haliyle; örneğin `sudo pacman -S --needed libarchive`.
- `.confirm(on_yes) -> Confirm<Msg>` — soru: paket, komut satırının kendisi ve yöneticinin parola sorabileceği; Kur butonuyla. Üstüne `.on_cancel(mesaj)` gibi `Confirm` seçenekleri eklenebilir.
- `.handoff(on_finish) -> Handoff<Msg>` — komutu terminali devrederek, çıktının üstünde bir bildirimle ve `pause(true)` ile çalıştırır; `on_finish` `HandoffOutcome`'u alır.
- `Manager::Pacman`, `Apt`, `Dnf`, `Zypper`, `Apk`, `Brew` (genişletilebilir); `Manager::name() -> &'static str` programdır (`Apt` için `apt-get`); `Manager::all() -> &'static [Manager]` arama sırasıyla.

## Komutlar

- `pacman -S --needed <paket>`, `apt-get install <paket>`, `dnf install <paket>`, `zypper install <paket>`, `apk add <paket>`, `brew install <paket>`; root değilken, `brew` dışında, başında `sudo` olur. Hiçbir zaman evet bayrağı eklenmez.

## Davranış

- Devir döndürülmeden hiçbir şey çalışmaz; onu yalnızca sorunun onay mesajından döndürün.
- Onay butonu Kur yazar; odak Vazgeç'tedir, Vazgeç, Esc ve kapatma işareti hiçbir şey kurmaz.
- `Harness` içinde devir, komutun programı ve argümanlarıyla ve `pause` açık olarak `handoffs()`'a kaydedilir; test başka bir sonuç vermezse `Finished { code: Some(0) }` ile cevaplanır.

## Tema anahtarları

- Soru bir `Confirm`'dür ve onun anahtarlarını kullanır: `modal`, `modal-title`, `close-mark`, `layer-backdrop`, `button.primary`.
- Dil — `quvyta.install.title` (`{package}` ile), `quvyta.install.runs`, `quvyta.install.asks` (`{manager}` ile), `quvyta.install.confirm`, `quvyta.install.notice` (`{package}` ve `{manager}` ile), yönetici bitince beklenen tuş için `quvyta.handoff.pause`.
