## Metotlar

- `Breadcrumb::new(segments)` — kökten bulunulan yere kadar yol.
- `.on_select(|index| msg)` — `index` seviyesi açılınca gönderilir; yoksa yol sade bir yazıdır.
- `.faint(bool)` — yolu, kişinin açamadığı bir yer için bir ton sönük çizer; seviyeler yine açılır.

## Davranış

- Her parça etiketi ve iki yanında birer hücre boşluktur; ayırıcılar bir hücre tutar.
- Yol çok genişse: kök, `…` ve sığan son seviyeler; daha da darsa `…` ve son seviye.
- `…` altında gizli seviyelerin listesini açar; ↑ ↓ gezer, harf yazmak atlar, Enter açar, Esc kapatır.
- Yalnızca `on_select` varsa ve en az iki parça varsa odak alır.

## Tuşlar

- `left` `right` parçalar arasında gezer, `home` `end` uçlara gider, `enter` ya da `space` açar.

## Fare

- Açmak için bir parçaya tıkla; gizli seviyeleri listelemek için `…` işaretine tıkla.

## Tema anahtarları

- `crumb` — `bg`, `fg`; `hover`, `focus`, `active` (açık `…`) durumları.
- `crumb.current` — son seviyenin `fg`, `bold` değerleri.
- `crumb.faint`, `crumb.faint-current` — sönük bir yolun seviyeleri ve son seviyesi; `crumb` durumları yine geçerlidir.
- `crumb-separator` — `fg`.
- `popup-menu`, `popup-item`, `popup-check` — gizli seviyeler listesi.

## İkonlar

- `crumb-separator`: Nerd Font ve Unicode'da küçük bir ok, ASCII'de `:`.
