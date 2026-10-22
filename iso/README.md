# Asena canlı ISO (live-build)

Faz 6C'nin iskeletidir. Henüz ISO derlenmedi; Asena'nın kendi Debian paketleri (Faz 6A) ve
Calamares kurulumu (Faz 6D) gelene kadar bu dizin yalnız yapıyı ve önerilen kararları taşır.
İşletim sistemi yapısıyla ilgili her karar önce `docs/PLANNING.md` karar kaydına yazılır, sonra
buraya, `data/` veya `packaging/` altına dosya olarak girer. Geliştirme VM'sinde elle yapılan
değişiklikler ISO'ya geçmez.

## Yapı

| Yol | İş |
|---|---|
| `auto/config` | `lb config` seçenekleri: Debian 13, amd64, UEFI, firmware politikası |
| `auto/build`, `auto/clean` | live-build'in standart sarmalayıcıları |
| `config/package-lists/asena-desktop.list.chroot` | Debian'dan gelen masaüstü çalışma zamanı paketleri |
| `config/package-lists/installer.list.chroot` | Calamares için yer (6D) |
| `config/packages.chroot/` | Derlemede `scripts/build-iso.sh` doldurur: yerel `.deb` paketleri (şimdilik yamalı wlroots) |
| `config/includes.chroot/`, `config/hooks/` | Henüz yok; sistem dosyaları önce Asena paketlerine konur |

## Derleme

Geliştirme VM'sinde:

```sh
sudo apt-get install live-build
ASENA_SOURCE_COMMIT=<commit> bash scripts/build-iso.sh
```

Çıktı ve `build-info.txt` (SHA-256, kaynak commit'i, live-build sürümü, yerel paketler)
`target/iso` altında kalır. Derleme yaklaşık 10 GB yer ister.

## Açık kararlar

| Kimlik | Konu | Bu iskeletteki öneri |
|---|---|---|
| K07 | Logo, duvar kâğıdı ve marka hakları | Görsel varlık eklenmedi |
| K08 | Depo barındırma, alan adı, imza anahtarı | Yalnız yerel paketler; uzak Asena deposu yok |
| K09 | Kurulum kapsamı | amd64 ve UEFI; Secure Boot kapalı ve desteklenir denmez; BIOS, dual boot ve şifreleme yok |
| K19 | Firmware politikası | `main non-free-firmware`, firmware canlı sistemde |

## Henüz eksik olanlar

- Asena paketleri ve `asena-desktop` metapaketi (6A): shell, daemon, kilit, greeter, polkit ajanı,
  uygulamalar, şemalar, simgeler, portal ve oturum dosyaları, gtk4-layer-shell 1.3.0 backport'u.
- greetd'nin varsayılan giriş yöneticisi olarak etkinleştirilmesi ve canlı oturum kullanıcısı.
- Calamares yapılandırması (6D) ve Hoş geldin (6E).
