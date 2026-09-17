## Metotlar

- `NodeMut::selectable(bool)` — `true` düğümü bir seçim bölgesi yapar; `false` seçimi düğümün ve içindeki her şeyin dışında tutar, bir bölgenin içinde de. Verilmezse düğüm ikisi de değildir; yani varsayılan olarak hiçbir şey seçilemez.
- `PaintCx::selectable(alan)` — bileşenin bir kısmını seçim bölgesi yapar; `CodeView`, `Markdown` ve `Terminal` içerikleri için bunu çağırır.
- `PaintCx::unselectable(alan)` — bileşenin, basışın asla seçim başlatmadığı kısmı.
- `PaintCx::decoration(alan)` — temiz kopyanın almadığı hücreler; `PaintCx::pillar` kendi hücresini kendisi işaretler.
- `ClipboardEvent::Copied(metin)` ile `App::clipboard` — seçimin her kopyasını duyar.
- `Harness::drag(başlangıç, bitiş)`, iki ya da üç kez `Harness::click`, menü için `Harness::mouse(MouseKind::Down(MouseButton::Right), x, y)`, kopyalanan için `Harness::clipboard()`.

## Kurallar

1. Bir düğüm ya da bileşen istemedikçe hiçbir şey seçilemez.
2. Sol basış önce bileşenlere gider; onu kullanan bileşen fareyi tutar.
3. Bir bölgenin içindeki kullanılmayan basış seçim başlatır; seçilemez bir alana düşmüşse ya da bölge bir pencerenin altındaysa başlatmaz.
4. Bölge, basışın altındaki en içteki bölgedir; kaydırma alanı gibi odaklanabilir bir bölge, basışın altındaki çocuğuna daraltılır ve kaydırma çubuğunu içermez.
5. Sürükleme hücre hücre seçer; aynı hücreye 400 ms içinde yapılan basışlar önce kelimeyi, sonra satırı seçer.
6. Bırakmak seçimi korur, bir şey kopyalamaz; hareket etmeden yapılan düz tıklama bir şey seçmez.
7. `copy` (`ctrl c`) temiz kopyalar ve `motion.flash` süresince parlar. Seçime sağ basış Kopyala (temiz) ve Ham kopyala menüsünü açar; menü kapanana kadar tuşlar ondadır.
8. Temiz kopya: süs hücreleri atlanır, yalnızca süsten oluşan satırlar atılır, her satırın sonundaki boşluklar kırpılır. Ham kopya: seçili her hücre.
9. Seçim, ilk basışın altındaki en küçük bileşenle birlikte hareket eder; o bileşen ya da bölge kaybolunca, başka bir yere basınca ya da `copy` dışındaki herhangi bir tuşta kalkar.

## Tuşlar

- `[global] copy = "ctrl+c"`, etiketi `quvyta.keys.copy` — seçimi temiz kopyalar. Kendi seçimi olan odaklı bir metin alanı `ctrl c` tuşunu önce kendisi işler.
- Menüde: ↑ ↓ gezinir, Enter seçer, Esc kapatır.

## Tema anahtarları

- `text-selection` (`bg`) ve kopyaladıktan sonraki parlama için `text-selection:pressed`.
- Menü `context-menu` ve `context-item` ile çizilir.

## Dil anahtarları

- `quvyta.edit.copy`, `quvyta.edit.raw-copy`.
