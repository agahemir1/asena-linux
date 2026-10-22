# labwc'den Wayfire'a geçiş

21 Ekim 2026. Kullanıcı tek compositor kalmasını, bunun Wayfire olmasını ve geçişin bu fazda
tamamlanmasını onayladı. Koşulu: Flatpak uygulamaları çalışmaya devam etmeli. Gerekçe K13
deneyidir ([blur-options.md](../research/blur-options.md)): Debian 13'ün Wayfire 0.9.0 paketi
Dual Kawase blur'u Asena yüzeylerinde çalıştırdı; labwc'de aynı sonuç için labwc, wlroots ve
SceneFX'in yeni sürümlerini Asena'nın paketlemesi gerekirdi.

## Plan

| Adım | İçerik | Kabul |
|---|---|---|
| 1. Compositor yapılandırması | `data/wayfire/wayfire.ini`: eklentiler, kısayollar (labwc `rc.xml` eşdeğerleri), Dual Kawase blur kuralı, pencere başlığı, animasyon, yerleşim, Xwayland | Wayfire ayrıştırır; kısayollar ve blur VM'de çalışır |
| 2. Daemon modülü | `asena-daemon` içinde `labwc.rs` yerine `wayfire.rs`: `org.asenalinux.input` ayarlarını `[input]` bölümüne, saydamlık ve animasyon tercihini eklenti listesine yazar. Wayfire dosyayı kendisi yeniden okur | Birim testleri; dosya değişince canlı uygulanır |
| 3. Oturum | `asena-session` Wayfire'ı `-c` ile başlatır; ilk girişte kullanıcı dosyasını sistem varsayılanından oluşturur, böylece ayarlar ilk oturumda da canlı uygulanır. labwc `shutdown` betiğinin işini Wayfire çıkınca oturum betiği yapar | Python oturum testleri |
| 4. Geliştirme ve deney düzenekleri | `dev-session.sh`, önizleme, smoke, greeter ve kilit deneyleri Wayfire ile çalışır. Önizleme iç içe compositor yerine doğrudan headless Wayfire + wayvnc kullanır | Önizleme açılır; smoke ve kilit deneyi geçer |
| 5. Saydamlık tercihi | `org.asenalinux.desktop transparency` ve Görünüm sayfasında anahtar. Açıkken shell cam malzemesini kullanır ve blur çalışır; kapalıyken veya yüksek kontrastta opak malzeme kullanılır ve blur eklentisi yüklenmez | Ayarlar, shell ve daemon aynı anahtarı izler; çeviriler tamam |
| 6. Greeter | Oturum tercih sırası yalnız Asena; labwc geri dönüşü kalkar | Birim testi |
| 7. Flatpak | `flatpak` geliştirme kurulumuna eklenir; Flathub'dan bir uygulama Wayfire oturumunda açılır, dosya seçici portalı ve koyu stil denenir | Uygulama açılır, portal diyaloğu görünür |
| 8. labwc kaldırma | `data/labwc`, labwc paket bağımlılığı ve betik başvuruları kaldırılır | Depoda labwc'ye çalışan başvuru kalmaz (tarihî raporlar hariç) |
| 9. Belgeler | CLAUDE.md, AGENTS.md, README, ARCHITECTURE, COMPONENTS, DESIGN, VISION, PLANNING, ROADMAP, DEVELOPMENT, DEPENDENCIES, uygulama belgeleri ve faz prompt'ları | Belgeler Wayfire'ı anlatır; eski raporlar tarih olarak kalır |

Kalite kapısı: `scripts/check-quality.sh` (build, clippy, fmt, Rust ve Python testleri) ve
önizlemede elle deneme.

## Bilinen farklar

- Wayfire yazılım (pixman) renderer'ı sunmaz. GPU sorunu olan makinede wlroots llvmpipe'a
  düşer (`WLR_RENDERER_ALLOW_SOFTWARE=1`). Geliştirmedeki `WLR_RENDERER=pixman` yolu kalkar.
- Bu Wayfire yapılandırmasında `wp_security_context_v1` yok. Flatpak bu protokol yoksa
  uygulamaya olağan Wayland soketini verir; uygulamalar çalışır ama compositor sandbox içindeki
  istemciyi ayrıcalıklı protokollerden (ekran yakalama, sanal klavye vb.) ayıramaz. labwc 0.8.3
  bu ayrımı yapabiliyordu. Bu bir yalıtım gerilemesidir.
- ext-workspace, ext-foreign-toplevel-list, cursor-shape, tearing-control,
  input-method-v2/text-input-v3 yok. Asena bunları kullanmıyor; girdi yöntemi (Çince, Japonca
  vb.) gerekirse ayrıca ele alınır.
- Pencere başlığı yazısını Wayfire her zaman beyaz çizer; başlık rengi iki temada da koyudur.
  GTK uygulamaları kendi başlıklarını (CSD) çizmeye devam eder.
- labwc'deki `W-Escape` (compositor'den çık) kısayolunun Wayfire karşılığı yok; oturum güç
  menüsündeki Oturumu kapat ile logind üzerinden kapanır.
- Kısayollar fiziksel tuş koduyla (`KEY_A`) eşleşir; labwc tuş simgesiyle eşliyordu. TR-F
  düzeninde Super+A, QWERTY'de A'nın yerindeki "u" tuşu, Super+N ise "z" tuşudur. Enter, Space,
  Print ve medya tuşları düzenden bağımsızdır. Aynı nedenle `wtype` Wayfire kısayollarını
  tetikleyemez (kendi keymap'inde tuşlara rastgele kod verir); kısayollar wayvnc/noVNC veya gerçek
  klavyeyle denenir.
- Wayfire başlattığı programların stdout/stderr çıkışını `/dev/null`'a yönlendirir. Gerçek
  oturumda Asena günlükleri journald'ye gider; geliştirme oturumunda `asena-session --dev` kendi
  stderr'ini `ASENA_DEV_LOG` ile autostart betiğine verir.
- Settings portalı (xdg-desktop-portal-gtk 1.15.3) `color-scheme` değerini iletir, `accent-color`
  için `NotFound` döner. Flatpak uygulamaları koyu temayı izler, Asena vurgu rengini almaz.
- Portal dosya seçicisi GTK3 iletişim kutusudur ve açık temada açılır; GTK3 teması K14'te seçilir.

## Sonuç

21 Ekim 2026, Debian 13 VM (QEMU, llvmpipe yazılım GL, headless Wayfire). Gerçek GPU, gerçek
giriş (greetd/TTY) ve fiziksel çoklu monitör denenmedi. Kalite kapısı: build, clippy, fmt,
shellcheck, 117 Rust ve 12 Python testi geçti.

| Adım | Sonuç |
|---|---|
| 1. Compositor yapılandırması | Wayfire dosyayı ayrıştırır. noVNC üzerinden gerçek tuş kodlarıyla Super+Enter terminal, Super+Space menü açtı. Alt+Tab, Super+ok ve medya tuşları elle denenmedi |
| 2. Daemon modülü | Birim testleri geçer. Saydamlık kapatılınca `blur` eklenti listesinden çıktı ve Wayfire dosyayı canlı yeniden okudu |
| 3. Oturum | 10 Python oturum testi geçer (geliştirme günlüğü ve Flatpak öneki dahil). Normal giriş açık |
| 4. Düzenekler | Önizleme (headless Wayfire + wayvnc), smoke (panel, D-Bus, pencere istekleri, %200 ölçek, görev çubuğu tıklaması, temizlik; boşta CPU %0,3, RSS 92 MB), kilit (iki çıkış, ikinci kilitleyicinin reddi, yanlış ve doğru PAM parolası, çökme sonrası kurtarma) ve greeter (normal ve küçük ekran) deneyleri geçti |
| 5. Saydamlık | Menünün arkasındaki yazının kenar şiddeti 36,3; saydamlık açıkken menü üzerinden 0,18 (blur), kapalıyken 0,0 (tam opak) |
| 6. Greeter | Birim testi ve GUI deneyi geçer |
| 7. Flatpak | Flathub'dan kullanıcı kurulumu `org.gnome.Loupe` (GNOME 51): menüde Türkçe adıyla listelenir, menüden D-Bus etkinleştirmesiyle açılır, dosya seçici portalı Türkçe açılır, seçilen dosya belge portalıyla uygulamaya geçer. `GDK_BACKEND=x11` ile bir GTK uygulaması Xwayland üzerinde açıldı |
| 8. labwc kaldırma | Kod, betik, veri ve CI'da labwc başvurusu yok. Geliştirme VM'sinde labwc paketi kurulu duruyor; Asena onu kullanmıyor |
| 9. Belgeler | Güncellendi; tarihî raporlar ve tamamlanmış faz prompt'ları olduğu gibi kaldı |

### Doğrulamada düzeltilenler

- **Opak malzeme tam opak değildi.** Saydamlık kapalıyken panel %94, yüzeyler %97 opaklıktaydı;
  blur kalkınca arkadaki yazı keskin olarak sızıyordu (kenar şiddeti 1,03). `shell.css` artık
  `@window_bg_color` ve `@popover_bg_color` renklerini tam opak kullanır.
- **Kenar paneli açma/kapama yarışı.** Bildirim merkezi açık olup olmadığını revealer'dan okuyordu,
  revealer ise ilk karede idle önceliğiyle açılıyordu. Yavaş çizimde hemen gelen ikinci komut
  yüzeyi kapalı sanıp yeniden açıyor, dışarı tıklama katmanı da görev çubuğu tıklamasını yutuyordu.
  Görünür yüzey artık yalnız kayarak kapanırken kapalı sayılır; birim testleri eklendi.
- **Geliştirme günlükleri kayboluyordu.** Autostart çıktısı `/dev/null`'a gittiği için shell, daemon
  ve kilit günlükleri iç içe oturum günlüğünde yoktu ve kilit deneyi düşüyordu. `ASENA_DEV_LOG`
  eklendi.
- **Flatpak uygulamaları geliştirme oturumunda menüden açılmıyordu.** Loupe gibi D-Bus ile
  etkinleştirilen uygulamaların servis dosyalarını özel veri yolu göremiyordu: Flatpak dizinleri veri
  yolu başladıktan sonra ekleniyordu ve GIO hatayı sessizce geçiyordu. `dev-session.sh` dizinleri veri
  yolundan önce ekler. Gerçek oturumda Debian'ın `60-flatpak` ortam üreticisi bu dizinleri systemd
  kullanıcı veri yoluna zaten verir.

### Açık kalanlar

- Gerçek GPU'da blur maliyeti ve üretim opaklık değerlerinin görsel kabulü.
- Normal giriş zinciri: greetd → asena-session → Wayfire → systemd target.
- `wp-security-context-v1` yok (Flatpak yalıtım farkı).
- ~~Flatpak uygulamalarına Asena vurgu rengi~~: 22 Ekim'de daemon'un Settings portal arka ucuyla çözüldü (K06).
- ~~Harf tuşlu kısayolların TR-F düzenindeki konumu~~: 22 Ekim'de daemon ilk düzene göre çeviriyor (K18).
- GTK3 portal iletişim kutusunun teması (K14).
