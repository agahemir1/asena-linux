# Debian paketleri ve yerel APT arşivi

Asena'nın uygulama ve masaüstü bileşenleri aynı `asena` kaynak paketinden
derlenir. Her masaüstü uygulaması ayrı ikili pakettir. Ortak Rust crate'leri
derleme sırasında bağlanır; uygulama kurmak için kaynak deposu veya Cargo
gerekmez. `asena-desktop`, varsayılan bileşenleri bağımlılıklarıyla toplar.

Bu belge uygulanan paketleme düzenini anlatır. Kabul sonuçları ve açık işler
[6A/6B raporunda](../reports/phase-06ab.md), komutlar
[paketleme rehberindedir](../../packaging/README.md).

## Dosya sahipliği

`packaging/debian/inventory.py` paket adlarının, mimarilerinin ve doğrudan
bağımlılıklarının kaynağıdır. `control` bu envanterden üretilir. Masaüstü
uygulamaları `data/applications/*.desktop` içinden çıkarılır. Çalışmayan
`asena-welcome` iskeleti paket listesine alınmaz.

| Sahip | İçerik |
|---|---|
| `asena-common` | GSettings şemaları, çeviriler, üçüncü taraf lisansları, kaynak kimliği |
| `asena-artwork` | Asena simge temaları ve lisanslı duvar kâğıtları |
| `asena-layer-shell` | Onaylı gtk4-layer-shell C 1.3.0 kitaplığı, özel kitaplık dizininde |
| Uygulama paketi | Kendi çalıştırılabilir dosyası, `.desktop` kaydı ve AppStream metaverisi |
| `asena-session` | Wayland oturum kaydı, Wayfire şablonu ve oturum başlatıcısı |
| `asena-daemon` | Daemon, kullanıcı birimi ve portal arka uç kayıtları |
| `asena-lock` | Kilitleyici ve `/etc/pam.d/asena-lock` conffile'ı |
| `asena-greeter` | Greeter, sistem hesabı tanımı ve örnek greetd yapılandırması |
| `asena-packages` | Yetkili paket servisi, çevrimdışı güncelleme birimi, D-Bus ve polkit kayıtları |
| `asena-recovery` | Kurtarma servisi, veri işçisi, kurulum yardımcısı ve GRUB girdisi |

`install.py` her dosya için tek sahip ister. Debian yardımcıları şema, simge,
systemd ve conffile işlemlerini üretir. Paketler kullanıcının ev dizinine
yazmaz. Greeter kurulumu `/etc/greetd/config.toml` dosyasını değiştirmez ve
çalışan giriş yöneticisini değiştirmez. ISO'nun giriş yöneticisi seçimi ayrı
canlı ortam/kurulum işidir.

Kullanıcı oturum birimleri paket kurulumunda etkinleştirilmez; oturum
başlatıcısı onları yönetir. Yedekleme zamanlayıcısı kullanıcı timer hedefinden
başlar; uygulama, yedekleme tercihi kapalıysa işlem yapmadan çıkar. Sistem
servisleri paket kurulumu sırasında başlatılmaz.

## Özel C kitaplığı

Debian'ın gtk4-layer-shell 1.0.4 paketi yerinde kalır. Asena'nın onaylı 1.3.0
kitaplığı `/usr/lib/<multiarch>/asena/` altındadır. Yalnız bu kitaplığa bağlı
Asena ELF dosyalarına o dizinin çalışma zamanı yolu yazılır. Diğer uygulamalar
ve sistem kitaplık arama sırası değiştirilmez.

Yukarı akış arşivinin SHA-256 özeti kaynak hazırlığında doğrulanır.
`packaging/native/` içindeki yama yalnız testin FIFO yanıt okuma yarışını
düzeltir; kitaplığın çalışma zamanı kodunu veya ABI'sini değiştirmez. Yamanın
özeti de kaynak kimliğine kaydedilir. Meson testleri paket derlemesinin
parçasıdır.

## Kaynak ve ağ sınırı

`prepare-debian-source.py`, kilitli Cargo kaynaklarını `vendor/cargo` içine
alır; onaylı C kaynağını, test yamasını ve lisans bildirimlerini ekler.
`source-info.json`, kaynak başvurusunu, Debian sürümünü, Cargo.lock özetini ve
C kaynak/yama özetlerini taşır. Hazırlık sırasında ağ kullanılabilir.

Üretilen yerel kaynak paketi `3.0 (native)` biçimindedir. Ayrı Debian yaması
yerine Asena'nın aynı sürüm kaynağı, tarifleri ve kilitli bağımlılıklarını bir
arada taşır. Yamalı wlroots kendi Debian kaynak paketi olarak ayrıca sunulur.

`build-debs-clean.sh`, temiz Debian 13 köküne yalnız bildirilen derleme
bağımlılıklarını kurar. Ardından derlemeyi dış ağ arayüzü olmayan ayrı ağ
alanında, ayrıcalıksız `builder` hesabıyla çalıştırır. Cargo `--locked
--offline` kullanır. Kaynak paketi, ikili paketlerden önce oluşturulur;
derleme dizinleri ve üretilen paket ağaçları kaynak arşivine girmez.

Bu düzen, çevrimdışı derlenebilir kaynak sağlar. Aynı girdilerin bit düzeyinde
aynı ikili çıktıyı ürettiği ayrıca denenmeden yeniden üretilebilirlik iddiası
yapılmaz.

## Katalog ve imza zinciri

Her uygulama İngilizce/Türkçe ad ve özet, lisans, kategori ve doğru
`desktop-id` taşıyan metainfo ile kurulur. `appstream-generator`, gerçek
paketlerden DEP-11 kataloğunu ve simge arşivlerini üretir. Katalog dosyaları
`Release` içindeki SHA-256 ve SHA-512 listelerine katılır; `InRelease` ve
`Release.gpg` bu son dosyayı imzalar. Böylece katalog da APT'nin doğruladığı
zincirdedir. Katalog üretimi dışarıdan görsel indirmez.

Araç yapılandırması için birincil kaynak:
[AppStream Generator yapılandırması](https://github.com/ximion/appstream-generator/blob/master/docs/asgen-config.md).

## Test anahtarı ve ürün anahtarı

Yerel arşiv `asena-test/main`, `amd64` ve `source` hedeflerini kullanır.
Test anahtarı yedi gün geçerlidir. Özel anahtar `target/keys/asena-test` altında,
herkese açık arşivden ayrı tutulur. Güven paketleri yalnız açık anahtarı ve
`Signed-By` kullanan yerel adresi taşır; küresel APT güveni eklenmez.

Test anahtarı ürün anahtarı değildir. Arşiv internete yüklenmeden önce K08
kararıyla kullanıcıya ait ürün anahtarı, çevrimdışı yedeği ve barındırma adresi
belirlenir. Olağan yenilemede yeni açık anahtar, eski anahtar hâlâ geçerliyken
keyring güncellemesiyle dağıtılır; geçiş süresince iki anahtar kabul edilir.
Kayıp veya şüpheli sızıntıda yayın durdurulur, anahtar iptal edilir ve yeni
parmak izi bağımsız doğrulanmış kanaldan duyurulur. Eski anahtarla güvenli
geçiş sağlanamıyorsa kullanıcı yeni keyring'i ayrıca doğrulayarak kurar.
Yedek ve imzalama yetkisi CI'a veya kaynak deposuna verilmez.

Şimdiki araçlar yerel kabul içindir. Ürün anahtarı üretilmesi ve herkese açık
APT/ISO yayını bu çalışmanın sonucu olarak yapılmış sayılmaz.
