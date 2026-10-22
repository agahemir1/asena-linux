# Geliştirme

Faz 0 Linux kalite kontrolleri ve iç içe panel denemesi geçti. Fiziksel donanım ve normal giriş kabulü açık.
Durum kaynağı [Faz 0 raporu](reports/phase-00.md).

## Debian 13 kurulumu

VM önerilir. WSL2/WSLg hızlı iç içe deneme içindir; gerçek giriş, kilit ve donanım
kabulünün yerine geçmez. Windows üzerinde Cargo derlemesi yapılmaz.

19 Ekim 2026'da yerel QEMU 11.1.2 / Debian 13 geliştirme VM'si hazırlandı.
QEMU, resmî indirme sayfasının yönlendirdiği MSYS2 paketlerinden, paket SHA-256
kontrolleriyle `target/vm/msys` altına açıldı. Debian genericcloud amd64 tabanı
Debian'ın SHA-512 kaydıyla doğrulandı; değişiklikler ayrı 32 GB QCOW2 overlay'e
yazılır. Windows'a sistem çapında QEMU kurulumu veya özellik değişikliği yapılmadı.

Bu çalışma alanında hazırlanan VM'yi yeniden açmak için Windows PowerShell'de:

```powershell
./scripts/start-dev-vm.ps1
ssh -i target/vm/dev-key -p 22310 -o UserKnownHostsFile=target/vm/known_hosts asena@127.0.0.1
```

Linux kaynakları `/home/asena/asena` altındadır. SSH yalnız `127.0.0.1:22310`
üzerinden erişilir, anahtar doğrulaması kullanır; parola girişi kapalıdır.
`target/vm` içindeki özel anahtarı veya VM dosyalarını depoya eklemeyin/paylaşmayın.
Konukta `sudo poweroff` güvenli kapatmadır. Başlatıcı varsayılan olarak 6 GB RAM,
4 vCPU, WHPX ve gerekirse TCG geri dönüşü kullanır; sanal donanım gerçek GPU
ve fiziksel monitör kabulü sayılmaz. Bu bölüm mevcut yerel kurulumu açıklar;
`start-dev-vm.ps1` eksik imajları indiren genel bir kurucu değildir.

```sh
bash scripts/setup-dev.sh
export PATH=/usr/bin:/bin:$PATH
source scripts/native-env.sh
bash scripts/check-quality.sh
```

Kurulum betiği yalnız Debian 13'te APT çalıştırır, tekrar çalıştırılabilir ve rustup
ayarlarını değiştirmez. Derleyici, Cargo, rustfmt ve Clippy Debian paketinden gelir.
`rustc --version` ve `/usr/bin/rustc --version` farklıysa PATH'i kontrol edin.
İlk kurulum betiğini `bash` ile çağırın; Windows'tan taşınan dosyalara Linux'ta
çalıştırma izni verir ve şema/çeviri çıktılarını hazırlar. Git'e eklerken betiklerin
executable bit'lerini koruyun. Bu çalışma dizini henüz bir Git deposu değildir.

## Günlük akış

```sh
bash scripts/compile-data.sh
export GSETTINGS_SCHEMA_DIR="$PWD/target/schemas"
export ASENA_LOCALE_DIR="$PWD/target/locale"
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash scripts/dev-session.sh
```

`check-quality.sh` ayrıca TOML/XML/kaynak listesi, ShellCheck, gettext ve desktop
kaydı kontrollerini, ardından Linux oturum sözleşmesi testlerini çalıştırır.
Şema varsayılanı testi `target/schemas` içindeki gerçek derlenmiş şemayı bellek
backend'iyle açar; kullanıcının ayarlarına yazmaz. Shell normal çalışmada gerçek
GSettings backend'ini kullanır. `fold` arama içindir, Türkçe liste sıralaması için
yerel ayarlı karşılaştırma gerekir.

Çeviriler için `bash scripts/update-po.sh`, doğrulama için
`bash scripts/update-po.sh --check` kullanın. Yeni kaynak dosyalarını `po/POTFILES`
listesine ekleyin. Kaynak dil İngilizce, Türkçe katalog aynı değişiklikte güncellenir.
Debian gettext 0.23, `--language=Rust` seçeneğini tanımıyor. Betik Rust kaynaklarını
`--language=C` tarayıcısıyla işler. `prepare-gettext.py` lifetime belirteçlerindeki apostrofları, dizgi/karakter ve yorumları koruyarak ön işler. Satır numaraları korunur; bu geçişin regresyon testleri vardır. Çevrilebilir metinlerde normal `gettext("...")`
dizgileri kullanın; Rust raw-string biçimi bu sözleşmede desteklenmez.
`#[...]` satırlarına gettext çağrısı koymayın; C tarayıcısı bunları önişlemci
satırı sayar. Hata metinleri normal Rust metotlarında çevrilir. Şema metinleri
ayrı `--language=GSettings` tarayıcısıyla alınır.
`LOCPATH="$PWD/target/locale-data" LANGUAGE=tr LC_ALL=tr_TR.UTF-8 TEXTDOMAINDIR="$PWD/target/locale" gettext -d asena "Pinned applications"`
çeviri yükleme denemesidir; uygulamadaki Türkçe hata akışı ayrıca denenmelidir.
`compile-data.sh`, sistem yerel ayarını değiştirmeden `target/locale-data` altında
Türkçe locale verisi üretir. C/C.UTF-8 locale'leri GNU gettext çevirisini açmadığı
için yalnız `LANGUAGE=tr` atamak yeterli değildir.

## İç içe oturum

Mevcut bir Wayland oturumu ve `XDG_RUNTIME_DIR` gerekir:

```sh
WLR_WL_OUTPUTS=2 ASENA_CAPTURE_PROTOCOLS=1 bash scripts/dev-session.sh
```

Betik workspace'i derler, özel D-Bus oturumu ve süreç grubu altında Wayfire başlatır.
Her sanal çıkışta 48 mantıksal piksel panel, uygulama görev çubuğu ve saat görünür. Super+Enter `foot` açar;
iç içe pencereyi kapatmak oturumu bitirir. Wayfire yalnız OpenGL ES ile çizer; GPU yoksa başına
`WLR_RENDERER_ALLOW_SOFTWARE=1` ekleyin (llvmpipe). `WLR_BACKENDS=headless` ile üst masaüstü gerekmez;
betik bu durumda ilk PCI görüntü aygıtını seçer. Gerçek donanım ve normal giriş ayrıca doğrulanır.

`asena-session` Cargo binary'si gömülü POSIX başlangıç betiğini çalıştırır; betik
ortamı ayarlayıp `wayfire -c ...` çalıştırır. `--dev` depodaki verileri ve
`target/debug` binary'lerini kullanır. Daemon yapılandırmayı yeniden yazdığı için
geliştirme başlatıcısı `data/wayfire/wayfire.ini` dosyasını `/tmp/asena-dev.XXXXXX`
içine kopyalar ve `ASENA_DEV_CONFIG` ile verir. Kaynak/binary yolları boşluk içerebilir.
Çıkışta bu geçici dosyalar ve süreç grubu temizlenir. Özel `CARGO_TARGET_DIR`
bu başlatıcı için kullanılmaz; binary yolları açıkça `target/debug` altındadır.

İç içe oturum gerçek systemd user yöneticisine ortam aktarmaz, gerçek Asena
target'ını başlatmaz/durdurmaz. `~/.config/asena/wayfire.ini` okunmaz veya değiştirilmez.
Normal oturumda Wayfire çıkınca `asena-session` target'ı durdurur; bu yol henüz
gerçek girişte denenmemiştir. `/usr/share` ve `/usr/libexec` kurulumları Faz 6 kapsamındadır.
Bu faz host'un display manager, PAM veya user servis kurulumunu değiştirmez.

Wayfire başlattığı programların çıkışını `/dev/null`'a gönderir. `asena-session --dev` kendi
stderr'ini `ASENA_DEV_LOG` ile autostart betiğine verir; `ASENA_LOG_STDERR=1` ile shell, daemon
ve kilit günlükleri başlatıcının çıktısında görünür (aksi halde journald'ye gider). Özel D-Bus
oturumu servis dosyalarını yalnız başlarken okur; bu yüzden başlatıcı Flatpak dışa aktarım
dizinlerini veri yolundan önce `XDG_DATA_DIRS`'e ekler ve D-Bus ile etkinleştirilen Flatpak
uygulamaları menüden açılır. Wayfire kısayolları fiziksel tuş koduna baktığı için `wtype` onları
tetikleyemez; kısayolları önizlemede noVNC ile veya gerçek klavyeyle deneyin.

## Kabul için kaydedilecek kanıt

1. `bash scripts/collect-environment.sh`: `target/evidence/environment.txt`.
2. `bash scripts/check-quality.sh`: komutların tam çıkışları ve çıkış kodları.
3. İç içe oturumun `target/wayland-info.txt` çıktısı; dış oturumun protokollerini
   bunun yerine kullanmayın.
4. Panel/terminal görüntüsü: 48 px yüzey, ayrılmış çalışma alanı, iki çıkış,
   çıkış çıkarma/ekleme, %100/%200 ölçek ve geçici sıfır çıkış durumu.
5. Normal kapanış ve Ctrl+C sonrası süreç listesi; gerçek oturumun D-Bus/systemd
   ortamının değişmediğinin karşılaştırması. Ani compositor ölümü ayrıca denenir.
6. `ldd target/debug/asena-shell`: layer-shell, libwayland-client'tan önce
   yüklenmeli. [Upstream bağlama notu](https://github.com/wmww/gtk4-layer-shell/blob/v1.0.4/linking.md).

CI Debian `trixie` konteynerinde dağıtım derleyicisini kullanır. İş sonunda
`Cargo.lock` ve ortam envanterini artifact olarak saklar. Lock dosyası Debian Cargo ile üretildi ve Rust 1.85.1 üzerinde derlendi. Kalite betiği `--locked` kullanır. Kullanıcının seçimiyle dış GitHub Actions koşusu çalıştırılmadı.

## Sonraki fazların sözleşmeleri

- `asena-core::layout`: anlamsal panel kenarı ve menü türü. Şimdilik yalnız
  varsayılan alt kenar uygulanır. Canlı ayarlar, boyut sınırları ve popup geometrisi
  Faz 1/2/5'te aynı modele eklenecek; widget'lara bağımsız koordinat konmaz.
- `asena-services` GTK içermez. Durum modelleri Faz 3'te GObject olacak.
- `asena-wayland` GTK içermez. Faz 1'de ayrı bağlantı/thread ve async kanal kullanır.
- `asena-widgets` ortak stil ve ileride uyarlanabilir uygulama bileşenlerini barındırır.
  Settings için dar/geniş gezinme, kısa yükseklik ve büyük yazı kabulü Faz 5'ten
  itibaren zorunludur; boş iskeletler responsive uygulama kabulü değildir.
- Uygulama simgeleri ile sembolik kontroller ayrı ailelerdir. Faz 0 nihai varlık
  üretmez. `data/icons` veya `wallpapers` için dolgu dosya eklenmez.
- Gerçek blur'u Wayfire `blur` eklentisi çizer; shell yalnız yarı saydam malzeme verir.
  [K13 deneyi](research/blur-options.md), [geçiş](reports/wayfire-migration.md).


## Sürüm denetimi

Depo 22 Ekim 2026'dan beri git'tedir (`main`). `target/` (derleme çıktıları ve VM diski) izlenmez;
`.gitattributes` bütün metin dosyalarını LF tutar. VM'deki `~/asena` kopyası `.git` taşımaz; ISO
derlerken kaynak commit'i `ASENA_SOURCE_COMMIT` ile verilir (`iso/README.md`).

## Yerel masaüstü önizlemesi

Hazırlanan VM için Windows PowerShell'de `scripts/open-dev-desktop.ps1` kaynakları
VM'ye kopyalar, Linux'ta derler ve headless Wayfire oturumunu hazırlar. Tarayıcı adresi
`http://127.0.0.1:22380/vnc.html?autoconnect=true&resize=scale` olur.
Linux tarafında `wayvnc`, `novnc`, `websockify` paketleri gerekir. Bu paketler
normal Asena oturumunun bağımlılığı değildir. HTTP yalnız QEMU'nun host loopback
portundan açılır; VNC konuğun loopback adresinde kalır. İnternete açmayın.
Önizlemede Asena oturumunun kendi Wayfire'ı headless çalışır ve wayvnc onun çıkışını
(`HEADLESS-1`) gösterir; iç içe compositor yoktur. Çizim llvmpipe yazılım GL ile yapılır;
blur görünür ama bu GPU veya performans kabulü değildir. VM'de `vgem` yüklüyse `card0`
vgem olur; betik bochs gibi gerçek KMS aygıtını `/dev/dri/by-path/pci-*-card` ile seçer.
Durdurma: SSH içinde `cd ~/asena && bash scripts/preview-desktop.sh stop`.
`ASENA_PREVIEW_OUTPUTS=2` ikinci bir sanal çıkış ekler (wayvnc yalnız ilkini gösterir); ekran düzeni
denemeleri için kullanın. `python3 -I scripts/probe-displays.py HEADLESS-1` oturum içinde daemon'un
onay, geri alma ve red davranışını uçtan uca dener. Çıkışı kapatıp açmak için `wlr-randr --off/--on`
kullanmayın: wlroots 0.18 modsuz headless çıkışlarda, daemon çıkışları izlerken compositor'ı düşürür;
`probe-displays.py HEADLESS-1 --disable-and-revert` aynı işi daemon üzerinden yapar. Yamalı wlroots için:
`bash scripts/build-wlroots.sh` ve ardından
`sudo apt-get install ./target/packages/wlroots/libwlroots-0.18_*_amd64.deb` (derleme araçları betiğin
başında yazılıdır). Önizleme yalıtılmış `XDG_CONFIG_HOME` kullandığından özel klasör simgeleri için
`target/preview/config/user-dirs.dirs` gerekir.

Tekrarlanabilir otomatik iç içe deneme: `bash scripts/smoke-session.sh`.
`GSK_RENDERER=cairo ASENA_SMOKE_APPS=1 bash scripts/smoke-session.sh` üç varsayılan
uygulamayı da dener; `nautilus`, `ptyxis`, `firefox-esr`, `wlrctl` paketleri gerekir.
Sonuçlar `target/evidence/session-smoke-<renderer>` altında tutulur. Süreç temizliği
ve user-manager ortam karşılaştırması aynı betikte yapılır. Faz 1 raporu başarı
ile sınırlamaları ayrı kaydeder.

## Faz 2–4 geliştirme kontrolleri

Native kilit backport'u `setup-dev.sh` tarafından yalnız `target/native` altına
kurulur. Ayrı tekrar üretim: `bash scripts/backport-layer-shell.sh`. Doğrudan
Cargo komutlarından önce `source scripts/native-env.sh` gerekir. Başlatıcılar ve
kalite betiği bu dosyayı yükler. Debian'ın sistem kitaplığı değiştirilmez;
ürün .deb backport paketi ayrıca üretilecek. Build, native >=1.3.0 yoksa açık
hata verir. Dış GitHub Actions koşusu henüz doğrulanmadı.

Menü/bildirim/OSD deneyi:

```sh
ASENA_SMOKE_PHASE2=1 GSK_RENDERER=cairo bash scripts/smoke-session.sh
```

Yalnız geliştirme VM'sinde servisler ve geçici null ses aygıtıyla GUI deneyi:

```sh
ASENA_SERVICE_TEST_VM=1 bash scripts/setup-vm-services.sh
systemctl --user start pipewire.socket pipewire-pulse.socket wireplumber.service
ASENA_AUDIO_TEST_VM=1 ASENA_SMOKE_PHASE3=1 GSK_RENDERER=cairo \
  bash scripts/probe-audio.sh bash scripts/smoke-session.sh
ASENA_AUDIO_TEST_VM=1 bash scripts/probe-audio-reconnect.sh
```

`target/debug/examples/{service_probe,objects_probe,mpris_probe,tray_probe}`
özel D-Bus ile bağımsız çalıştırılır. `tray_probe --collision` ve
`notifications_probe --collision` mevcut adın devralınmadığını sınar.
Bu deneyler gerçek Wi-Fi, Bluetooth, pil veya iki gerçek tepsi uygulaması kabulü
değildir. İç içe oturum daemon'ı gerçek masaüstüne görünüm/gamma yazmaz.

Kilit deneyi yalnız kurtarma SSH'ı ve disk görüntüsü bulunan, başlangıçta parolası
kilitli **asena** hesabına sahip geliştirme VM'si içindir:

```sh
ASENA_LOCK_TEST_VM=1 bash scripts/test-lock-vm.sh
```

Betik geçici PAM dosyası ve rastgele parola kurar; deney çıkışında hesabı yeniden
kilitler, geçici parolayı ve kendi PAM dosyasını kaldırır. Mevcut bir PAM dosyasını
veya kullanılmakta olan bir hesap parolasını değiştirmeyi reddeder. Sıfırdan
VM/TTY oturum kabulü ayrı yapılır. Kilitleyici zorla sonlandırılırsa kurtarma
yolu özel iç içe compositor süreç grubunu kapatmaktır; shell'i yeniden başlatmak
kilit açma yolu değildir. [Faz 4 raporu](reports/phase-04.md).

Bu VM betiği ayrıca tek swayidle yardımcı sürecini, ayar değişiminde temizliği,
geçersiz süre sırasını ve otomatik kilidi dener. Uyku bekletmesinde doğru parola
kabul edilmez; bekletme sahibi kesilince giriş geri gelir. Teste özel
`ASENA_POWER_TEST_VM=1` yalnız bu özel oturumda verilir. Normal önizleme kilit ve
otomatik güç eylemlerini etkinleştirmez. Betik gerçek askıya alma yapmaz.

`target/debug/examples/power_probe`, yalnız ayrı ve kurtarılabilir bir VM/DRM
deneyinde `ASENA_LOCK_TEST_VM=1` ile çalıştırılmalıdır. Ekranları kapatıp geri
açmayı dener. İç içe Wayland backend'inde başarısız oldu; PAM testi içine
konulmaz. [Sınırlar](research/idle-sleep.md).

Greeter protokol/arayıüz deneyi, yalnız özel geliştirme VM'sinde:

```sh
ASENA_GREETER_TEST_VM=1 bash scripts/probe-greeter.sh
LOCPATH=$PWD/target/locale-data LC_ALL=tr_TR.UTF-8 LANGUAGE=tr \
  ASENA_GREETER_TEST_VM=1 ASENA_GREETER_TEST_SMALL=1 bash scripts/probe-greeter.sh
```

İkinci komut 640×480 ve özel GTK kullanıcı CSS'inde 22 px yazı kullanır.
GDK_DPI_SCALE büyük yazı kanıtı sayılmaz. Betik yalnız eksikse geçici root sahipli
Asena session kaydı kurar ve çıkışta kaldırır. Mevcut `asena.desktop` kaydını
değiştirmez. Root yetkisi test kaydı içindir; test sunucusu ayrıcalıksızdır,
kimlik doğrulamaz veya oturum başlatmaz. Sonuçlar `target/evidence/greeter-0`
ve `greeter-1` altındadır. Gerçek greetd/TTY kabulü ayrı yapılır.

`data/greetd/config.toml.example`, **etkin olmayan** VT7 örneğidir. Gerçek
kurulum ayrı greeter hesabı, onun state dizini ve doğrulanmış greetd/cage/PAM
ortamı ister. Mevcut display manager değiştirilmez. VM'de TTY/SSH kurtarma erişimi
ve anlık görüntüyle doğrulama bitmeden normal açılışa bağlanmaz.

## Dağıtım denemeleri

Önyükleme zinciri, sistem kimliği ve takas (K07, K20, K23, K24), yalnız geliştirme VM'sinde:

```sh
sudo apt-get install debootstrap qemu-system-x86 qemu-utils ovmf gdisk dosfstools librsvg2-bin
ASENA_BOOT_TEST_VM=1 bash scripts/test-boot-vm.sh
```

Betik `target/boot-test` altında 6 GB seyrek bir disk dosyasına debootstrap ile Debian kurar,
imajın sistem dosyalarını `scripts/stage-system-files.sh` ile yerleştirir, `iso/config/hooks`
altındaki chroot kancalarını çalıştırır ve diski OVMF ile Secure Boot kapalı ve açık iki kez açar.
Açılış sırasında 10 saniyede bir ekran görüntüsü alır. Disk dosyası ve loop aygıtı dışında
sisteme dokunmaz. VM'de KVM yoksa TCG kullanılır ve deneme 30 dakika kadar sürer. Sonuçlar
`target/evidence/boot-test`; ayrıntı [iso/README.md](../iso/README.md).

Paket servisi (K21, Faz 6F), yalnız geliştirme VM'sinde:

```sh
cargo build -p asena-packages
ASENA_PACKAGES_TEST_VM=1 bash scripts/test-packages-vm.sh
```

Betik `asena-packages`'ı deneme süresince D-Bus ile etkinleşen sistem servisi olarak kurar
(`/usr/local`, `/etc/dbus-1/system.d`, polkit eylemleri, `/run/systemd/system`) ve çıkışta
kaldırır. Önce yetkisiz istekleri, sonra çağıran kullanıcıya geçici bir polkit kuralıyla `hello`
paketinin kurulup kaldırılmasını, güncelleme listesini ve yeniden başlatma zamanlamasını dener.
Makineyi yeniden başlatmaz; zamanlanan güncellemeyi iptal eder. Sonuçlar
`target/evidence/packages-test`.
