# Asena paket ve uygulama dağıtımı

Bu belge ilk GitHub yüklemesinden sonra üretilecek dağıtım dosyalarının planıdır.
Henüz Debian paket tanımları, imzalı Asena APT arşivi veya yayımlanmış paket yoktur.
Kapsam [Faz 6](../prompts/06-dagitim.md), kararlar
[PLANNING K08, K21 ve K31](../docs/PLANNING.md) ile izlenir.

## Kaynak ve yayın yerleri

| İçerik | Yer | Durum |
|---|---|---|
| Kaynak, belgeler ve sorun takibi | [agahemir1/asena-linux](https://github.com/agahemir1/asena-linux) | Depo kullanıcı tarafından açıldı; ilk yükleme hazırlanıyor |
| Proje sitesi | `https://asenalinux.netlify.app` | Mevcut Netlify sitesi |
| Asena sistem ve uygulama paketleri | Ayrı imzalı APT arşivi | Barındırma ve anahtar işlemleri açık |
| ISO ve sürüm indirmeleri | Ayrı indirme alanı | ISO yok; barındırma seçilmedi |

İlk aşamada kaynaklar tek depoda tutulur. Ayrı uygulama paketleri için ayrı Git
depoları gerekmez. Debian sistem paketleri Debian'dan, XanMod kendi sınırlı
kaynağından gelmeye devam eder. Asena arşivi kendi paketlerini ve onaylı
backport veya yamaları taşır.

## 6A için üretilecek paketler

Aşağıdaki adlar paketleme önerisidir. Dosya sahipliği ve bağımlılıklar temiz
Debian derlemesinde doğrulanmadan yayımlanmış paket adı olarak gösterilmez.

| Paket grubu | İçerik |
|---|---|
| `asena-desktop` | Varsayılan masaüstü ve uygulamaları toplayan metapaket |
| `asena-common` | Ortak şemalar, çeviriler ve çalışma zamanı verileri |
| `asena-artwork` | Simgeler, duvar kâğıtları ve lisans bildirimleri |
| Sistem bileşenleri | Shell, daemon, session, kilit, greeter ve polkit ajanı için ayrı paketler |
| Yetkili servisler | `asena-packages` ve `asena-recovery`; D-Bus, polkit ve systemd dosyalarıyla |
| Uygulamalar | Her uygulama ayrı paket; ortak source package'tan derlenebilir |
| `asena-archive-keyring` | Yalnız herkese açık Asena depo doğrulama anahtarı |
| `asena-sources` | Asena APT adresi ve `Signed-By` kaydı |
| Onaylı yerel bağımlılıklar | gtk4-layer-shell 1.3.0 backport'u ve yamalı wlroots |

`asena-live-config` yalnız canlı ortam ayarlarını taşır. Kurulum bu paketi kaldırır.
Paket kurulumu mevcut display manager'ı kendiliğinden değiştirmez.
Greeter etkinleştirmesi yönetici seçimi olarak kalır.

## Uygulamaları Yazılım'da göster

Her masaüstü uygulamasının paketine kimliğiyle eşleşen
`/usr/share/metainfo/org.asenalinux.<App>.metainfo.xml` eklenir. Metaveri
Türkçe ve İngilizce ad, özet, açıklama, lisans, sürüm bilgisi ve gerçek ekran
görüntülerini taşır. `desktop-id`, `.desktop` dosyasıyla eşleşir.

`appstream-generator`, üretilen `.deb` paketlerinden katalog ve simgeleri
oluşturur. APT arşivinin imzalı metadata zinciri bu kataloğu da kapsar.
Yazılım zaten sistem AppStream kataloğunu okur ve `asena` kökenini ayırır.
Kabul denemesinde yeni bir Asena uygulaması bulunur, kurulur, açılır,
güncellenir ve kaldırılır. Metapaket bağımlılıklarının kaldırma davranışı ayrıca denenir.

## 6B için önce yerel arşiv kur

1. Temiz Debian 13'te kilitli kaynaklardan `.deb` ve karşılık gelen kaynak paketlerini üretin.
2. `reprepro` ile yalnız amd64 hedefli izole test arşivi oluşturun.
3. Test anahtarıyla `InRelease` ve `Release.gpg` üretin. Test anahtarını ürün anahtarı olarak kullanmayın.
4. Test VM'sine keyring ve `Signed-By` kaynak kaydını kurun.
5. Önceki sürümden güncellemeyi ve kullanıcı ayarlarının korunmasını deneyin.
6. Yanlış anahtarın, imzasız veya değiştirilmiş metadata'nın reddedildiğini doğrulayın.
7. Paket ve kaynak envanterini, test sonuçlarını ve özetleri raporlayın.

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
