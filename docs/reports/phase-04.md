# Faz 4 uygulama raporu

20 Ekim 2026. **4A–4C kaynakları mevcut; faz tamamlanmadı.** Genel uygulama ve dar native
backport onayı alındı. Otomatik uyku/kilit henüz etkinleştirilmedi.

## Native API ve geri dönüş

gtk4-layer-shell C 1.3.0, checksum denetimiyle `target/native/gtk4-layer-shell-1.3.0`
altına kuruldu. Debian'ın 1.0.4 paketi, GTK 4.18.6, libadwaita 1.7.6, Rust 1.85.1
ve varsayılan labwc korunuyor. 53 upstream test ve mevcut Asena panel/hızlı ayar
regresyonu geçti. Kaynak/lisans/bağımlılık gerekçesi [DEPENDENCIES](../DEPENDENCIES.md)
ve [K03](../research/session-lock-options.md) içindedir.

WHPX canlı `savevm` denemesi dirty-memory/register takibi eksikliğiyle reddedildi.
VM temiz kapatıldıktan sonra qcow2 içinde **before-phase4-auth** disk anlık
görüntüsü oluşturuldu ve `qemu-img snapshot -l` ile doğrulandı. Bu RAM görüntüsü
değildir. SSH kurtarma erişimi korunuyor. Geri yükleme yalnız VM kapalıyken:

```powershell
& ./target/vm/msys/ucrt64/bin/qemu-img.exe snapshot -a before-phase4-auth target/vm/asena-dev.qcow2
./scripts/start-dev-vm.ps1
```

Bu komutlar görüntüden sonraki VM disk değişikliklerini geri alır. Windows kaynak
dosyaları görüntünün parçası değildir. Ürün kurulumunda display manager veya PAM
politikası otomatik değiştirilmedi.

## 4A kaynakları

- Native >=1.3.0 build kapısı ve aynı GTK/GLib ailesinde session-lock binding'i.
- Compositor `locked` onayı gelmeden hazır olmayan D-Bus sözleşmesi. İkinci
  süreç mevcut kilide yönlenir; adı veya kilidi devralmaz.
- `monitor` sinyalinde her çıkışa ayrı, opak GTK kilit yüzeyi; native monitör
  yaşam döngüsü. Saat/tarih, kullanıcı/baş harf avatarı, parola, iptal ve sonuç.
  Kaydırılabilir Blueprint düzeni; etkin olmayan ekranda parola alanı gizlenir.
  Etkin düzen/Caps Lock, GTK 4.18 GdkDevice değerleri ve notify sinyalleriyle
  okunur; tercih listesinden türetilmez. [K05 incelemesi](../research/keyboard-source.md).
- GTK dışında PAM worker ve çok adımlı conversation. İptal edilen/gecikmiş cevap
  kilidi açamaz; eski worker bitmeden yenisi başlamaz. Parola D-Bus/log'a gitmez.
  Kullanıcı PAM sonunda değişmişse kilit açılmaz. Uygulamanın tuttuğu cevaplar
  zeroize ile temizlenir; GTK/native PAM geçici kopyaları ayrıca incelenecek.
- Debian common-auth/common-account kullanan, henüz normal sisteme kurulmamış
  `data/pam.d/asena-lock` örneği. Ayrıcalıklı özel daemon ve yerel unsafe FFI yok.

## Yerel sonuçlar, 20 Ekim 2026

- 4A–4B Linux kalite kapısı **44 Rust + 8 Python** testiyle geçti: build,
  Clippy `-D warnings`, fmt, çeviri, Blueprint/veri ve ShellCheck.
  [Kalite günlüğü](evidence/phase-04/quality.log).
- Link sırası incelendi: ayrı prefix'teki 1.3.0, libwayland-client'tan önce
  yükleniyor. Sistem 1.0.4 dizininin önce seçilmesi build script'inde düzeltildi.
  [Native sıra](evidence/phase-04/lock-link-order.txt).
- İki iç içe çıkışta yüzey, compositor hazır onayı, ikinci çağrı, yanlış parolanın
  reddi ve doğru parola ile açılma geçti. Kilitleyici SIGKILL ile kapatılınca
  iç içe ekran siyah kaldı; arka plan/panel görünmedi. labwc, terk edilmiş kilit
  için yeni kilitleyiciye izin veriyor; **yeni süreçte de PAM gerekliydi**.
  İlk testte “yeni kilidi reddetmeli” varsayımı yanlıştı; test compositor'ın bu
  güvenli kurtarma davranışını kapsayacak şekilde düzeltildi.
  [Kilit deneyi](evidence/phase-04/lock-probe.log).
- Aynı akış tek çıkışta Türkçe locale ile de geçti. Tarih, parola, hata ve açma
  eylemi görüntüde incelendi. Geliştirme başlatıcısının ASCII aralık kontrolü
  Türkçe locale'de `cairo` sözcüğünü reddediyordu; locale bağımsızlaştırıldı.
  Doğrudan denenen kilitleyiciye `ASENA_LOCALE_DIR` aktarımı da düzeltildi.
- Test hesabı yeniden kilitlendi, geçici parola ve test PAM dosyası kaldırıldı.
  Test günlüğünde geçici parolanın bulunmadığı denetlendi.

## Ortak kilit ve 4B kaynakları

`asenactl lock`, menü ve güç menüsü daemon'ın aynı kilit koordinatörüne gider.
Wayland socket yolu ve login kimliğinden türeyen kapsam farklı iç içe oturumları
ayırır; yanlış kapsam gerçek D-Bus deneyinde reddedildi. Program başlatmak hazır
olmak sayılmaz. Root sahipli güvenilir PAM politikası bulunmadan kilit açılmaz;
normal geliştirme önizlemesinde kilit düğmesi kapalıdır.

Uyku hazırlığı ile kilit açma aynı durum kilidiyle birbirini dışlar. Hazırlık
sırasında başlayan/eski PAM cevabı kilidi açamaz. Koordinatörün ayrı D-Bus
bağlantısı bekletmenin sahibidir; bağlantı kaybolursa yalnız logind uyanıkken
parola girişi geri gelir. Canlı ekran gücü sahibi, uyanma sinyalinde ekran geri
açılmadan bırakılmaz. logind yeniden başlarsa izleyiciler tekrar bağlanır.

Daemon tek swayidle süreç grubunu yönetir. Değişiklikte eski grup durdurulup
toplanır. Süre sırası geçersizse yardımcı durur. Karartma geri alınmadan önce
sysfs worker'dan tekrar okunur; başka aygıtın veya kullanıcının değiştirdiği
değerin üstüne yazılmaz. Ekran gücü ayrı, kalıcı Wayland bağlantısında ve sonlu
süreyle çalışır. Kısmi hata sonrası geri açma denenir; başarısızlık gizlenmez.

logind `PrepareForSleep` ve `sleep:delay` inhibitörü bağlandı. Normal oturumda
kilit hazır olmadan inhibitör bırakılmaz; logind'in sonlu süresi aşılırsa dış
uykuyu engelleme garantisi verilmez ve hata kaydedilir. Elle uyku önce kilit
onayı ister, ardından yalnız `CanSuspend=yes` durumunda `Suspend(false)` çağırır.
Kapak politikasının tek sahibi logind'dir; daemon ikinci kapak eylemi üretmez.

Son VM deneyinde tek yardımcı, ayar değişiminde eski PID'nin temizlenmesi,
geçersiz sıra reddi ve gerçek boşta kalma sonrası compositor onaylı kilit geçti.
Bekletme sırasında doğru parola kilidi açmadı; bekletmenin bırakılması ve sahibin
kopması sonrasında giriş geri geldi. Ardından normal doğru parola ve
SIGKILL/kurtarma akışı tekrar geçti. [Idle](evidence/phase-04/idle-probe.log),
[kilit/bekletme](evidence/phase-04/lock-lease-probe.log).
Geçici parola tüm deney günlüklerinde arandı; hesap tekrar `L`, test politikası
ve parola dosyası kaldırılmış olarak doğrulandı.

## 4C greeter kaynakları ve protokol deneyi

Blueprint/CompositeTemplate giriş ekranı, AccountsService'in sistem dışı
kullanıcı listesi, elle kullanıcı adı, baş harf avatarı, etkin düzen/Caps Lock,
güç yetenekleri ve oturum seçimi eklendi. Son kullanıcı adı 0600 dosyaya yazılır;
PAM mesajı, parola veya oturum geçmişi saklanmaz. Fotoğraf avatarı henüz yoktur.
Dar/kısa pencere kaydırılabilir. Asena kaydı varsa varsayılandır; yoksa labwc
seçilir. Alfabetik sıranın Wayfire'ı varsayılan yapmasına izin verilmez.

greetd_ipc 0.10.3 enum'ları, GTK dışında bounded Unix socket worker ile kullanılır.
64 KiB frame, 5 saniye bağlantı ve sonlu IO süreleri vardır. Görünür/gizli sorular,
bilgi mesajları, yanlış cevap ve iptal işlenir. Başlatma öncesi ayrı UI onay adımı
iptal yarışını kapatır; greetd başarı vermeden uygulama çıkıp oturum başlatılmış
gibi davranmaz. Seçili oturum doğrulama boyunca sabittir.

Yalnız root sahipli/yazmaya kapalı sistem Wayland kayıtları okunur. `Exec`
argümanlara ayrılır, desteklenmeyen field code reddedilir. greetd 0.10.3'ün
argümanları içeride `sh -c` için birleştirdiği kaynakta görüldü; bu sınıra her
argüman ayrı POSIX alıntıyla taşınır. Boşluk, apostrof ve `$()` gerçek `/bin/sh`
testinde literal kaldı. Ham `.desktop` metni kabuk komutu olarak gönderilmez.
[Upstream yürütme sınırı](https://raw.githubusercontent.com/kennylevinsen/greetd/0.10.3/greetd/src/session/worker.rs).

Genel kapı **51 Rust + 8 Python** testiyle geçti.
[Greeter kalite günlüğü](evidence/phase-04/greeter-quality.log).
Özel headless compositor ve **ayrıcalıksız test sunucusunda** yanlış cevap,
iptal/yeniden deneme, bilgi mesajı, seçilen argv, uygulamanın çıkışı ve yalnız
kullanıcı adı saklama geçti. [Normal](evidence/phase-04/greeter-probe.log),
[Türkçe 640×480 / 22 px yazı](evidence/phase-04/greeter-small.log).
İlk `GDK_DPI_SCALE` denemesi yazıyı büyütmediği için büyük yazı kanıtı sayılmadı;
son deneyde özel GTK kullanıcı CSS'iyle 22 px zorlandı ve görüntü incelendi.
Normal sistemin metin ölçekleme tercihi ayrıca doğrulanacak.

Test, geçici root sahipli `/usr/share/wayland-sessions/asena.desktop` kaydı
kullanır; mevcut kaydın üstüne yazmaz. Deneyden sonra kayıt kaldırıldı. Hiçbir
gerçek kullanıcı için oturum veya PAM doğrulaması yapılmadı. Native greetd/cage,
ayrı greeter hesabı ve iki kullanıcıyla TTY kabulü **henüz doğrulanmadı**.
`data/greetd/config.toml.example` etkinleştirilmedi; mevcut display manager korunur.

## Açık kabul

labwc kilitliyken `wlr-randr` değişikliğini reddetti. Bu **hotplug doğrulaması
değildir**. Gerçek TTY/DRM, monitör takma/çıkarma, TR-Q/TR-F/US ve Caps Lock,
uyku/uyanma ve shell çökmesi kabulü açık. Sentetik klavyede düzen adı yoksa
alan gizlenir; fiziksel klavye kaynağı denenmiş sayılmaz. Genel görsel kabul ve
native kimlik doğrulama tamponlarının güvenlik incelemesi de açık.

İç içe Wayland backend'inde DPMS deneyi `Unavailable` ve compositor temel çıkış
testi hatalarıyla başarısız oldu; geri açma isteği giriş yüzeylerini düzeltmedi.
Bu deney başarılı PAM/bekletme deneyinden ayrıldı. Gerçek DRM ekran kapat/aç,
karartma donanımı ve uyku/inhibitör sırası kabulü **doğrulanmadı**.
`idle-enabled=false` varsayılanı ve normal önizlemede güç eylemi kısıtı korunur.
4C gerçek giriş kabulü ve 4D polkit tamamlanmadı; Faz 4 veya M2 tamamlandı işaretlenmez.

## Sonraki adaylar

Gerçek TTY/DRM ve uyku deneyi; ardından greetd ve polkit kaynak/doğrulaması.
