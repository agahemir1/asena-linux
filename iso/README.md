# Asena canlı ISO (live-build)

Faz 6C'nin iskeletidir. Henüz ISO derlenmedi; Asena'nın kendi Debian paketleri (Faz 6A) ve
Calamares kurulumu (Faz 6D) gelene kadar bu dizin yalnız yapıyı ve önerilen kararları taşır.
İşletim sistemi yapısıyla ilgili her karar önce `docs/PLANNING.md` karar kaydına yazılır, sonra
buraya, `data/` veya `packaging/` altına dosya olarak girer. Geliştirme VM'sinde elle yapılan
değişiklikler ISO'ya geçmez.

## Yapı

| Yol | İş |
|---|---|
| `auto/config` | `lb config` seçenekleri: Debian 13, amd64, UEFI, Debian'ın imzalı shim ve GRUB'u, firmware politikası, canlı çekirdek satırı (dil, klavye, saat dilimi, kullanıcı) |
| `auto/build`, `auto/clean` | live-build'in standart sarmalayıcıları |
| `config/package-lists/asena-desktop.list.chroot` | Debian'dan gelen masaüstü çalışma zamanı paketleri, XanMod LTS, mikro kod, zram, `appstream` ve `gnome-keyring` (K21, K22) |
| `config/package-lists/installer.list.chroot` | Calamares için yer (6D) |
| `config/archives/xanmod.list.chroot` | Derleme sırasında XanMod deposu (K20) |
| `config/bootloaders/grub-pc/grub.cfg` | Canlı önyükleme menüsü: Asena Linux, İngilizce girdi, gelişmiş seçenekler ve araçlar |
| `config/bootloaders/splash.svg`, `grub-pc/live-theme/theme.txt` | Canlı menünün zemini (logo ve ad, `scripts/icons/brand.py` çizer) ve teması (K07) |
| `config/hooks/live/0050-asena-locales.hook.chroot` | `tr_TR.UTF-8` ve `en_US.UTF-8` yerel ayarlarını üretir (K25) |
| `config/hooks/live/0060-asena-identity.hook.chroot` | Debian'ın os-release dosyasını yönlendirip Asena'nınkini koyar (K23) |
| `config/hooks/live/0070-asena-plymouth.hook.chroot` | Asena açılış ekranını seçer ve initramfs'leri yeniden üretir (K07) |
| `config/hooks/live/0100-asena-kernel-menu.hook.binary` | Menüye XanMod sürümünü yazar, yedek girdinin Debian çekirdeği olduğunu denetler |
| `config/packages.chroot/` | Derlemede `scripts/build-iso.sh` doldurur: yerel `.deb` paketleri (şimdilik yamalı wlroots) |

`scripts/build-iso.sh` derleme kopyasına ayrıca şunları ekler:

- `data/apt/keyrings/xanmod-archive-keyring.asc` ve `data/apt/preferences.d/asena-xanmod.pref`,
  `config/archives/xanmod.key.chroot` ve `xanmod.pref.chroot` olarak. live-build anahtarı yalnız
  derleme chroot'unda ve geçici olarak güvenilir sayar, chroot aşamasının sonunda kaynakla
  birlikte kaldırır.
- `config/includes.chroot_after_packages/` altına `scripts/stage-system-files.sh` ile imajın sistem
  dosyaları: XanMod kaynağı, anahtarı ve pin'i, Debian kataloğunun uygulama simgeleri (`data/apt`), GRUB ayarları (`data/grub`), zram
  (`data/zram`, `data/sysctl`), canlı sisteme özgü live-config bileşeni (`data/live`), os-release,
  `/etc/issue`, fastfetch yapılandırması ve logosu, `asena-logo` simgeleri, Plymouth teması
  (`data/branding`, `data/fastfetch`, `data/icons`, `data/plymouth`). live-build bunları rsync ile
  root sahipliğinde kopyalar. Faz 6A'da bu dosyaların sahibi Asena paketleri olur; live-config
  bileşeni Calamares'in kurulumda kaldırdığı canlı pakete gider.

## Derleme

Geliştirme VM'sinde:

```sh
sudo apt-get install live-build
ASENA_SOURCE_COMMIT=<commit> bash scripts/build-iso.sh
```

Çıktı ve `build-info.txt` (SHA-256, kaynak commit'i, live-build sürümü, yerel paketler)
`target/iso` altında kalır. Derleme yaklaşık 10 GB yer ister.

## Çekirdek (K20)

Varsayılan çekirdek XanMod LTS (`linux-xanmod-lts-x64v2`), yanında Debian'ın imzalı
`linux-image-amd64` çekirdeği kurulu kalır.

- **Depo:** `deb.xanmod.org` yalnız çekirdek paketlerini sağlayabilir. Pin, depodaki clang/LLVM,
  dkms, libbpf ve diğer paketlerin Debian'ınkilerin yerine geçmesini engeller. Anahtar parmak izi
  `D38D 7D1D A134 9567 ADED 882D 86F7 D09E E734 E623` (XanMod Kernel, 22 Ekim 2026'da
  `dl.xanmod.org/archive.key` adresinden alındı; XanMod parmak izini ayrıca yayımlamıyor).
- **Kurulu sistem:** `42_asena_kernel`, `update-grub` sırasında Debian'ın en yeni `-amd64`
  çekirdeğini bulur. GRUB Secure Boot nedeniyle kilitli moddaysa (`lockdown=y`) varsayılan girdi
  bu çekirdektir; aynı girdi `fallback` olarak da yazılır. `asena.cfg` çekirdek satırına
  `preempt=full` ekler.
- **Canlı ISO:** `config/bootloaders/grub-pc/grub.cfg` çekirdeği `lockdown` değişkenine göre
  seçer: kilitli modda Debian'ın imzalı çekirdeği, değilse XanMod. XanMod yüklenemezse `fallback`
  gelişmiş seçeneklerin ilk girdisine, Debian çekirdeğine geçer.

### VM denemesi (23 Ekim 2026)

`ASENA_BOOT_TEST_VM=1 bash scripts/test-boot-vm.sh` debootstrap ile küçük bir UEFI test diski
kurar. Diskte Debian'ın imzalı shim, GRUB ve çekirdeği, XanMod LTS ve imajın sistem dosyaları
vardır; dosyalar `scripts/stage-system-files.sh` ile yerleştirilir ve imajın chroot kancaları
çalıştırılır. Disk OVMF 4M secboot ile iki kez açılır, açılış sırasında ekran görüntüsü alınır.
Sonuçlar [docs/reports/evidence/boot-test](../docs/reports/evidence/boot-test):

| Senaryo | Açılan çekirdek | Secure Boot | Kesme modeli | Takas |
|---|---|---|---|---|
| Microsoft anahtarı yok | `6.18.58-x64v2-xanmod1` | kapalı | `full` | `/dev/zram0` 963 MB |
| `OVMF_VARS_4M.ms.fd` | `6.12.113+deb13-amd64` | açık, çekirdek kilitli | `full` | `/dev/zram0` 966 MB |

- Pin doğrulandı: `clang-21` için aday yok, `dkms` ve `libbpf1` Debian'dan, XanMod meta paketi 500.
- Secure Boot açıkken GRUB menüden önce iki kez "prohibited by secure boot policy" yazar ve
  açılışa devam eder. Satırlar Debian'ın standart GRUB başlığından gelir; Asena bloğu yalnız
  `if` ve `set` kullanır. Hangi komutun reddedildiği ayrıca ayrılmadı.
- Deneme iç içe QEMU'da TCG ile yapıldı; gerçek donanım ve gerçek firmware değildir.

`ASENA_BOOT_TEST_OFFLINE=1` ile aynı disk yeniden başlatmada güncellemeyi de dener (K28): paket
servisinin çevrimdışı modu ve `asena-offline-update.service` diske kurulur, güncellemeler indirilir
ve `/system-update` işareti konur. Bekleyen Debian güncellemeleri (7 paket, tzdata 2026c → 2026e)
kuruldu ve sonuç `success: true` yazıldı. Postinst'i hata veren bir test paketinde sistem yine açıldı,
paket `half-configured` kaldı ve sonuç `success: false` ile APT'nin hata satırını taşıdı.

## Markalama ve sistem kimliği (K07, K23)

Logo, kullanıcının 23 Ekim'de seçtiği çini plaka taslağıdır; kaynağı `scripts/icons/brand.py`.

| Yer | Dosya | Durum |
|---|---|---|
| os-release, `lsb_release` | `data/branding/os-release`, `0060-asena-identity` | VM: "Asena Linux 1.0 RC", `ID=asena`, `ID_LIKE=debian`; Debian'ın dosyası `dpkg-divert` ile yönlendirildi |
| GRUB menüsü | `data/grub/default/asena.cfg` | VM: "Asena GNU/Linux"; EFI dizini `debian` kaldı |
| Canlı GRUB menüsü | `config/bootloaders/splash.svg`, `live-theme/theme.txt` | Görsel üretildi; ISO derlenmedi |
| Plymouth | `data/plymouth/asena/` | VM: açılışta logo ve sekiz noktalı gösterge görüldü |
| Konsol girişi | `data/branding/issue`, `issue.net` | VM: dosya yerinde |
| fastfetch | `data/fastfetch/config.jsonc`, `data/branding/asena-logo.txt` | VM: Asena logosu ve "OS: Asena Linux 1.0 RC" |
| Simge | `asena-logo`, `asena-logo-symbolic` | Ayarlar › Hakkında, giriş ve kilit ekranı kullanır |

`grub-install` EFI dizinini `GRUB_DISTRIBUTOR` değerinden türetir. Debian'ın imzalı GRUB'u ise
yapılandırmayı yalnız `EFI/debian/grub.cfg` içinde arar. Asena os-release'iyle dizin `EFI/asena`
olurdu ve ilk GRUB paketi güncellemesinden sonra sistem açılmazdı (VM'de görüldü). Bu yüzden
`asena.cfg` değeri yalnız `grub-mkconfig` için Asena yapar. Calamares (6D) `--bootloader-id debian`
kullanmalıdır.

### Doğrulanmadı

- Canlı ISO menüsü yalnız live-build'in yer tutucu değişimini taklit eden örnek bir `binary/`
  ağacında denendi (`grub-script-check` geçti, yanlış yedek girdide kanca derlemeyi durdurdu); ISO
  derlenmedi.
- Gerçek donanımda Secure Boot. Microsoft'un 2011 sertifikalarının yerini 2023 sertifikalarına
  bırakan firmware'lerde Debian shim'inin durumu.
- x86-64-v2 desteklemeyen işlemciler (2009 öncesi) XanMod'u açamaz ve bu bir yükleme hatası
  olmadığı için GRUB yedeğe geçmez; Debian girdisi menüden elle seçilir.
- XanMod deposu kaynak paketi yayımlamıyor (`main/source/Sources` boş). ISO dağıtıldığında
  GPL gereği XanMod'un aynı sürümünün kaynağı ISO ile birlikte sunulmalı (6C/6E).

## Kararlar

| Kimlik | Konu | Bu iskeletteki durum |
|---|---|---|
| K07 | Logo, duvar kâğıdı ve marka hakları | Uygulandı: logo, GRUB, canlı menü, Plymouth, fastfetch, konsol; Calamares ve Hoş geldin 6D/6E'de |
| K08 | Depo barındırma, imza anahtarı | Karar verildi; barındırma açık, uzak Asena deposu yok |
| K09 | Kurulum kapsamı | Karar: amd64 ve UEFI, boş diske tek sistem; BIOS, dual boot ve şifreleme yok. Secure Boot zinciri Debian'ın imzalı paketleriyle açık, destek beyanı yok |
| K19 | Firmware politikası | Karar: `main non-free-firmware`, firmware canlı sistemde, mikro kod paketleri |
| K20 | Çekirdek | Uygulandı: XanMod LTS varsayılan, Debian imzalı çekirdeği yedek |
| K21 | Yazılım ve paket servisi | Uygulandı: `appstream` ve simge hedefleri imajda; servis, polkit ve D-Bus dosyaları 6A paketini bekliyor |
| K22 | Flatpak | Uygulandı: `gnome-keyring`; Flathub Yazılım'da açık seçimle |
| K23 | Sistem kimliği | Uygulandı: Asena os-release, `ID_LIKE=debian`, `VERSION_CODENAME=trixie` |
| K24 | Dosya sistemi ve takas | Uygulandı: zram VM açılışında etkin; ext4 bölümlemesi ve K30 kurtarma bölümü Calamares'te (6D) |
| K25 | Yerel ayar ve klavye | Uygulandı: Türkçe canlı sistem, İngilizce girdi, iki yerel ayar üretilir |
| K26 | Canlı kullanıcı | Uygulandı: `asena`/`live`, greetd ile bir kez otomatik giriş (6A paketleri bekleniyor) |
| K27 | Yetki ve ağ güvenliği | Karar: root kilitli, ilk kullanıcı `sudo`; güvenlik duvarı yok, dinleyen servis yok |
| K28 | Güncelleme politikası | Uygulandı: `unattended-upgrades` yok, yeniden başlatmada güncelleme (aşağıda) |
| K30 | Sıfırlama ve kurtarma | Servis/işçi/GRUB/kurulum sonu aracı kaynakta; Calamares bağlantısı ve gerçek ISO kabulü açık (6D) |
| K31 | Asena uygulama kapsamı | Karar: 16 Asena uygulaması varsayılan kurulumda; paket listesi 6A paketlerini bekliyor |

## Henüz eksik olanlar

- Asena paketleri ve `asena-desktop` metapaketi (6A): shell, daemon, kilit, greeter, polkit ajanı,
  uygulamalar, şemalar, simgeler, portal ve oturum dosyaları, gtk4-layer-shell 1.3.0 backport'u;
  paket servisi `asena-packages` (`/usr/libexec`), polkit politikası, sistem D-Bus dosyaları ve
  `asena-packages.service` ile `asena-offline-update.service` (etkin).
- greetd yapılandırması ve asena-greeter paketi (6A); canlı otomatik giriş bileşeni hazır.
- Calamares yapılandırması (6D) ve Hoş geldin (6E).

## Kurtarma kurulumu (K30)

Çalışan kaynaklar ve kurulum sonu çağrısı [RECOVERY.md](../docs/architecture/RECOVERY.md) içindedir. `scripts/install-recovery.sh` bağlı ext4 kök/kurtarma bölümleri ve canlı ortamı alır, imajı ve UUID/özet manifestini kurar. Calamares henüz bu çağrıyı ve bölüm düzenini uygulamaz; ISO kurulum/kurtarma kabulü bu yüzden açıktır. İmaj için gereken alan ISO üretildikten sonra ölçülür.
