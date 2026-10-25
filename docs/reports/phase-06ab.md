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

Derleme ve kurulum kabulü sürüyor. Sonuçlanmamış adımlar başarılı sayılmaz.

| Kontrol | Gözlenen sonuç |
|---|---|
| Envanter ve imzalı Release içeriği testleri | 6 Python testi geçti |
| Kaynak, XML, çeviri girdileri ve LF denetimi | Geçti; 34 crate, 200 Rust dosyası |
| AppStream metainfo doğrulaması | 19 uygulama geçti |
| C 1.3.0 testleri | Test düzeltmesinden sonra 53/53 geçti |
| Asena temiz paket derlemesi | Sürüyor |
| Paket dosya sahipliği, ELF yolları ve lintian | Bekliyor |
| İmzalı arşiv ve katalog | Bekliyor |
| Temiz kurulum ve kurulu uygulama pencereleri | Bekliyor |
| İkinci sürüme yükseltme ve veri koruma | Bekliyor |
| Yanlış anahtar, imzasız/değişmiş kayıt ve bozuk katalog reddi | Bekliyor |
| Purge ve bağımsız uygulama tekrar kurulumu | Bekliyor |

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
