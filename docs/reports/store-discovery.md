# Yazılım Keşfet akışı · 24 Ekim 2026

Kullanıcı kategori listesinin yerine Google Play benzeri canlı ve etkileşimli
bir Keşfet akışı istedi. Bu çalışma yalnız mağazanın keşif yüzeyini değiştirir.

## Yerleşim kararı

`target/store-layouts.html` içinde aynı içerikle iki yerleşim karşılaştırıldı:
tek vitrin ve yatay raflar; iki sütunlu mozaik. Vitrin ve raflar seçildi.
Geniş pencerede ekran görüntüsü okunur kalır; dar pencerede sıralama değişmeden
raflar yatay kayar. Kategori kısayolları akışın başındadır.

## Kaynak davranışı

- Dört uygulamaya kadar elle kaydırılan vitrin; farklı kategoriler ve ekran
  görüntüsü olan kayıtlar önceliklidir. Otomatik dönme yoktur.
- Her dolu kategori için en fazla sekiz gerçek uygulamalı yatay raf.
  Grafik, ses/video ve oyun raflarında varsa AppStream ekran görüntüsü gösterilir.
  Farklı katalog kimlikleriyle gelen aynı adlı uygulama rafta bir kez gösterilir;
  tam kategori ve arama sonuçlarında kaynak kayıtları korunur.
- Kart tıklaması mevcut uygulama ayrıntısına gider. Kategori kısayolu ve
  “Tümünü gör” tam katalog listesini açar. Vitrin ve rafların okları,
  dokunma/kaydırma ve standart GTK klavye odağı kullanılabilir.
- Seçim gerçek katalogdan yapılır; erişilebilir kaynak bulunmayan kayıt seçilmez.
  Editoryal tercihler indirme, puan veya popülerlik iddiası değildir.
- Görseller HTTPS ve mevcut disk önbelleği üzerinden gelir; toplam üç,
  sunucu başına iki bağlantı ve akış okunurken 8 MiB yanıt sınırı vardır.
  İstekler kart görünür alana girince başlar. Görsel başarısızsa diğer
  katalogdan gelen kapak ve sonraki görüntü denenir; hepsi başarısızsa
  uygulamanın gerçek simgesi ve bilgileri kalır. Canlı denemede Krita'nın
  Debian adresi 404 döndü; aynı uygulamanın Flathub görseliyle doğrulandı.
- Kart renkleri libadwaita tema renkleridir; açık, koyu ve yüksek kontrast
  için sabit renk kullanılmaz. Yeni metinler Türkçeye çevrilmiştir.

## Doğrulama

- Debian 13 VM'de `cargo build --workspace --locked`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`, `cargo test --workspace --locked` geçti.
  218 Rust testi; üç yeni test gerçek kaynak seçimi, kategori/sınır,
  vitrin çeşitliliği ve ikinci katalogdan görsel yedeğini kapsar.
- `scripts/update-po.sh --check`, `msgfmt --check` ve `git diff --check` geçti.
  Testlerde `GSETTINGS_SCHEMA_DIR=target/schemas`, `GSETTINGS_BACKEND=memory`
  kullanılır; normal deneme oturumu kendi kayıtlı ayarlarını kullanır.
- `scripts/dev-session.sh` üzerinden çalışan Wayfire/noVNC oturumunda vitrin
  ileri geçişi, Krita kartından gerçek ayrıntı/kurulum sayfasını açma,
  geri dönme ve Grafik kısayolundan tam kategori listesi doğrulandı.
- 360 piksel genişlikte alt sekme çubuğu, tek sütun ve yatay raflar kontrol
  edildi. 25 Ekim son oturumunda koyu tema, gerçek Krita katalog araması ve
  Kurulu sekmesi de doğrulandı; son oturum Keşfet açık bırakılarak teslim edildi.
- 25 Ekim görsel düzeltmesi: kategori kısayolları ve bütün yatay uygulama
  raflarında içerik üzerine binen kaydırma çubuğu ortak `External`
  politikasıyla kaldırıldı. Yatay kaydırma, klavye odağı ve raf okları korunur.
- Kanıtlar: `target/evidence/store-discovery/` içindeki derleme/kalite/test
  günlükleri ve canlı ekran görüntüleri. Kullanıcının görsel kabulü açık.
