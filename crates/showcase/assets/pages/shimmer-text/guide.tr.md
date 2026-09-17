## Ne zaman kullanılır

Süren bir işi temsil eden ve kelimelerin bir simgeden daha önemli olduğu mesajlar için ışıltılı yazı kullan: süren bir arama, işlenen bir istek, yazan biri. Spinner'dan daha sakindir ve konuşmanın bir parçası gibi okunur.

## Adım adım

1. Mesajı ekle: `ui.add(ShimmerText::new(t!("processing")))`. Üzerinden ışık süzülür.
2. Yazıyor göstergesi için nokta stilini seç: `.style(ShimmerStyle::Dots)`.
3. İşe yarıyorsa birleştir: önünde bir `Spinner`, arkasında silik bir geçen süre tam bir çalışma satırı olur.
4. İş biter bitmez yerine sonucu koy.

## Nasıl çalışır

- **Hücre başına ışık.** Her harf kendi rengini alır: merkezinin iki yanında beşer hücrede sönen hareketli ışık bandına uzaklığına göre dinlenme renginden parlak renge karışır. Yalnızca renk değişir; metin hiç kıpırdamaz.
- **Her `motion.shimmer` süresinde bir geçiş.** Bant kenarlara yaklaşırken yavaşlar.
- **Noktalar** aynı sürede sıfırdan üçe çoğalır, sonra baştan başlar. Üç nokta için yer ayrılmıştır; yerleşim hiç kaymaz.
- **Hareketi azaltma.** Süpürme düz renginde durur; noktaların üçü de görünür.

## Sık yapılan hatalar

- **Gerçek içeriği ışıldatmak.** Yalnızca çalışma mesajı için kullan; insanların dikkatle okuması gereken metin için asla.
- **Uzun cümleler.** Işık her uzunlukta aynı sürede geçer; uzun metinde çok hızlı akar. Birkaç kelimede tut.
