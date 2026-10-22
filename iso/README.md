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
| `config/package-lists/asena-desktop.list.chroot` | Debian'dan gelen masaüstü çalışma zamanı paketleri, XanMod LTS, mikro kod, zram |
| `config/package-lists/installer.list.chroot` | Calamares için yer (6D) |
| `config/archives/xanmod.list.chroot` | Derleme sırasında XanMod deposu (K20) |
| `config/bootloaders/grub-pc/grub.cfg` | Canlı önyükleme menüsü: Asena Linux, İngilizce girdi, gelişmiş seçenekler ve araçlar |
| `config/hooks/live/0050-asena-locales.hook.chroot` | `tr_TR.UTF-8` ve `en_US.UTF-8` yerel ayarlarını üretir (K25) |
| `config/hooks/live/0100-asena-kernel-menu.hook.binary` | Menüye XanMod sürümünü yazar, yedek girdinin Debian çekirdeği olduğunu denetler |
| `config/packages.chroot/` | Derlemede `scripts/build-iso.sh` doldurur: yerel `.deb` paketleri (şimdilik yamalı wlroots) |

`scripts/build-iso.sh` derleme kopyasına ayrıca şunları ekler:

- `data/apt/keyrings/xanmod-archive-keyring.asc` ve `data/apt/preferences.d/asena-xanmod.pref`,
  `config/archives/xanmod.key.chroot` ve `xanmod.pref.chroot` olarak. live-build anahtarı yalnız
  derleme chroot'unda ve geçici olarak güvenilir sayar, chroot aşamasının sonunda kaynakla
  birlikte kaldırır.
- `config/includes.chroot_after_packages/` altına imajın sistem dosyaları: `data/apt` (XanMod
  kaynağı `Signed-By` ile, anahtar, pin) ve `data/grub` (`/etc/default/grub.d/asena.cfg`,
  `/etc/grub.d/42_asena_kernel`), `data/zram` ve `data/sysctl` (K24), canlı sisteme özgü
  live-config bileşeni `data/live/0125-asena-greetd` (K26). Faz 6A'da bu dosyaların sahibi Asena
  paketleri olur; live-config bileşeni Calamares'in kurulumda kaldırdığı canlı pakete gider.

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

`ASENA_KERNEL_TEST_VM=1 bash scripts/test-kernel-vm.sh` debootstrap ile küçük bir UEFI test
diski kurar: Debian'ın imzalı shim, GRUB ve çekirdeği, XanMod LTS ve `data/apt`, `data/grub`
dosyaları. Disk OVMF 4M secboot ile iki kez açılır. Sonuçlar
[docs/reports/evidence/k20-kernel](../docs/reports/evidence/k20-kernel):

| Senaryo | Açılan çekirdek | Secure Boot | Kesme modeli |
|---|---|---|---|
| Microsoft anahtarı yok | `6.18.58-x64v2-xanmod1` | kapalı | `full` |
| `OVMF_VARS_4M.ms.fd` | `6.12.113+deb13-amd64` | açık, çekirdek kilitli | `full` |

- Pin doğrulandı: `clang-21` için aday yok, `dkms` ve `libbpf1` Debian'dan, XanMod meta paketi 500.
- Secure Boot açıkken GRUB menüden önce iki kez "prohibited by secure boot policy" yazar ve
  açılışa devam eder. Satırlar Debian'ın standart GRUB başlığından gelir; Asena bloğu yalnız
  `if` ve `set` kullanır. Hangi komutun reddedildiği ayrıca ayrılmadı.
- Deneme iç içe QEMU'da TCG ile yapıldı; gerçek donanım ve gerçek firmware değildir.

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
| K07 | Logo, duvar kâğıdı ve marka hakları | Karar verildi; logo taslakları inceleniyor, görsel varlık eklenmedi |
| K08 | Depo barındırma, imza anahtarı | Karar verildi; barındırma açık, uzak Asena deposu yok |
| K09 | Kurulum kapsamı | Karar: amd64 ve UEFI, boş diske tek sistem; BIOS, dual boot ve şifreleme yok. Secure Boot zinciri Debian'ın imzalı paketleriyle açık, destek beyanı yok |
| K19 | Firmware politikası | Karar: `main non-free-firmware`, firmware canlı sistemde, mikro kod paketleri |
| K20 | Çekirdek | Uygulandı: XanMod LTS varsayılan, Debian imzalı çekirdeği yedek |
| K24 | Dosya sistemi ve takas | Karar: ext4; takas zram (`data/zram`, `data/sysctl`) |
| K25 | Yerel ayar ve klavye | Uygulandı: Türkçe canlı sistem, İngilizce girdi, iki yerel ayar üretilir |
| K26 | Canlı kullanıcı | Uygulandı: `asena`/`live`, greetd ile bir kez otomatik giriş (6A paketleri bekleniyor) |
| K27 | Yetki ve ağ güvenliği | Karar: root kilitli, ilk kullanıcı `sudo`; güvenlik duvarı yok, dinleyen servis yok |

## Henüz eksik olanlar

- Asena paketleri ve `asena-desktop` metapaketi (6A): shell, daemon, kilit, greeter, polkit ajanı,
  uygulamalar, şemalar, simgeler, portal ve oturum dosyaları, gtk4-layer-shell 1.3.0 backport'u.
- greetd yapılandırması ve asena-greeter paketi (6A); canlı otomatik giriş bileşeni hazır.
- Calamares yapılandırması (6D) ve Hoş geldin (6E).
