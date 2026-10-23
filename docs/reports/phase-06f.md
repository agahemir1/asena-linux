# Faz 6F raporu: Yazılım ve paket arka ucu

23 Ekim 2026. Plan: [06f-yazilim.md](../../prompts/06f-yazilim.md). Kararlar K21, K22 ve K28
([PLANNING](../PLANNING.md)). Sözleşmeler [ARCHITECTURE](../ARCHITECTURE.md) içindedir.

## Karar kapıları

| Kapı | Sonuç |
|---|---|
| `rust-apt` Debian 13 ile | 0.11.3, libapt-pkg 3.0.3 ve rustc 1.85.1 ile VM'de derlendi. `cxx` 1.0.198'de kalır; sonraki sürümler rustc 1.88 ister |
| `libflatpak`, `libappstream` | 0.6.0 ve 0.3.0, MIT, glib 0.20. Ekran görüntüleri için `soup3` 0.7.0 (MIT) eklendi ([DEPENDENCIES](../DEPENDENCIES.md)) |
| Otomatik güvenlik güncellemeleri | `unattended-upgrades` kurulmaz; güncellemeler arka planda iner, yeniden başlatmada kurulur (K28) |
| Asena deposu | 6B'de yayımlanır; o zamana kadar katalogda Asena kaynağı yok |

## Yapılanlar

- **6F-A paket servisi:** `crates/asena-packages`, `asena_core::packages` sözleşmesi, polkit politikası,
  sistem D-Bus ve systemd dosyaları. Ad doğrulama, korunan paketler, sahibine bağlı ve bayatlayınca
  reddedilen planlar, tek seferde tek APT işi, etkileşimsiz dpkg, yarım kalan dpkg işinin
  algılanması ve yetkili onarımı.
- **6F-B katalog:** Debian DEP-11 ve her Flatpak uzak deposu ayrı okunur, aynı uygulama tek girdide
  birleşir. Debian'ın uygulama simgeleri `data/apt/apt.conf.d/50asena-appstream-icons` ile iner.
  Arama `fold` ile, listeler yerel ayara göre sıralı. Ekran görüntüleri ayrıntı sayfasında iner.
- **6F-C Flatpak (K22):** kullanıcı kurulumu, kurulum öncesi izin özeti, Flathub'ı açık seçimle ekleme
  ve ağ hatasında yeniden deneme; kurulum, kaldırma ve güncelleme sürerken İptal düğmesi. `gnome-keyring`
  (Secret), daemon'da Inhibit ve Background arka uçları.
- **6F-D güncellemeler:** daemon'un günlük denetimi ve indirmesi, tek bildirim, yeniden başlatmada
  kurulum (`asena-offline-update.service`), sonucun bir sonraki oturumda gösterilmesi. Flatpak
  uygulama ve çalışma zamanı güncellemeleri oturum içinde.
- **6F-E ön yüz:** Keşfet, kategori, arama, Kurulu, Güncellemeler ve ayrıntı sayfaları; dar pencerede
  tek sütun. Yazılım menüde görünür (`NoDisplay` kaldırıldı); `asena-store --updates` Güncellemeler'i açar.

## VM denemeleri

Ortam: Debian 13 geliştirme VM'i (QEMU, WHPX), oturum `scripts/preview-desktop.sh` ile headless Wayfire,
llvmpipe. Paket servisi denemelerde `scripts/test-packages-vm.sh` ile geçici olarak kurulur ve sonunda
kaldırılır.

### Paket servisi ([kanıt](evidence/packages))

`ASENA_PACKAGES_TEST_VM=1 bash scripts/test-packages-vm.sh`:

- Kuralsız ve ajansız çağıran `Refresh` için `NotAuthorized` aldı. `-o` ve `hello world` adları
  `InvalidName`, `sudo` kaldırma planı `Protected`, olmayan paket `UnknownPackage` ile reddedildi.
  Başka bir bağlantının planı `StalePlan` ile reddedildi.
- Yetkiyle `hello` 2.10-5 kuruldu ve kaldırıldı; güncelleme listesi `tzdata` 2026c → 2026e gösterdi;
  `ScheduleUpgrade` `/system-update` bağını kurdu, `CancelUpgrade` kaldırdı.
- Onarım: postinst'i başarısız olan test paketi `half-configured` kaldı, `NeedsRepair` doğru oldu;
  neden ortadan kalkınca `Repair` paketi `installed` yaptı ve `NeedsRepair` yanlışa döndü.

### Yeniden başlatmada güncelleme ([kanıt](evidence/boot-test))

`ASENA_BOOT_TEST_VM=1 ASENA_BOOT_TEST_OFFLINE=1 bash scripts/test-boot-vm.sh` önyükleme testinin
diskine servisin çevrimdışı modunu ve `asena-offline-update.service` birimini kurar, güncellemeleri
indirip `/system-update` işaretini koyar ve diski iki kez açar.

| Senaryo | Sonuç dosyası | Paketler | dpkg durumu |
|---|---|---|---|
| Bekleyen Debian güncellemeleri | `success: true`, 7 paket | tzdata 2026c → 2026e, openssl, ca-certificates … | temiz |
| Test paketinin 1.1 sürümü, postinst `exit 1` | `success: false`, "E: Sub-process /usr/bin/dpkg returned an error code (1)" | asena-offline-test 1.1 | `half-configured`; sistem açıldı |

İki güncelleme açılışında da birim `system-update.target` içinde çalıştı, işareti sildi, güncellemeleri
kurdu, makineyi yeniden başlattı ve "Deactivated successfully" ile bitti; sonraki açılış normal hedefe
girdi. Birimin günlüğü `offline-journal.txt` içindedir; betik günlükte birim hatası görürse başarısız sayar.

Denemenin bulduğu dört hata düzeltildi:

- İlk denemede birim hiç başlamadı: `Requires=dbus.socket` test diskinde karşılanmadı, systemd
  işareti silip yeniden başlattı. Çevrimdışı mod D-Bus kullanmaz, bağımlılık kaldırıldı.
- `systemctl reboot` önce logind'e D-Bus üzerinden ulaşmaya çalıştı ve günlüğe hata yazdı; bu açılışta
  D-Bus başlamaz. Yeniden başlatma artık doğrudan systemd'ye iş olarak verilir (`reboot.target`).
- systemd, istenen yeniden başlatma sırasında servisi SIGTERM ile durdurduğu için başarılı güncellemede
  bile günlüğe "Failed with result 'signal'" düşüyordu; birim bu sinyali başarı sayar
  (`SuccessExitStatus=SIGTERM`).
- Hata metni Rust'ın iç gösterimiyle (`Apt("…\n")`) kaydediliyordu; artık APT'nin satırıdır.

Gerçek kullanıcı oturumunda sonuç bildirimi denenmedi: geliştirme oturumunda (`ASENA_DEV`) daemon'un
güncelleme modülü çalışmaz.

### Yazılım ([ekran görüntüleri](evidence/store))

- Katalog Debian ve Flathub uygulamalarını simgeleriyle gösterdi (`category.jpg`); arama ve ayrıntı
  sayfası (`app-page.jpg`) çalıştı.
- Debian'dan `freesweep` kuruldu ve kaldırıldı; önce değişiklik özeti çıktı (`plan-dialog.jpg`),
  Kurulu sayfasında göründü (`installed.jpg`).
- Flathub'dan Tetravex kullanıcı kurulumu olarak kuruldu, açıldı ve kaldırıldı; kurulumdan önce
  izinler gösterildi (`flatpak-permissions.jpg`).
- Güncellemeler sayfası sistem güncellemesini yeniden başlatmaya zamanladı ve zamanlamayı iptal etti
  (`updates-scheduled.jpg`); Türkçe arayüz (`updates-tr.jpg`).

- Flatpak güncellemesi: VM'de Flathub'da GNOME Platform 51 için yeni bir commit vardı. Yazılım önce
  yalnız uygulamaları listelediği için bu güncelleme hiç görünmüyordu; çalışma zamanlarının kendi
  güvenlik düzeltmeleri de uygulanmazdı. Düzeltmeden sonra Güncellemeler sayfasında çıktı
  (`flatpak-runtime-update.jpg`; üstteki satır hata biçimi düzeltilmeden önce alındı). Güncelle
  düğmesi iş sürerken İptal'e döndü; iptal hata göstermeden durdu ve güncelleme listede kaldı
  (`flatpak-update-cancel.jpg`). İkinci denemede çalışma zamanı `28e84823…` commit'inden
  `843e7281…` commit'ine geçti (`flatpak-updated.jpg`). Bekleyen bir uygulama güncellemesi
  olmadığından güncelleme bir uygulamada denenmedi.
- Başarısız çevrimdışı güncellemenin sonucu, önyükleme testindeki dosyanın VM'e geçici olarak konan
  kopyasıyla Güncellemeler sayfasında gösterildi (`last-upgrade-failed.jpg`); dosya sonra silindi.
- Paket servisi kurulu değilken Yazılım çökmedi, Güncellemeler sayfası "Paket servisine
  ulaşılamıyor" dedi (`no-service.jpg`).
- 23 Ekim 09:08 UTC'de VM'de web sitesi çekimi için ikinci bir oturum açıktı. Oturuma komut
  çalıştıran yardımcı en yeni daemon'u seçtiği için `target/debug/asena-store --updates` bir kez o
  oturumda açıldı ve kapatıldı. [website.md](website.md) içindeki "Yazılım oturum açılışında
  kendiliğinden açıldı" maddesi büyük olasılıkla budur; denemeler sonra yalnız `asena-preview`
  oturumunu hedefleyen bir yardımcıyla sürdü.
- "görüntü" araması özetinde "Görüntüleri…" geçen uygulamaları buldu; eşit sıradaki sonuçlar büyük ve
  küçük harften bağımsız, yerel ayara göre dizildi (`search-tr.jpg`).

### Portallar

VM oturumunda `xdg-desktop-portal` Background, Inhibit ve Secret arayüzlerini ilan etti. Boşta
kalma engeli daemon'un zamanlayıcısını durdurdu, `Close` ve çağıranın çıkışı engeli kaldırdı;
çağıran istek tamamlanmadan çıktığında engel hemen bırakıldı.

## Kabul kriterleri

| Kriter | Durum |
|---|---|
| Yetkisiz çağıran kur, kaldır ve güncelle yapamaz; `auth_admin_keep` süresinde tekrar sorulmaz | Kuralsız çağıran reddedildi. Denemeler geçici bir polkit kuralıyla yapıldı; gerçek ajanla parola ve tekrar sorulmama denenmedi |
| Korunan paket ve seçenek benzeri ad reddedilir | Geçti |
| Debian'dan küçük uygulama kurulur ve kaldırılır; yarıda kalan işlem onarılır | Geçti (`hello`, `freesweep`, onarım senaryosu) |
| Flatpak kullanıcı kurulumu: kur, aç, güncelle, kaldır | Tetravex kuruldu, açıldı, kaldırıldı; güncelleme çalışma zamanında denendi |
| Yeniden başlatmada güncelleme; başarısız pakette sistem açılır ve sonuç gösterilir | Geçti; sonuç Yazılım'da gösterildi, daemon bildirimi gerçek oturumda denenmedi |
| Türkçe arama ve Türkçe sıralama | Birim testleri ve arayüzde arama geçti |
| Ana iş parçacığı bloklanmaz; servis yokken çökme yok | Bütün APT, Flatpak ve dosya işleri `gio::spawn_blocking` veya D-Bus üzerinden; servis yokken durum gösterildi |
| Çeviri ve belgeler | `po/tr.po` eksiksiz; ARCHITECTURE, APP_CATALOG, APP_ROADMAP, DEPENDENCIES, PLANNING güncel |

## Doğrulanamayanlar

- logind askıya alma engeli: SSH ile açılan önizleme oturumu etkin bir koltuk oturumu olmadığından
  logind "Interactive authentication required" döndü. Normal girişte denenecek.
- Gerçek ağ kesintisi ve ölçülü bağlantı; gerçek donanımda yeniden başlatmada güncelleme.
- Asena simge temasının ve adw-gtk3'ün Flatpak uygulamalarında çizimi (6A paketlerini bekliyor;
  yol K22'de).
- Paket servisi ve birimler henüz paket değil (6A); ISO derlenmedi.

## Sonraki adaylar

- 6A: `asena-packages` paketi (servis, polkit, D-Bus, birimler), Asena simge temaları ve adw-gtk3.
- 6B: Asena deposu ve `appstream-generator` ile Asena kataloğu.
- Background portalında penceresiz çalışan uygulama listesi ve kullanıcıya soran arayüz.
- GlobalShortcuts için Wayfire tarafında protokol desteği.
- APT işleri için iptal: Flatpak işleri iptal edilebilir, paket servisinin iptal yöntemi yok.
- Flatpak güncellemeleri için daemon bildirimi (şu an yalnız APT bildirilir).
- Gerçek polkit ajanıyla parola ve `auth_admin_keep` denemesi (normal giriş gerektirir).
