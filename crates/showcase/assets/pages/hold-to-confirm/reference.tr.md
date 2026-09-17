## Metotlar

- `HoldToConfirm::new(etiket)` — etiketi ve üç barı olan, odak alabilen bir çip.
- `.on_confirm(mesaj)` — basılı tutma tamamlanınca bir kez gönderilir. Yoksa kontrol etkin değildir.
- `.duration(süre)` — ne kadar basılı tutulacağı. Varsayılan: 1,2 sn.
- `.key(tuş)` — kontrol görünümde olduğu sürece tuş birleşimini her yerde basılı tutmak da onaylar.
- `.floating(bool)` — yer kaplamaz; yalnızca basılı tutulurken sol üst köşede bir kart gösterir. Varsayılan: `false`.
- `.disabled(bool)` — soluk, odak almaz, basılı tutmak bir şey yapmaz.
- `.color(ifade)` — dolu barın rengi, tema rengi gibi yazılır: `"$danger"`, `"$success"`, `"mix($accent, $danger, 50%)"` ya da `"#RRGGBB"`. Varsayılan: temanın `hold.to` rengi (uyarı). Hedef tema tokenlarıdır; sabit renk kodu çalışır ama temayı dinlemez. Geçerli temada tek bir renk olmayan ifade `hold.to` rengine düşer.
- `Theme::solid(ifade)` — böyle bir ifadeyi temaya göre çözer ya da neden çözemediğini döner.

## Tuşlar

- Odaktayken basılı tutulan `enter` / `space`.
- `key` ile verilen birleşim her yerde; odaklı bileşenlerden sonra, kısayol haritasından önce. Açık bir pencere varsa yalnızca en üstteki pencerenin içinde.

## Fare

- Kontrolün üstünde sol düğmeyi basılı tut. Dışına çıkmak ya da bırakmak iptal eder.

## Davranış

- 9 hücre, aralarında birer hücre olan üçer hücrelik 3 bar; kalan yeri etiket kullanır, dar alanda `…` ile kesilir.
- `n`. bar (0, 1, 2), ilerleme sürenin `n/3` ile `(n+1)/3` arasındayken bir bütün olarak `track` renginden `to` rengine geçer; ilerleme 1 olunca mesaj gönderilir. 1/6'da birinci bar yarıdadır, 1/2'de birinci dolu ve ikinci yarıda, 5/6'da üçüncü yarıdadır.
- Bırakınca barlar bulundukları yerden `motion.enter` süresinde boşalır; yüzen kart onlar boşalana kadar kalır.
- `.color` verilince barlar ve yüzen kartın çubuğu `to` yerine o renge geçer; karışım ve süre aynıdır. İfade her karede çözülür, tema değişince hemen uyar.
- Hover ve odakta vurgu çubuğu ilk hücrede çizilir.
- Tuş, bırakma olayı gelince (kitty klavye protokolü) ya da basıştan sonra 650 ms, son tekrardan sonra 350 ms içinde tekrar gelmezse bırakılmış sayılır.
- Çalışma motorunun butonlara yeniden bastırmadığı Enter ve Boşluk tekrarları kontrole yine ulaşır.
- Basılı fare düğmesi, çalışma motorunun işaretçi tekrarıyla her 40 ms'de bir yoklanır.
- Hareket azaltılmışsa her bar kendi üçte birinin sonunda birden geçer ve bırakınca hemen boşalır; süre değişmez.

## Tema anahtarları

- `hold` — `bg`, `fg`, `bold`, `padding`, `track` (boş bar), `to` (dolu bar; yerleşik temalarda uyarı rengi), `pillar`; durumlar `hover`, `focus`, `active` (basılı), `disabled`.
- `hold-card` — `bg`, `padding`, `pillar` (varsayılanı basılı tutma ilerledikçe `muted` renginden `to` rengine geçer).
- `[motion]` — `enter` (boşalma).
- `[icons]` — `pillar`.
