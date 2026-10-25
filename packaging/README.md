# Asena paket ve uygulama dağıtımı

Debian paket tarifleri ve yerel imzalı test arşivi araçları bu dizindedir.
Gerçek derleme ve kabul sonuçları [6A/6B raporunda](../docs/reports/phase-06ab.md)
tutulur. Herkese açık APT servisi veya ISO yayını henüz yoktur.
Kapsam [Faz 6](../prompts/06-dagitim.md), kararlar
[PLANNING K08, K21 ve K31](../docs/PLANNING.md) ile izlenir.

## Kaynak ve yayın yerleri

| İçerik | Yer | Durum |
|---|---|---|
| Kaynak, belgeler ve sorun takibi | [agahemir1/asena-linux](https://github.com/agahemir1/asena-linux) | `main` ilk kaynak yüklemesi 25 Ekim'de tamamlandı |
| Proje sitesi | `https://asenalinux.netlify.app` | Mevcut Netlify sitesi |
| Asena sistem ve uygulama paketleri | Yerel imzalı APT test arşivi | Ürün barındırması ve anahtarı açık |
| ISO ve sürüm indirmeleri | Ayrı indirme alanı | ISO yok; barındırma seçilmedi |

İlk aşamada kaynaklar tek depoda tutulur. Ayrı uygulama paketleri için ayrı Git
depoları gerekmez. Debian sistem paketleri Debian'dan, XanMod kendi sınırlı
kaynağından gelmeye devam eder. Asena arşivi kendi paketlerini ve onaylı
backport veya yamaları taşır.

## Paket düzeni

`debian/inventory.py`, aynı kaynak paketinden üretilen 32 ikili paketi tanımlar.
19 masaüstü uygulaması ayrı ayrı kurulabilir. `asena-welcome` henüz çalışan bir
uygulama olmadığı için bu envantere alınmaz. Dosya sahipliği, özel C kitaplığı
ve kaynak düzeni [paket mimarisinde](../docs/architecture/PACKAGING.md) açıklanır.

| Paket grubu | İçerik |
|---|---|
| `asena-desktop` | Varsayılan masaüstü ve uygulamaları toplayan metapaket |
| `asena-common` | Ortak şemalar, çeviriler ve çalışma zamanı verileri |
| `asena-artwork` | Simgeler, duvar kâğıtları ve lisans bildirimleri |
| Sistem bileşenleri | Shell, daemon, session, kilit, greeter ve polkit ajanı için ayrı paketler |
| Yetkili servisler | `asena-packages` ve `asena-recovery`; D-Bus, polkit ve systemd dosyalarıyla |
| Uygulamalar | Her uygulama ayrı paket; ortak source package'tan derlenebilir |
| `asena-layer-shell` | gtk4-layer-shell 1.3.0; Debian'ın 1.0.4 paketini değiştirmeyen özel dizin |
| `asena-test-archive-keyring` | Yalnız yerel test arşivinin açık anahtarı; arşiv aracı üretir |
| `asena-test-sources` | Yerel test adresi ve `Signed-By` kaydı; arşiv aracı üretir |
| `libwlroots-0.18` | Onaylı `0.18.2-3+asena1` yaması; ayrı Debian kaynak paketi |

Planlanan `asena-live-config` yalnız canlı ortam ayarlarını taşıyacaktır.
Sistem kimliği ve canlı ortam paketlerinin ISO'ya bağlanması ayrı adımdır.
Paket kurulumu mevcut display manager'ı kendiliğinden değiştirmez.
Greeter etkinleştirmesi yönetici seçimi olarak kalır.

## Uygulamaları Yazılım'da göster

Her masaüstü uygulamasının paketine kimliğiyle eşleşen
`/usr/share/metainfo/org.asenalinux.<App>.metainfo.xml` eklenir. Metaveri
Türkçe ve İngilizce ad, özet, açıklama, lisans ve kategori taşır.
`desktop-id`, `.desktop` dosyasıyla eşleşir. Ekran görüntüleri ve yayın geçmişi
henüz kataloğa eklenmedi.

`appstream-generator`, üretilen `.deb` paketlerinden katalog ve simgeleri
oluşturur. APT arşivinin imzalı metadata zinciri bu kataloğu da kapsar.
Yazılım zaten sistem AppStream kataloğunu okur ve `asena` kökenini ayırır.
Kabul denemesinde yeni bir Asena uygulaması bulunur, kurulur, açılır,
güncellenir ve kaldırılır. Metapaket bağımlılıklarının kaldırma davranışı ayrıca denenir.

## Debian 13'te derle

Komutları depoda çalıştırın. Windows'ta paket derlenmez.

```sh
sudo apt-get update
sudo apt-get install debhelper devscripts debootstrap patchelf reprepro \
    appstream-generator lintian gnupg xz-utils
bash scripts/setup-dev.sh
python3 scripts/prepare-debian-source.py \
    --version 0.1.0~dev20261025.1 --source-ref "$(git rev-parse HEAD)"
sudo bash scripts/build-debs-clean.sh \
    target/packages/asena/asena-0.1.0~dev20261025.1
python3 scripts/check-debs.py target/packages/asena \
    --report target/evidence/debian/packages.tsv
lintian --fail-on error target/packages/asena/*.changes
```

Kaynak hazırlama ağ kullanır. Kilitli Cargo bağımlılıklarını, onaylı C kaynağını
ve lisansları kaynak paketine alır. Temiz kök derleme bağımlılıklarını kurar;
asıl derleme ayrıcalıksız hesapla, dış ağa erişimi olmadan yapılır.
`.deb`, `.dsc`, `.tar.xz`, `.changes` ve `.buildinfo` dosyaları
`target/packages/asena/` altında oluşur. Özel anahtarlar bu dosyalara girmez.

Yamalı wlroots için Debian kaynak depoları açık olmalı ve
`sudo apt-get build-dep wlroots` tamamlanmış olmalıdır:

```sh
bash scripts/build-wlroots.sh
cp target/packages/wlroots/libwlroots-0.18_0.18.2-3+asena1_amd64.deb \
   target/packages/wlroots/wlroots_0.18.2-3+asena1.dsc \
   target/packages/wlroots/wlroots_0.18.2-3+asena1.debian.tar.xz \
   target/packages/wlroots/wlroots_0.18.2.orig.tar.bz2 \
   target/packages/asena/
```

Geliştirme ve örnek wlroots paketlerini arşive almak gerekmez. Çalışma zamanı
kitaplığı ve onun karşılık gelen kaynak paketi birlikte taşınır.

## Yerel arşivi kur ve dene

```sh
python3 scripts/create-apt-test-repo.py \
    --packages target/packages/asena --output target/apt-test
sudo bash scripts/test-debian-packages.sh install target/apt-test
sudo bash scripts/test-installed-apps.sh
```

İlk komut yedi günlük test anahtarıyla arşivi imzalar; katalog ve simgeleri
gerçek paketlerden üretip aynı imza zincirine ekler. Güven paketleri
`target/apt-test-trust/`, özel anahtar `target/keys/asena-test/` altındadır.
`test-installed-apps.sh`, çalışan geliştirme oturumunun ekranını kullanır;
uygulamaları temiz test kökündeki `/usr/bin` ve sistem veri yollarından açar.

Yükseltme denemesinde kaynak hazırlığı farklı bir Debian sürümüyle yeniden
yapılır, yeni paketler aynı arşive aktarılır. Ardından:

```sh
sudo bash scripts/test-debian-packages.sh upgrade target/apt-test
sudo bash scripts/test-debian-packages.sh negative target/apt-test
sudo bash scripts/test-debian-packages.sh purge target/apt-test
```

Bu betikler yalnız `target/debian-test-root` içindeki test kurulumunu değiştirir.
Gerçek kullanıcının işletim sistemi hedef değildir. Ayar, kişisel dosya ve
giriş yöneticisi yapılandırması korunumu; yanlış anahtar, imzasız arşiv,
değiştirilmiş imzalı kayıt ve bozuk katalog reddi denenir. Kanıtlar
`target/evidence/debian/` altında tutulur.

## Yayın öncesi

Yerel test sonucu ürün yayını yerine geçmez. Ürün adresi, anahtar sahipliği,
çevrimdışı yedek ve anahtar geçişi K08'de kesinleşir. Gerçek ekran görüntüleri,
yayın geçmişi, ISO bağlantısı ve dağıtımın kalan paketleri ayrıca tamamlanır.

Ürün imza anahtarının sahibi kullanıcıdır. Özel anahtar kaynak deposuna, ISO'ya
ve CI çıktısına girmez. Anahtar yedekleme ve yenileme yöntemi K08'de yazılır.
Herkese açık barındırma ve yayın yerel testten ayrı adımdır.

## Diğer dağıtımlara uygulama ulaştır

Önce Asena üzerinde ayrı Debian uygulama paketleri tamamlanır. Daha sonra
Hesap Makinesi, Notlar ve Hava Durumu gibi bağımsız uygulamalar için Flatpak
manifestleri hazırlanabilir. Sistem servislerine bağlı uygulamalar ayrıca değerlendirilir.

Mevcut site adresi `asenalinux.netlify.app` olarak kabul edilir. `org.asenalinux.*`
kimlikleri mevcut kararla korunur. Flathub kimliği, ters çevrilmiş alan adı veya
denetlenen kod barındırma hesabıyla eşleşmelidir. Mevcut uygulama kimliğinin
alan adı `asenalinux.org` olur; Netlify site adresi bu alan adıyla eşleşmez.
Flathub yayını seçildiğinde denetlenen alan adına veya GitHub hesabına uygun
kimlik ve geçiş planı ayrıca kararlaştırılır.

Kaynak: [Flathub uygulama kimliği koşulları](https://docs.flathub.org/docs/for-app-authors/requirements).
