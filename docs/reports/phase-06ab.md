# Faz 6A/6B: Debian paketleri ve yerel test deposu

Tarih: 25 Ekim 2026. Bu çalışma, kullanıcının kaynak deposundan sonra gerçek
uygulama paketleri ve imzalı yerel test deposuyla devam etme onayıyla başladı.
Herkese açık APT/ISO yayını kapsamda değildir.

## Kaynak ve ortam

- Taban kaynak: `2504954704f2c02ffe05b807a42f352e5588f9df`; buna paketleme
  çalışma ağacı eklenir. Kaynak arşivinin kendisi ve özeti kesin girdidir.
- Debian 13 amd64 geliştirme VM'si; rustc 1.85.1 ve Debian debhelper 13.24.2.
- Derleme: ayrı debootstrap kökü, ayrıcalıksız `builder`, dış ağ arayüzü
  bulunmayan ağ alanı. Bağımlılık kurulumu bu sınırdan önce yapılır.
- Kurulum testleri için ikinci debootstrap kökü; geliştirme oturumunun
  kurulu paketleri bu testlerin hedefi değildir.
- Bu sırada başka çalışmada değişen duvar kâğıtları ve site başlığı bu paket
  kaynağına veya paketleme commit'ine alınmaz.

## Oluşturulan araçlar

32 Asena ikili paketi tek kaynak paketinden tanımlanır; 19 uygulamanın her
biri ayrı kurulabilir. `asena-welcome` çalışmayan iskelet olduğu için dışarıda
kalır. `asena-layer-shell`, onaylı C 1.3.0 kitaplığını özel dizinde taşır;
Debian'ın sistem kitaplığı değiştirilmez. Dosya sahipliği ve doğrudan
bağımlılıklar `packaging/debian/inventory.py` ile kontrol edilir.

Kaynak hazırlama, temiz derleme, gerçek paket inceleme, yerel imzalı arşiv
üretimi ve kur/yükselt/kaldır denemeleri yeniden çalıştırılabilir betiklerdir.
Komutlar [paketleme rehberinde](../../packaging/README.md), sorumluluklar
[paket mimarisindedir](../architecture/PACKAGING.md).

## Doğrulama durumu

Yerel paket kurma, yükseltme, kaldırma ve imza kabulü geçti. Bu sonuç gerçek
ISO, normal giriş veya ürün yayını kabulü değildir.

| Kontrol | Gözlenen sonuç |
|---|---|
| Envanter ve imzalı Release içeriği testleri | 7 Python testi geçti; toplam son Python koşusu 50/50 |
| Kaynak, XML, çeviri girdileri ve LF denetimi | Geçti; 34 crate, 200 Rust dosyası |
| AppStream metainfo doğrulaması | 19 uygulama geçti |
| C 1.3.0 testleri | Test düzeltmesinden sonra 53/53 geçti |
| Asena temiz paket derlemesi | Geçti; 218 Rust birim/doc testi, temiz kökte 49 Python testi |
| Rust Clippy ve fmt | Linux'ta geçti; Rust kaynakları bu çalışmada değişmedi |
| Paket dosya sahipliği ve ELF yolları | Geçti; 32 ana paket, bir C debug paketi ve yamalı wlroots |
| Binary lintian | Her iki sürümde sıfır hata; uyarılar aşağıda |
| Source lintian | Gereksiz build-essential beyanı düzeltildi; son denetim sürüyor |
| İmzalı arşiv ve katalog | Geçti; Signed-By ile Sources, DEP-11 ve simgeler indirildi; 19 uygulama |
| Temiz kurulum ve kurulu uygulama pencereleri | Shell ile Hesap Makinesi, Dosyalar, Notlar ve Yazılım açıldı |
| İkinci sürüme yükseltme ve veri koruma | `.1` → `.2` geçti; tüm Asena paketleri, mor vurgu, kişisel dosya ve giriş yöneticisi yapılandırması doğrulandı |
| Yanlış anahtar, imzasız/değişmiş kayıt ve bozuk katalog reddi | Dört deneme de APT tarafından reddedildi |
| Purge ve bağımsız uygulama tekrar kurulumu | Geçti; shell olmadan Hesap Makinesi yeniden kuruldu ve açıldı; tercih ve dosya korundu |

Sürümler `0.1.0~dev20261025.1` ve `0.1.0~dev20261025.2`'dir. İkinci
sürüm aynı uygulama kodunun paket yükseltme denemesidir; yeni bir ürün sürümü
ilanı değildir. Derleme önbelleğini kullanmak için hazırlanmış kaynak dizini
ilk sürüm adında kaldı; ikinci kaynak paketinin sürümü changelog ve `.dsc`
içinde `.2`'dir.

Test ekranı ayrı headless Wayfire'dır. Programlar chroot içindeki `/usr/bin`
ve `/usr/share` yollarından, geliştirici kitaplık/şema ortamı temizlenerek
başlatıldı. Ekran görüntüleri pencere açılışını gösterir; bütün uygulama
işlevlerinin veya Yazılım'ın yetkili sistem işlemlerinin yeni kabulü sayılmaz.

Kanıtlar `target/evidence/debian/` altındadır: kurulum ve yükseltme paket
envanterleri, `installed-libraries.log`, `asena-catalog.yml`, dört
`rejected-*.log`, kaldırma çözüm planı ve uygulama ekran görüntüleri.
Paketler/kaynaklar `target/packages/asena/`, herkese açık dosyalardan oluşan
yerel arşiv `target/apt-test/` altındadır. Özel anahtar ayrı
`target/keys/asena-test/` dizinindedir ve teslim arşivine alınmaz.

### Temiz ortamda bulunan düzeltmeler

- SVG testinin gerektirdiği `librsvg2-common` derleme bağımlılığı eklendi.
- Debian'ın zaten zorunlu tuttuğu `dpkg`, `util-linux` ve `build-essential`
  beyanları uygun alanlardan çıkarıldı; standart derleme araçları temiz kök
  hazırlanırken kurulur.
- APT'nin sıkıştırılmış dosya kadar açılmış indeksin özetine de ihtiyaç duyduğu
  doğrulandı. Sources, DEP-11 ve simgelerin açılmış biçimleri imzalı Release'e
  katıldı; bu durum için regresyon testi eklendi.
- Test arşivi normal APT önceliğini kullanır. Böylece oturumun istediği
  onaylı `wlroots +asena1`, Debian'ın eski sürümü yerine çözülebilir.
- Tekrarda eski debug çıktısı kalmaması için Debhelper durumu sıfırlanır;
  Cargo/native derleme önbelleği korunur. Rust çıktılarında debug bilgisi
  olmadığından yalnız C kitaplığının ayrı debug paketi üretilir.

`asena-desktop` bütün varsayılan uygulamalara Depends ile bağlıdır. Notlar'ı
kaldırmanın simülasyonu metapaketi de kaldırır; shell/session/daemon aynı
işlemde kaldırılmaz. Otomatik kaldırmadan önce korunacak paketleri elle
kurulmuş olarak işaretlemek gerekir; farklı metapaket politikası ayrı karardır.

Lintian uyarıları GUI başlatıcılarında man sayfası olmaması, özel C kitaplığı
adlandırması ve çevrimdışı güncelleme/DPKG erişiminin standart dışı görünmesi
gibi alanlarda kalır. Kapsayıcı bastırma eklenmedi. Kaynakların bit düzeyinde
yeniden üretilebilirliği denenmedi.

Çalışma sırasında VM iki kez kapandı; diskteki önbellekten devam edildi.
Ek debug derlemesi diski doldurunca yalnız derleme önbelleği temizlendi.
Paket kabulü daha sonra boş alan varken yeniden çalıştırıldı. Bu kesintiler
başarılı test sonucu olarak sayılmadı.

### Yukarı akış test yarışının düzeltmesi

İlk temiz derlemede C kitaplığının 53 testinden biri, çok monitörlü kilit
testinde `bytes_read > 0` koşulunda kaldı. Test istemcisi yanıt FIFO'sunu
yeniden açtığında önceki sunucu yazıcısı hâlâ açık olabiliyordu. O yazıcı
kapanınca yeni yanıt gelmeden EOF okunuyordu.

`packaging/native/0001-keep-test-response-fifo-open.patch`, istemcinin FIFO'yu
okuma/yazma açmasını sağlar; okuma sonraki yanıtı bekler. Yalnız test altyapısı
değişir. Kurulu kitaplığın çalışma zamanı kodu ve ABI'si değişmez. Orijinal
arşiv özeti ile yama özeti ayrı kaydedilir. Tekrarda 53 testin hepsi geçti.

## Açık sınırlar

Paketler henüz sürüm adayı beyanı değildir. Gerçek GPU, normal greetd girişi,
tam paketlenmiş masaüstü kabulü, Flatpak'ta simge/GTK3 tema kabulü, sistem
kimliği ve canlı ortam paketlerinin ISO'ya bağlanması açıktır. Hoş geldin,
Calamares, kurtarma bölümü ve gerçek ISO denemeleri diğer Faz 6 işleridir.
Katalogda yayın geçmişi ve gerçek ekran görüntüleri henüz yoktur.

Yerel test anahtarı ürün anahtarı değildir. Ürün anahtarı sahipliği, yedek,
barındırma ve yayın adımı K08'de kesinleşir. Dış CI çalıştırılmadı.

## Sonraki adaylar

- K08 için ürün anahtarı ve kalıcı APT/indirme barındırması.
- Sistem kimliği, canlı ortam paketleri ve ISO'nun yerel arşivi tüketmesi.
- K22 için adw-gtk3 paketi ve Flatpak tema kabulü.
- Uygulamalar için gerçek ekran görüntüleri ve yayın geçmişi.
- CLI man sayfaları ve isteğe bağlı uygulamaların metapaket politikası.
