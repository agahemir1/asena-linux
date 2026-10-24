# Asena — Codex talimatları

## Proje
Asena, Debian 13 tabanlı bir Linux dağıtımı ve onun Wayland masaüstü ortamıdır. **Wayfire** compositor'ünün üzerinde, **Rust + GTK4/libadwaita** ile yazılmış kendi shell'imizi çalıştırır. Düzen: alt panel + başlat menüsü.

Ayrıntılar (bir göreve başlamadan önce ilgili olanları oku):
- `docs/VISION.md` — ilkeler, tasarım dili, performans bütçeleri
- `docs/ARCHITECTURE.md` — teknoloji yığını, süreçler, IPC, protokoller, yapılandırma
- `docs/COMPONENTS.md` — her bileşenin MVP ve sonraki kapsamı
- `docs/ROADMAP.md` — fazlar ve durum
- `docs/PLANNING.md` — uygulama öncesi kurgu, bağımlılıklar, açık kararlar ve kabul kapıları
- `docs/DESIGN.md` — onaylı Odak yönü; simgeler, cam malzeme, responsive ve görsel kabul
- `docs/ICONOGRAPHY.md` — gerçek hacimli uygulama simgelerinin üretimi ve kullanıcı inceleme kapısı
- `docs/design/` — görsel kimlik, tasarım token'ları, bileşenler, yerleşimler ve etkileşimler
- `docs/architecture/` — uygulama ekosistem mimarisi ve Asena Application Framework
- `docs/apps/` — 16 uygulama kataloğu, geliştirme rehberi, yol haritası ve test standartları
- `prompts/` — faz görevleri

## Mevcut çalışma aşaması
- 23 Ekim 2026 uygulama/sıfırlama devamı: 19 uygulama ve 20 Ayarlar paneli kaynakta; VTE GTK4, GStreamer ve poppler-utils eklendi, Open-Meteo seçildi. K30 servisi `asena-recovery`, veri işçisi `data/recovery/recovery.py`, sözleşme `docs/architecture/RECOVERY.md`. Gerçek ISO/Calamares/Secure Boot kurtarma kabulü açık; sonuçlar `docs/reports/apps-recovery.md` içindedir.
- 21 Ekim 2026 denetimi: uygulamalar açılmıyordu ve 15'i maketti; çatı düzeltildi, 11 uygulama gerçek veriye bağlandı, Uçbirim/Kamera/Belgeler/Hava Durumu/Yazılım gizlendi (bağımlılık kararı bekliyor). Shell yüzeyleri Odak'a göre yeniden kuruldu; çizgi simgeler `scripts/icons/symbolic.py`, hacimli inceleme simgeleri `scripts/icons/apps.py` ile üretilir ve ilk dört simge kullanıcı onayı bekler. Ayrıntı: docs/reports/audit-2026-10-21.md.
- 22 Ekim 2026 dağıtım hazırlığı: depo git'te (`main`). `iso/` live-build iskeleti ve `scripts/build-iso.sh` hazır, ISO derlenmedi; işletim sistemi yapısı kararları önce PLANNING'e K kaydı olarak yazılır, sonra `data/`, `packaging/` veya `iso/` altına dosya olarak girer. VM'de elle yapılan değişiklik ISO'ya geçmez. Ayrıntı: iso/README.md.
- 22 Ekim 2026 vurgu ve klavye: `asena-daemon` Flatpak uygulamaları için Settings portal arka ucu sunar (`portal.rs`, `color-scheme` ve `accent-color`). Vurgu paleti ve simge teması adı `asena_core::theme` içindedir; klasörler ile Dosyalar/Ayarlar simgeleri `Asena-<vurgu>` temalarından gelir, `scripts/icons/apps.py` üretir (klasörler kullanıcı tarafından onaylandı). Kullanıcı isteğiyle 16 Asena uygulamasının hepsine hacimli simge çizildi; masaüstü `Icon=` ve pencere simgesi uygulama kimliğidir, yedek `org.asenalinux.App` yalnız üçüncü taraf içindir; uygulama simgelerinin görsel onayı bekliyor. Harf kısayolları ilk klavye düzeninde o harfi yazan tuşa çevrilir (K18). wlroots 0.18 yukarı akış yaması kullanıcı onayıyla `packaging/wlroots/` altında (K17).
- 22 Ekim 2026 Faz 5D: Ayarlar › Ekranlar düzeni değiştirir; test/uygulama/15 sn onay/geri almanın tek sahibi `asena-daemon`'un `displays` modülüdür (sözleşme ARCHITECTURE'da). GTK 4.18 ölçek 1'e dönen ekranda bayat geometri verir; shell geometriyi `outputs.rs` ile compositor'dan okur, `gdk::Monitor::geometry()` doğrudan kullanılmaz. wlroots 0.18 modsuz (headless) çıkışlarda `wlr-randr --off/--on` ile düşer (yamalı paket `0.18.2-3+asena1`); düzenekler daemon yolunu kullanır. Gerçek monitör kabulü açık. Ayrıntı: docs/reports/phase-05.md.
- 21 Ekim 2026 compositor geçişi: kullanıcı tek compositor olarak **Wayfire 0.9.0**'ı onayladı; labwc kaldırıldı. Yapılandırma `data/wayfire/wayfire.ini`, ayarları `asena-daemon`'un `wayfire` modülü yazar. Dual Kawase blur panel, menü, hızlı ayarlar ve kenar panelinde çalışır; Görünüm'deki Saydamlık efektleri kapalıysa veya yüksek kontrastta shell opaktır ve blur yüklenmez. Flatpak uygulamaları çalışmalıdır; Wayfire'da `wp-security-context-v1` olmadığı bilinen bir yalıtım farkıdır. VM'de smoke, kilit, greeter ve Flathub uygulaması doğrulandı; gerçek GPU ve normal giriş açık. Wayfire kısayolları fiziksel tuş koduna bakar, `wtype` onları tetikleyemez. Önceki satırlardaki "varsayılan labwc korunur" ve "compositor değişikliği onaylanmadı" ifadeleri bu kararla geçersizdir. Ayrıntı: docs/reports/wayfire-migration.md.
- 21 Ekim 2026 K13 deneyi: Wayfire 0.9.0 `blur` eklentisi Dual Kawase (`method = kawase`) ile Asena panel, menü, hızlı ayarlar ve kenar panelinin arkasındaki pencereleri VM'de (llvmpipe) bulanıklaştırdı; shell kodu değişmedi. Bu sonuç Wayfire geçiş kararının dayanağıdır. Ayrıntı: docs/research/blur-options.md.
- 19 Ekim 2026 ek kullanıcı yanıtı: Faz 4 için yalnız gtk4-layer-shell C kitaplığının 1.3.0 backport'u onaylandı; önce VM'de doğrulanacak. GTK 4.18, libadwaita 1.7 ve varsayılan labwc korunur. K03 ayrıntısı docs/research/session-lock-options.md içindedir.
- 19 Ekim 2026 kullanıcı mesajı diğer fazların da sırayla uygulanmasına onaydır. Her faz için tekrar genel uygulama onayı istenmez; kabul koşulları ve güvenlik/API karar kapıları korunur. Compositor değişikliği ayrıca onaylanmadı. Kullanıcı 19 Ekim’de yalnız VM içinde yalıtılmış Wayfire uyumluluk/blur deneyini seçti; varsayılan oturum labwc olarak kalır.
- 18 Ekim 2026 kullanıcı mesajı Faz 0 uygulamasına açık onaydır. Bu faz için yeniden plan/onay bekleme ve T0 bitmeden altyapıya başlamama koşulları kaldırıldı. Kaynak uygulaması başladı; doğrulama durumu `docs/reports/phase-00.md` içindedir. Diğer fazlar için kapsam onayı veya compositor değişikliği çıkarılmaz.
- Faz 0–7 planları yazıldı. Faz 0–3 kaynakları ve yerel kontroller mevcut; Faz 3 son kapısı 34 Rust + 8 Python testiyle geçti. Donanım ve görsel kabul açık. Onaylı C 1.3.0 ayrı prefix backport ve 53 upstream test/panel regresyonu VM’de geçti; Faz 4A kilit/PAM uygulaması ve güvenlik deneyi sürüyor. Gerçek durum docs/reports/phase-01.md ile phase-04.md arasındaki raporlardadır.
- Belge/prompt hazırlığı fazı uygulanmış yapmaz. Kaynak kod, bağımlılık kurulumu ve dağıtım üretimine plan düzenlemesi sırasında geçilmez.
- Ürün kararları ile teknik doğrulamalar `docs/PLANNING.md` içinde izlenir; varsayım ve öneriler doğrulanmış sonuç olarak yazılmaz.
- Tasarım hazırlığı `prompts/T0-tasarim.md` ile yürütülür. Kullanıcı Odak yönünü, derinlikli uygulama simgelerini, zarif çizgi simgelerini ve gerçek blur hedefini seçti. Blur yolu K13 kararıyla Wayfire'dır; üretim opaklık ve blur değerleri gerçek GPU ve görsel kabulle kesinleşir.
- Menü tipi ve panelin alt/üst/sol/sağ konumları Ayarlar Merkezi'nden değiştirilebilir olacak. Odak varsayılandır; sabit düzen sınırlaması değildir. Önceki kutu/çizgi simge önizlemeleri nihai kalite onayı sayılmaz; hacimli uygulama simgeleri kullanıcının görsel onayını bekler (klasörler 22 Ekim'de onaylandı).
- Kullanıcı arayüzünde dolgu açıklama, tanıtım cümlesi, tekrarlanan başlık/alt başlık, em dash, dekoratif renkli rozet veya gerekçesiz üçlü kart şablonu istemiyor. DESIGN'in arayüz metni ve düzen kuralları bütün ekran ve taslakların kabul koşuludur. Gerekli hata/sonuç açıklamaları ve gerçek ikincil veriler korunur.

## Geliştirme ortamı
- Derleme ve test **Linux'ta** yapılır: tercihen Debian 13 sanal makine, hızlı denemeler için WSL2 + WSLg. Windows'ta derleme yapılmaz.
- Komutlar (depo kökünde):
  - `scripts/setup-dev.sh` — bağımlılıkları ve onaylı yerel C backport'unu kur (bir kez)
  - `source scripts/native-env.sh` — doğrudan Cargo komutlarından önce
  - `cargo build --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo fmt --all`
  - `cargo test --workspace`
  - `scripts/dev-session.sh` — mevcut masaüstünde iç içe Wayfire + Asena oturumu
- GPU yoksa: `WLR_RENDERER_ALLOW_SOFTWARE=1 scripts/dev-session.sh` (Wayfire yalnız OpenGL ES ile çizer; pixman yolu yoktur)
- GSettings şemaları geliştirmede `target/schemas` altına derlenir (`GSETTINGS_SCHEMA_DIR`).

## Değişmez kurallar
1. **API sınırı:** GTK 4.18, libadwaita 1.7, gtk4-layer-shell Debian 13 tabanı 1.0.4; Faz 4 için kullanıcı onaylı C 1.3.0 backport istisnası vardır (önce VM doğrulaması). Crate feature'ları en fazla `v4_18` / `v1_7`. Daha yeni bir API gerekirse dur ve sor.
2. **MSRV:** Debian 13'ün rustc sürümü (`Cargo.toml`'daki `rust-version`).
3. **Ana thread bloklanmaz.** Senkron IO, `block_on` veya `std::thread::sleep` ana thread'de yasak. D-Bus için zbus async + `glib::spawn_future_local`, bloklayan iş için `gio::spawn_blocking`.
4. **Panik yok:** `unwrap()`/`expect()` test dışında yasak (workspace lint'i). Kütüphanelerde `thiserror`, binary'lerde `anyhow`. Servis veya donanım yoksa öğe gizlenir, süreç çökmez.
5. **Çeviri:** Kullanıcıya görünen her metin `gettext("...")` içindedir. Kaynak dil İngilizce, `po/tr.po` aynı committe güncellenir.
6. **Renk:** CSS'te sabit renk yazılmaz; libadwaita adlandırılmış renkleri (`@accent_bg_color` …) ve `docs/VISION.md`'deki token'lar kullanılır.
7. **Compositor bağımsızlığı:** Wayfire'a özgü kod yalnızca `data/wayfire/`, `asena-session` ve `asena-daemon`'un `wayfire` modülünde bulunur. Shell yalnızca standart Wayland protokolleriyle konuşur.
8. **Türkçe arama ve eşleştirme** her zaman `asena_core::text::fold()` ile yapılır; `to_lowercase()` doğrudan kullanılmaz (`İ` → `i̇` hatası). Kullanıcıya gösterilen alfabetik listelerde Türkçe yerel ayarına duyarlı karşılaştırma kullanılır; `fold()` Türk alfabesinin sırasını korumaz.
9. **Ayrıcalıklı işlemler** polkit korumalı D-Bus servisleriyle yapılır (localed, timedated, hostnamed, AccountsService …). Shell betiğini `pkexec` ile çalıştırmak yasak.
10. **Yeni bağımlılık:** eklemeden önce bakımı süren bir crate olduğunu ve lisansının GPL-3.0 uyumlu olduğunu kontrol et; nedenini commit mesajına yaz.
11. **Adlandırma:** D-Bus ve uygulama kimlikleri `org.asenalinux.*`, GSettings şemaları `org.asenalinux.<alan>`.
12. **Uygulama Ekosistemi Standardı:** Gelecekte geliştirilen bütün Asena uygulamaları öncelikle `docs/design/` ve `docs/architecture/` belgelerini okumalı, ortak bileşenleri (`asena-app`, `asena-widgets`, `asena-core`) yeniden kullanmalı, aynı görsel/işlevsel standartları izlemeli ve yeni bileşen veya mimari değişikliklerde dokümantasyonu (`docs/apps/`, `docs/design/`, `docs/architecture/`) güncellemelidir. Sahte veri ve çalışmayan mockup bırakılamaz. Bu kural kalıcıdır ve sonraki oturumlarda da geçerlidir.

## Kod stili
- rustfmt varsayılanları; clippy temiz.
- GObject alt sınıfları standart `mod imp` deseniyle yazılır; dosya başına bir widget.
- Karmaşık ekranlar Blueprint (`.blp`) + `CompositeTemplate`, küçük shell widget'ları Rust kodunda.
- Servis kodu (`asena-services`) GTK widget'ı import etmez; yalnızca GObject property ve signal sunar.
- Kod, kod yorumları ve commit mesajları İngilizce. Belgeler ve prompt'lar Türkçe.
- Commit: Conventional Commits (`feat(shell): …`, `fix(lock): …`), küçük ve tek konulu.

## "Bitti" tanımı
Bir görev şunların hepsi sağlanınca biter:
- [ ] `cargo build`, `clippy -D warnings`, `fmt --check`, `test` temiz
- [ ] Mantık içeren kodun birim testi var (arama sıralaması, ayrıştırıcılar, durum makineleri …)
- [ ] `scripts/dev-session.sh` içinde elle denendi; ne denendiği rapora yazıldı
- [ ] Yeni metinler çevrilebilir ve `tr.po` güncel
- [ ] Mimari veya kapsam değiştiyse ilgili `docs/` dosyası güncellendi
- [ ] Faz bittiyse `docs/ROADMAP.md`'de durum ve faz notu güncellendi

## Çalışma biçimi
- Her faz `prompts/NN-*.md` dosyasıyla başlar. Önce planı sun, onay al, sonra uygula.
- Kapsamı kendiliğinden genişletme. Fark ettiğin ek işleri raporun sonunda "Sonraki adaylar" başlığı altında listele.
- Belgelerle çelişen bir karar gerekiyorsa uygulamadan önce sor.
- Doğrulanamayan şeyleri (gerçek donanım, çoklu monitör vb.) raporda açıkça belirt.
