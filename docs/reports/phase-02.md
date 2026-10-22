# Faz 2 uygulama raporu

19 Ekim 2026. Kullanıcı diğer fazların kaynak uygulamasına devam edilmesini,
elle denemelerin daha sonra yapılmasını istedi. Fazın durumu **sürüyor**.

## Oluşturulan kaynaklar

- Blueprint + CompositeTemplate menü; bir kez oluşturulup açılıp kapanan,
  dış tıklama/Esc ile klavyeyi bırakan layer-shell yüzeyi.
- Görünümden ayrı uygulama, hesap makinesi, kullanıcı komutu ve boş ayarlar
  arama sağlayıcıları. Türkçe `fold`, eşleşme sınıfları, logaritmik kullanım
  katkısı, yerel sıralama, eski sorgu sonuçlarını eleme.
- Worker üzerinde sınırlı JSON kullanım önbelleği; atomik yazma.
- logind yetenekleri, onay/geri sayım; iç içe oturumda gerçek güç işlemleri
  devre dışı. Güvenli kilitleyici henüz olmadığı için kilitle/askıya al etkin değil.
- GTK içermeyen GObject bildirim modeli ve asenkron Notifications 1.2 sunucusu.
  Mevcut sunucunun adı devralınmaz ve kuyruğa girilmez.
- Markup beyaz listesi, güvenli bağlantı açma, rowstride kontrollü sınırlı
  image-data, yerel resim yükleme, replacement, eylemler ve kapanış nedenleri.
- Bellekte en çok 100 geçmiş öğesi; transient geçmişe yazılmaz. Açılır yüzeyler,
  hover ile duran süre, kritik bildirimler, Rahatsız etme, takvim/merkez ve sayaç.
- Boş input region kullanan, 1,5 saniyelik ve yeni değerle süresi yenilenen OSD.

## Doğrulama

Linux Debian 13 VM'de build, fmt, clippy `-D warnings`, test, çeviri, Blueprint,
XML ve ShellCheck kapısı bir ara revizyonda geçti: 23 Rust ve 8 Python testi.
Sonraki değişikliklerin son kontrolü ve GUI denemesi sürüyor; bu ara sonuç fazın
tamamlandığı anlamına gelmez.

Özel D-Bus üzerinde `notifications_probe` geçti: replace aynı kimliği koruyor;
temizlenmiş gövde, eylem, kapanış 1/2/3 nedenleri, kapanmış kimlik reddi ve bellekte
geçmiş sınandı. Ayrı collision deneyi, replacement izni veren mevcut sunucunun
bile devralınmadığını doğruladı.

`scripts/probe-phase2.sh` hesap makinesi/pano, Işık/AYAR/Ayar/ayar uygulama
başlatma, kullanım sayacı ve gerçek yüzeyleri yalıtılmış oturumda sınar.
Yalıtılmış tek çıkışlı oturumda hesap makinesi/pano, Türkçe arama/başlatma, kullanım
sayacı, bildirimler, merkez, OSD ve süreç/ortam temizliği denemesi geçti. Görseller
incelendi; merkez üstündeki popup çakışması düzeltildi. Son kullanıcı görsel kabulü
ve sıcak açılış kare süresi ölçümü açık.

## Açık kabul maddeleri

- Kullanıcının dar/kısa alan, uzun Türkçe metin, açık/koyu, büyük yazı ve
  klavye ile elle denemesi.
- İlk kareye kadar menü açılışı ve tuştan sonuca gecikme ölçümü. Kayda geçen
  visibility çağrısı süresi, kare sunum süresi olarak raporlanmaz.
- Harf başlıkları/harf atlama, avatarın AccountsService kaynağı, sağlayıcı başına
  16 ms birleştirme bütçesi ve hareket token'larının görsel kabulü.
- Gerçek oturum logind/polkit işlemleri; güvenli geçici kilitleyici kararı K04.
- Tüm panel kenarlarında popup/OSD yerleşiminin canlı tercihle bağlanması Faz 5
  ile birlikte doğrulanacak; mevcut popup varsayılan alt paneli kullanır.
- Gerçek blur ve nihai simgeler tamamlanmış sayılmaz.

## Sonraki adaylar

Faz 3 ortak servis modelleri, gerçek ses/ağ/güç denetimi ve hızlı ayarlar.
