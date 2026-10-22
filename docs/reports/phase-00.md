# Faz 0 kabul raporu

Güncelleme: 19 Ekim 2026. Durum: **🟨 sürüyor**.
19 Ekim ek yönlendirmesi diğer fazların sıralı uygulamasına da izin verdi;
bu rapor yalnız Faz 0 kabulünü izler.
Kullanıcı Faz 0 uygulamasını, T0 bitmeden ilerlemeyi ve ek mesajla QEMU üzerinde
Debian kurulmasını açıkça onayladı. Nihai simgeler, tüm ekranlar ve compositor
değişikliği bu kapsamda uygulanmadı.

## Oluşturulanlar

- 14 üyeli Cargo workspace; GPL-3.0-or-later, MSRV 1.85.1 ve ortak lint'ler.
- `asena-core`: Türkçe/aksan normalleştirme, gettext, journal/stderr günlük,
  güvenli GSettings şema yükleme, anlamsal panel ve menü modeli, IPC kimlikleri.
- `asena-shell`: monitör listesi ve hotplug dinleyicisi, monitör başına 48 px
  exclusive-zone panel, klavye odağı almayan yüzey ve yalnız Asena etiketi.
  CSS GResource ile gömülür; opak uyumluluk malzemesidir.
- `asena-session`: Cargo giriş noktası ve gömülü POSIX oturum betiği.
  Geliştirmede özel D-Bus/süreç grubu, geçici config ve gerçek oturuma ortam
  aktarmayan başlatma/temizleme yolu.
- Sonraki fazlar için kütüphane ve binary iskeletleri. Uygulanmayan binary'ler
  başarı veya güvenli kilit izlenimi vermez; çevrilebilir hata döndürür.
- labwc, systemd user target/service, desktop session, portal ve GSettings verileri;
  kurulum, derleme/veri, kalite, çeviri ve kanıt toplama betikleri; Debian CI.
- Altı Rust mantık testi ve altı Linux oturum sözleşmesi testi yazıldı.
  Yazılmış test ile çalıştırılmış test ayrı tutulur.

## Ortam ve doğrulama

İlk ortam Windows/PowerShell'dir. `wsl --list --verbose` WSL'nin kurulu olmadığını
bildirdi. Docker/Podman/QEMU PATH'te bulunmadı. VirtualBox'taki tek VM `x`, kapalı
Windows XP sistemidir; bu VM değiştirilmedi. Kullanıcının ek izniyle QEMU ve
Debian 13 bulut imajı hazırlığına geçildi. VM kuruldu; aşağıdaki sonuçlar bu Linux konuğunda üretildi.

19 Ekim VM sonucu: QEMU 11.1.2 (MSYS2 paket özetleri doğrulandı), Debian 13.7
genericcloud amd64 (Debian SHA-512 doğrulandı), 6 GB RAM, 4 vCPU ve 32 GB ayrı
QCOW2 overlay. Cloud-init tamamlandı; SSH ve konuk kimliği doğrulandı.
Geliştirme bağımlılıkları temiz Debian sisteminde kuruldu. Cargo.lock Debian Cargo ile üretildi; build/fmt/clippy/test ve veri kontrolleri geçti. Kullanıcı GitHub Actions yerine
şimdilik yerel Linux eşdeğer kontrolleriyle devam edilmesini seçti.

| Kontrol | Gerçek sonuç |
|---|---|
| Taşınabilir `python scripts/check-source.py` | Geçti: 14 crate, 21 Rust dosyası, TOML/XML/GResource/POTFILES/LF |
| Literal gettext metinlerinin Türkçe katalog kapsamı | Geçti: 8 ileti, eksik yok; GNU extractor doğrulaması yerine geçmez |
| Python kaynaklarının AST sözdizimi | Geçti; Linux oturum testleri çalıştırılmış sayılmaz |
| Resmî Debian paket indeksi | İncelendi, tüm istenen paketler mevcut; kurulu APT kanıtı değil |
| Doğrudan crate manifestleri/lisans ve bakım metadata'sı | İncelendi; geçişli Cargo çözümlemesi ve Linux derlemesi ayrı |
| Temiz Debian setup + cargo build | Geçti: Debian 13.7, rustc/cargo 1.85.1 |
| cargo fmt --check | Geçti |
| cargo clippy -D warnings | Geçti: workspace ve all-targets |
| cargo test --workspace | Geçti: 6 Rust testi |
| ShellCheck / msgfmt / extractor / desktop / XML Linux araçları | Geçti; Türkçe gerçek locale altında gettext çıktısı da doğrulandı |
| Linux oturum sözleşmesi testleri | Geçti: 6 test |
| Gerçek iç içe panel / exclusive zone / çoklu monitör | İki sanal çıkışta 48 px panel; büyütülmüş foot panelin üstünde bitti. %200 ölçekte panel 96 fiziksel px. Fiziksel monitör denenmedi |
| wayland-info ve oturum temizliği | Protokol envanteri alındı. İç içe oturum kapanınca shell/session kalmadı, gerçek user manager ortamı değişmedi. Normal systemd oturumu denenmedi |
| GitHub Actions koşusu | Çalıştırılmadı; bu klasörde Git deposu/remote yok |

Windows üzerinde Rust derlemesi veya uygulama testi yapılmadı.

## Debian ve API bulguları

[Tam paket kaydı](../research/debian13-packages.json) ve
[mimari sürüm tablosu](../ARCHITECTURE.md). İndeks: trixie/main amd64.
GTK 4.18.6, libadwaita 1.7.6, layer-shell 1.0.4, labwc 0.8.3 ve Rust 1.85.1
tabanı manifestlere işlendi. GTK/libadwaita üst API feature'ı eklenmedi.
Varsayılan uygulamalar ve yedeklerinin tüm paket adları mevcut;
APT aday sürümleri kanıt dosyasında toplandı; bütün varsayılan uygulamalar kurulup çalıştırılmış sayılmaz.

Doğrudan crate lisansları kaydedildi, gerekçeler [DEPENDENCIES.md](../DEPENDENCIES.md)
içindedir. `Cargo.lock` Linux üzerinde üretildi ve derlendi. Geçişli lisans/bildirimlerin dağıtım denetimi açıktır; elle lock/checksum uydurulmadı.

## Protokol, blur ve kilit

[Araştırma](../research/blur-and-lock.md), kaynak ön envanteri ile çalışma zamanı
kanıtını ayırır. labwc 0.8.3'te layer-shell, foreign-toplevel, lock, idle, output,
gamma, screencopy, wlr-data-control ve ext-workspace yolları mevcut. İncelenen
etikette ext-image-copy-capture ve ext-data-control oluşturma yolu bulunmadı.
Çalışma zamanı globals/sürümler `wayland-info` ile doğrulandı; mimari tablosuna işlendi.

GTK session-lock API'si C 1.1.0 ile geliyor; Debian C 1.0.4'te yok. Faz 4 için
binding/backport ve güvenlik düzeltmeleri kararı açıktır. Toplevel listesi pencere
geometrisinin kanıtı değildir; K10 da açık kalır.

Blur My Shell GNOME'un çizim altyapısında çalışır. Asena'da benzer gerçek blur
compositor render ve GTK yüzey entegrasyonu gerektirir. labwc PR #3751, API
incelemesinde birleştirilmeden kapalıdır. Deneysel yama veya yeni compositor
kurulmadı; K13 tamamlanmadı. Opak CSS blur hedefini tamamlanmış saydırmaz.

## Kanıtlar ve sınırlar

- [Ortam ve APT](evidence/phase-00/environment.txt), [kalite çıktısı](evidence/phase-00/quality.log), [Wayland envanteri](evidence/phase-00/protocols.txt).
- Görüntüler yerelde `target/vm/panel-maximized.png` ve `panel-scale-2.png` altında.
- İç içe Wayland arka ucu `wlr-randr --output WL-1 --off` isteğini reddetti. Bu deneme monitör çıkarma başarısı sayılmaz; gerçek hotplug açık.
- Normal kurulu systemd oturumu, fiziksel GPU/monitörler, portal/Flatpak ve dış CI koşusu doğrulanmadı. Kullanıcı dış CI yerine şimdilik yerel kontrolleri seçti.
- Debian gettext 0.23.1 Rust dilini tanımıyor. Extractor C string/keyword taraması ve ayrı GSettings taraması kullanıyor; Rust raw string ve attribute içindeki gettext bu yöntemle desteklenmez. Hata metinleri normal Rust çağrılarıdır.
- İlk derlemede bulunan namespace/closure tür hataları, geçersiz desktop anahtarı ve panelin yalnız etiket genişliğine daralması giderildi. Son Linux kalite koşusu geçti.

## Açık işler ve sonraki faz

1. Gerçek monitör ekle/çıkar ve normal systemd giriş/çıkış denemeleri.
2. Kullanıcı istediğinde GitHub Actions koşusu. Şu an çalıştırılmadı; Faz 0 tüm özgün kabul maddeleri için ✅ değil.
3. K13 gerçek blur yolu, K14 tema/simge/imleç ve ilk dört simgenin incelemesi açık.
4. Kullanıcının diğer fazlara devam yönlendirmesiyle Faz 1'in compositor bağımsız veri/IPC altyapısına geçildi. Üretim cam malzemesi veya güvenlik/API kararları tamamlanmış sayılmaz.
