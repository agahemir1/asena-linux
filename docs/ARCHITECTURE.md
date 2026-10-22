# Asena — Mimari

Bu belge hedef mimariyi ve Faz 0 kaynak uygulamasını tanımlar. Debian 13 VM üzerinde build/fmt/clippy/test ve iç içe panel denemesi geçti; normal giriş ve fiziksel donanım kabulü açık. Faz sırası, sorumluluk sınırları ve açık teknik kararlar [PLANNING.md](PLANNING.md), gerçek sonuçlar [Faz 0 raporunda](reports/phase-00.md) tutulur.

**Compositor:** 21 Ekim 2026'da kullanıcı onayıyla labwc'den **Wayfire 0.9.0**'a geçildi. Gerekçe, Odak'ın gerçek buzlu cam hedefidir: Wayfire'ın `blur` eklentisi Dual Kawase ile shell yüzeylerinin arkasını bulanıklaştırır ve Debian 13 paketidir. Deney ve ölçümler [blur-options.md](research/blur-options.md), geçiş [wayfire-migration.md](reports/wayfire-migration.md) içindedir.

## Katmanlar

```
┌──────────────────────────────────────────────────────────┐
│ Uygulamalar  (GTK4/libadwaita · APT · Flatpak)           │
├──────────────────────────────────────────────────────────┤
│ Asena süreçleri  (Rust + GTK4/libadwaita)                │
│ asena-shell · asena-daemon · asena-lock · asena-settings │
├────────────────────────────┬─────────────────────────────┤
│ Wayland protokolleri       │ D-Bus servisleri            │
│ layer-shell, toplevel,     │ NetworkManager, BlueZ,      │
│ session-lock, idle …       │ UPower, logind, PipeWire …  │
├────────────────────────────┴─────────────────────────────┤
│ Wayfire  (wlroots tabanlı compositor, Dual Kawase blur)  │
├──────────────────────────────────────────────────────────┤
│ Debian 13 "trixie" · systemd · Mesa · PipeWire           │
└──────────────────────────────────────────────────────────┘
```

## Teknoloji yığını

| Alan | Seçim | Neden |
|---|---|---|
| Dil | Rust (stable) | Bellek güvenliği; uzun süre açık kalan shell süreçlerinde çökme riskini azaltır |
| Arayüz | gtk4-rs + libadwaita-rs | Modern GTK, uyarlanabilir bileşenler, sistem vurgu rengi |
| Shell yüzeyleri | gtk4-layer-shell (Rust crate) | Panel, menü, OSD ve bildirimler için wlr-layer-shell |
| Arayüz tanımı | Blueprint (`.blp`) + CompositeTemplate karmaşık ekranlarda; küçük widget'lar Rust kodunda | |
| D-Bus | zbus (async), glib ana döngüsünde çalışır | Saf Rust, makro ile arayüz tanımı |
| Ses | libpulse-binding + libpulse-glib-binding (pipewire-pulse üzerinden) | glib döngüsüyle doğrudan entegre; yerli PipeWire sonra |
| Wayland istemcisi | wayland-client + wayland-protocols(-wlr) | Toplevel, çıkış yönetimi, gamma, boşta kalma |
| .desktop | GIO DesktopAppInfo + AppInfoMonitor | XDG önceliği ve Flatpak export dizinleri; tarama worker üzerinde |
| Çeviri | gettext-rs, `po/` | |
| Günlük | tracing + tracing-journald | |
| Hata | anyhow (binary), thiserror (kütüphane) | |
| Ayarlar | GSettings (`org.asenalinux.*`) | GTK'ya özgü; Ayarlar uygulaması ve shell anında senkron |

## Hedef sürümler (Debian 13)

18 Ekim 2026'da resmî trixie/main amd64 paket indeksi okundu. 19 Ekim Debian 13.7 VM içinde `apt policy`, kurulu native kütüphaneler ve derleyici de doğrulandı. [Linux kanıtı](reports/evidence/phase-00/environment.txt). [Ham envanter](research/debian13-packages.json), [crate eşlemesi](DEPENDENCIES.md).

| Paket | Debian 13 | Not |
|---|---|---|
| libgtk-4-dev | 4.18.6+ds-2 | gtk4 =0.9.7, feature `v4_18` |
| libadwaita-1-dev | 1.7.6-1~deb13u1 | libadwaita =0.7.2, feature `v1_7` |
| libgtk4-layer-shell-dev | 1.0.4-2 | Rust binding =0.5.0; C session-lock API'si yok, 1.1.0 ile eklenmiş |
| wayfire | 0.9.0-5 | 21 Ekim'den beri compositor; `blur` eklentisi (Dual Kawase), session-lock, foreign-toplevel |
| xwayland | 2:24.1.6-1 | X11 uygulamaları ve X11 kullanan Flatpak'lar için |
| flatpak | 1.16.6-1~deb13u3 | Flatpak uygulamaları; dışa aktarım dizinleri oturum `XDG_DATA_DIRS` içinde |
| rustc / cargo | 1.85.1+dfsg1-1+deb13u1 | Workspace MSRV 1.85.1; kurulu rustc/cargo 1.85.1 doğrulandı |
| greetd | 0.10.3-4 | Faz 4; host'ta kurulmadı/etkinleştirilmedi |
| cage | 0.2.0-2 | Faz 4 |
| xdg-desktop-portal-wlr | 0.7.1-2 | Backend eşlemesi `data/portals/` altında |
| xdg-desktop-portal-gtk | 1.15.3-1 | Tema/Settings portal aktarımı henüz denenmedi |
| blueprint-compiler | 0.16.0-3 | Karmaşık ekranlar sonraki fazlarda |
| fonts-inter | 4.1+ds-1 | Paket mevcut, son tema eşlemesi K14 |
| fonts-jetbrains-mono | 2.304+ds-5 | Paket mevcut |
| wlsunset | 0.3.0-1 | Faz 3 |
| swayidle | 1.8.0-1 | Faz 4 |
| grim | 1.4.0+ds-2+b1 | Faz 7 |
| slurp | 1.5.0-1 | Faz 7 |

Faz 4A dar istisnası: kullanıcı onayıyla native C **1.3.0** ayrı geliştirme
prefix'ine backport edildi. Debian 1.0.4 dosyaları değiştirilmedi. 53 upstream
test ve mevcut panel regresyonu VM'de geçti. GTK 4.18, adwaita 1.7, MSRV ve
o zamanki labwc aynı kaldı. Lock binding 0.1.2, ayrıca native >=1.3.0 build
denetimi kullanır. Dağıtım paketi Faz 6'da ayrıca üretilecek.

## Depo yapısı

```
asena/
├── Cargo.toml                  # workspace; ortak bağımlılıklar ve lint'ler
├── crates/
│   ├── asena-core/             # ortak: GSettings yardımcıları, D-Bus arayüz tanımları,
│   │                           #   i18n, logging, text::fold (Türkçe normalize)
│   ├── asena-services/         # servis modelleri (GObject): ses, ağ, bluetooth, güç,
│   │                           #   parlaklık, MPRIS, logind, accounts
│   ├── asena-wayland/          # wayland-client: foreign-toplevel, output-management,
│   │                           #   gamma, idle-notify (ayrı bağlantı, ayrı thread)
│   ├── asena-widgets/          # paylaşılan GTK widget'ları + temel CSS
│   ├── asena-shell/            # bin
│   ├── asenactl/               # bin
│   ├── asena-session/          # bin
│   ├── asena-daemon/           # bin
│   ├── asena-lock/             # bin
│   ├── asena-greeter/          # bin
│   ├── asena-polkit-agent/     # bin
│   ├── asena-settings/         # bin (libadwaita)
│   ├── asena-welcome/          # bin (libadwaita)
│   └── asena-screenshot/       # bin (libadwaita)
├── data/
│   ├── schemas/                # org.asenalinux.*.gschema.xml
│   ├── wayfire/                # wayfire.ini: eklentiler, kısayollar, blur, başlık, girdi
│   ├── sessions/               # asena.desktop  → /usr/share/wayland-sessions/
│   ├── systemd/                # asena-session.target ve user servisleri
│   ├── portals/                # asena-portals.conf
│   ├── style/                  # shell CSS (gresource ile gömülür)
│   ├── icons/
│   └── wallpapers/
├── po/                         # tr.po (+ diğer diller), POTFILES
├── scripts/                    # setup-dev.sh, dev-session.sh, update-po.sh
├── packaging/debian/           # .deb paketleme
├── iso/                        # live-build yapılandırması
├── docs/
└── prompts/
```

## Süreç modeli

| Süreç | Neden ayrı / birleşik |
|---|---|
| `asena-shell` (panel, menü, hızlı ayarlar, bildirimler, OSD, duvar kâğıdı) | **Tek süreç:** durum paylaşımı kolay, tek GTK başlatma, düşük bellek. Çökerse systemd 1 sn içinde yeniden başlatır (`Restart=on-failure`). |
| `asena-lock` | **Ayrı:** güvenlik kritik, küçük ve basit kalmalı. |
| `asena-daemon` | **Ayrı:** arayüzsüz; shell çökse de ayarlar uygulanmaya devam eder. |
| `asena-polkit-agent` | **Ayrı:** shell yeniden başlarken kimlik doğrulama kesilmemeli. |
| `asena-settings`, `asena-welcome`, `asena-screenshot` | Normal uygulamalar; tek örnek (single instance), D-Bus ile etkinleştirilebilir. |

## Oturum başlangıcı

```
greetd
 └─ cage -s -- asena-greeter          (giriş ekranı)
     └─ kullanıcı "Asena" oturumunu seçer
         └─ asena-session             (/usr/share/wayland-sessions/asena.desktop)
             ├─ ortam: XDG_CURRENT_DESKTOP=Asena, XDG_SESSION_TYPE=wayland,
             │         QT_QPA_PLATFORM="wayland;xcb", _JAVA_AWT_WM_NONREPARENTING=1
             └─ wayfire -c ~/.config/asena/wayfire.ini
                 └─ autostart eklentisi (session-autostart.sh):
                     dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP …
                     systemctl --user start asena-session.target
                         ├─ asena-shell.service
                         ├─ asena-daemon.service
                         ├─ asena-polkit-agent.service
                         └─ xdg-desktop-autostart.target   (~/.config/autostart)
```
- `asena-session.target` için `BindsTo=graphical-session.target` kullanılır.
- Cargo `asena-session` binary'si gömülü POSIX `scripts/asena-session` betiğini çalıştırır; betik ortamı ayarlar, ilk girişte `/usr/share/asena/wayfire/wayfire.ini` dosyasını `~/.config/asena/wayfire.ini` olarak kopyalar ve Wayfire'ı bu dosyayla başlatır. Wayfire'ın `autostart` eklentisi `session-autostart.sh` ile systemd target'ını açar. Wayfire'ın kapanış kancası yoktur; Wayfire çıkınca oturum betiği target'ı durdurur. Target'taki `DefaultDependencies=no`, açık `Before=graphical-session.target` sırasını örtük target sırasıyla döngüye sokmaz. Normal giriş yolu henüz gerçek girişte denenmedi; kurulum Faz 6'dadır.
- Geliştirmede ayrı D-Bus, ayrı süreç grubu ve geçici config kopyası kullanılır. Gerçek systemd user ortamına dokunulmaz. Ayrıntılar [DEVELOPMENT.md](DEVELOPMENT.md).
- Oturumu kapatma: shell logind `TerminateSession` ister; Wayfire çıkar ve target ile systemd user servisleri temizlenir.
- Wayfire yapılandırması `~/.config/asena/wayfire.ini` dosyasıdır ve **asena-daemon tarafından GSettings'ten üretilir.** Dosyanın başında "otomatik üretildi" uyarısı bulunur. Wayfire dosyayı izler; daemon dosyayı atomik olarak değiştirince yeniden okur, bu yüzden ayarlar ilk oturumdan itibaren canlı uygulanır.

## IPC

### D-Bus (oturum veri yolu)
| Ad | Sahibi | Amaç |
|---|---|---|
| `org.asenalinux.Shell` (`/org/asenalinux/Shell`) | asena-shell | `ToggleMenu`, `ToggleQuickSettings`, `ToggleNotificationCenter`, `ShowOsd(kind, value)`, `ReloadStyle` — `asenactl` bunu çağırır |
| `org.freedesktop.Notifications` | asena-shell | Bildirim sunucusu |
| `org.kde.StatusNotifierWatcher` | asena-shell | Sistem tepsisi |
| `org.asenalinux.Settings` | asena-settings | Belirli bir paneli açma (`asena-settings <panel-id>`) |

Faz 4 kaynak sözleşmesi: `org.asenalinux.Daemon` aynı yoldaki `Lock(scope)` ve
`Idle(scope, action)` isteklerini karşılar. `scope`, Wayland socket yolunun ve
login kimliğinin kayıpsız kodlanmış biçimidir; farklı oturum istekleri reddedilir.
Bu aynı kullanıcıya karşı bir yetkilendirme sınırı değildir. `LockAvailable`
native protokol ve güvenilir PAM politikası ön denetimidir; asıl başarı
`org.asenalinux.Lock.WaitReady(scope)` compositor onayından gelir.
`SetSleepPending(scope, bool)` hazırlık boyunca PAM sonucunun kilidi açmasını
engeller. Bekletme bir unique D-Bus sahibine bağlıdır; sahibin kaybı ve logind
uyku durumu birlikte değerlendirilir. Kilit açma önce hazır durumunu atomik
olarak kaldırır. [Uyku sözleşmesi](research/idle-sleep.md),
[gerçek doğrulama durumu](reports/phase-04.md).

Daemon'un D-Bus sözleşmesi `asena-core::dbus` içindedir (`org.asenalinux.Daemon`, `/org/asenalinux/Daemon`). Bu arayüz ayrıcalık yükseltmez; yetkili işler mevcut sistem servislerine gider.

**Ekran düzeni (Faz 5D).** Test, uygulama, onay ve geri almanın tek sahibi daemon'dır. Model `asena_core::displays` içindedir: modlar fiziksel piksel, konumlar mantıksal pikseldir; geçerli düzen bağlı ekranların hepsini içerir, en az bir ekranı açık tutar, desteklenen mod ve ölçeği kullanır, ekranları üst üste bindirmez ve aralarında boşluk bırakmaz.

| Yöntem / sinyal | İş |
|---|---|
| `DisplayLayout() → s` | JSON `displays::Layout`: kullanılabilirlik, bekleyen değişiklik, ekranlar |
| `ApplyDisplays(scope, layout) → u` | JSON `Config` listesi. Doğrular, compositor'da test eder, uygular ve onay için kalan saniyeyi (15) döndürür; değişiklik yoksa 0 |
| `ConfirmDisplays(scope)` / `RevertDisplays(scope)` | Bekleyen değişikliği korur ve kaydeder ya da geri alır |
| `DisplaysChanged` | Düzen veya bekleme durumu değişti |

Uygulayan D-Bus bağlantısı veri yolundan ayrılırsa (Ayarlar kapanır veya çöker), süre dolarsa ya da bağlı ekran takımı değişirse daemon önceki düzeni hâlâ bağlı ekranlara geri yükler. Bekleyen bir değişiklik varken ikinci istek reddedilir. Ekran kimliği üretici, model ve seri numarasıdır; seri yoksa bağlantı noktası da eklenir. Onaylanan düzen ekran takımı başına `~/.config/asena/displays.json` dosyasına atomik yazılır (en çok 16 takım; geliştirmede geçici yapılandırma dizini) ve daemon başlarken ya da tam o ekranlar bağlanınca yeniden uygulanır. Birincil ekran shell'in `panel-monitor` tercihidir: masaüstü simgeleri ve geçici yüzeyler orada açılır.

Ayarlar için ayrı IPC yazılmaz: GSettings değişiklik sinyalleri yeterli.

### Wayland protokolleri
19 Ekim Debian labwc 0.8.3 iç içe oturumunda `wayland-info` alındı: [kanıt](reports/evidence/phase-00/protocols.txt). 21 Ekim Wayfire 0.9.0 envanteri ve labwc ile farkı: [protocols-compare.txt](reports/evidence/k13-wayfire/protocols-compare.txt). Asena'nın kullandığı protokollerin hepsi Wayfire'da vardır; `xdg-activation-v1` için `xdg-activation` eklentisi yüklenir. Wayfire'da `wp-security-context-v1` yoktur (Flatpak notu aşağıda). Aşağıdaki sürümler ilan edilen global sürümleridir; donanım yeteneği veya kilit güvenlik kabulü değildir. [Araştırma ve kaynaklar](research/blur-and-lock.md).

| Protokol | Kullanan | 0.8.3 çalışma zamanı |
|---|---|---|
| wlr-layer-shell-unstable-v1 | shell | v4, panel yüzeyleri denendi |
| wlr-foreign-toplevel-management-unstable-v1 | shell | v3, istemci Faz 1 |
| ext-session-lock-v1 | lock | v1, Debian GTK binding yolu açık (K03) |
| ext-idle-notify-v1 | daemon | v1 |
| wlr-output-management-unstable-v1 | daemon (değiştirir), shell (yalnız okur) | v4; Asena v3'e kadar bağlanır |
| wlr-output-power-management-unstable-v1 | daemon | v1 |
| wlr-gamma-control-unstable-v1 | daemon | v1, donanımda ayrıca denenecek |
| wlr-screencopy-unstable-v1 | screenshot | v3, pencere geometrisi kanıtı değil |
| ext-image-copy-capture-v1 | screenshot | İlan edilmiyor |
| wlr-data-control-unstable-v1 | shell (Sonra) | v2 |
| ext-data-control-v1 | shell (Sonra) | İlan edilmiyor |
| ext-workspace-v1 | shell (Sonra) | Wayfire 0.9.0'da yok (labwc 0.8.3 v1 ilan ediyordu) |
| ext-background-effect-v1 | shell hedefi | İlan edilmiyor; blur Wayfire `blur` eklentisinde (K13) |

**Bilinen yukarı akış hataları (Debian 13):**
- GTK 4.18.6 `apply_monitor_change`, xdg-output mantıksal boyutu fiziksel moda eşitse (yani ölçek 1'de) koordinatları önceki ölçekten türetilmiş tam sayı ölçeğe böler. Ölçek 1'e dönünce açık istemciler ekranı yanlış boyut ve ölçekte görür; GTK 4.20 bunu kaldırdı. Shell bu yüzden yüzey boyutu, tepsi menüsü konumu ve duvar kâğıdı çözünürlüğü için geometriyi `wlr-output-management`'tan okur (`asena-shell/src/outputs.rs`); GTK değeri yalnız yedektir.
- wlroots 0.18.2 (0.19.0'da da), mod listesi olmayan bir çıkış (headless, iç içe) kapalıyken bağlanan bir istemci onu açarsa ve başka bir istemci çıkışları izliyorsa `head_send_state` içindeki `assert(found)` ile compositor'ı düşürür. Yukarı akış düzeltmesi `40640950` (istemci başına denetim) kullanıcı onayıyla 22 Ekim'de `packaging/wlroots/` altına alındı; `scripts/build-wlroots.sh` Debian paketini `0.18.2-3+asena1` olarak yeniden derler. Yamalı kitaplıkla VM'de üç tur `wlr-randr --off/--on` sorunsuz geçti. Mod listesi olan DRM çıkışları bu yola girmez. Düzenekler yamasız sistemde de çalışsın diye çıkışı daemon üzerinden kapatıp açar; smoke, yamalı paket kuruluysa `wlr-randr` yolunu ayrıca dener.

## İş parçacığı ve asenkron model
- **GTK ana thread'i asla bloklanmaz.** Senkron dosya/ağ/D-Bus çağrısı yapılmaz.
- D-Bus: zbus async; future'lar `glib::spawn_future_local` ile ana bağlamda çalışır.
- Bloklayan iş: `gio::spawn_blocking`.
- **Wayland istemcisi (`asena-wayland`):** GTK'nın bağlantısından ayrı, kendi thread'inde açılan ikinci bir `wl_display` bağlantısı. Olaylar `async_channel` ile ana döngüye taşınır, istekler aynı yolla geri gider.
- `asena-services` içindeki her servis bir **GObject singleton**'dır: durum property, olay signal olarak sunulur. Widget'lar `bind_property` ve expression'larla bağlanır; servis kodu GTK widget'ı bilmez.
- Servis bulunamazsa (D-Bus adı yok, donanım yok) `available = false` olur ve arayüz öğeyi gizler.
- Kilit hazır durumu `asena-lock` tarafından yalnız compositor kilitlendi onayından sonra bildirilir. Uyku koordinasyonunda daemon bu onayı bekler; bir sürecin başlatılmış olması kilit kanıtı değildir. GTK session-lock kütüphanesi/binding seçimi Faz 0 araştırması ve Faz 4 karar kapısıdır.

## Yapılandırma (GSettings)
| Şema | İçerik |
|---|---|
| `org.asenalinux.shell` | `pinned-apps`, `menu-pinned-apps`, `menu-layout`, `panel-position`, `panel-size`, `panel-icon-size`, `taskbar-alignment`, `clock-show-seconds`, `panel-on-all-monitors`, `panel-monitor` |
| `org.asenalinux.desktop` | `color-scheme`, `accent-color`, `background-uri`, `background-mode` |
| `org.asenalinux.notifications` | `do-not-disturb`, `disabled-apps` |
| `org.asenalinux.power` | Boşta kalma süreleri, kapak davranışı |
| `org.asenalinux.night-light` | Açık/kapalı, sıcaklık, zamanlama |
| `org.asenalinux.input` | Klavye düzenleri, fare ve dokunmatik yüzey seçenekleri |

GTK ve libadwaita uygulamalarının uyması için `asena-daemon` şu anahtarları da yazar: `org.gnome.desktop.interface` → `color-scheme`, `accent-color`, `font-name`, `monospace-font-name`, `icon-theme`, `clock-format`.

### Menü ve panel kişiselleştirme sözleşmesi (Faz 1–2 altyapısı, Faz 5 arayüzü)
- `menu-layout`: `compact-list` (Odak varsayılanı), `classic`, `grid`. Menü sunumları aynı arama/uygulama modelini ve sabitleme listesini kullanır.
- `panel-position`: `bottom` (varsayılan), `top`, `left`, `right`. `taskbar-alignment` anlamsal `start`/`center`/`end` değerleriyle yatay/dikey düzende yorumlanır; üretimde eski şema varsa veri koruyan geçiş gerekir.
- `panel-size` ve `panel-icon-size`: mantıksal piksel; geçerli aralık ve eşleşme T0 boyut tasarımında belirlenir. Yatay panelde yükseklik, dikey panelde genişlik anlamına gelir.
- `panel-on-all-monitors` ve `panel-monitor`: tercih edilen monitör kararlı kimlikle saklanır. Monitör kaybolursa shell erişilebilir ekrana döner; geçici geri dönüş kullanıcının tercih ettiği ekran kimliğini gereksiz yere silmez.
- Shell'de tek yerleşim modeli panel yönü, anchor, exclusive zone, çalışma alanı ve popup/OSD konumlarını türetir. Faz 5 arayüzü bu modeli canlı yapılandırır; farklı widget'larda birbirinden kopuk koordinat sabitleri kullanılmaz.
- Kenar değişimi eski exclusive zone'u temizler; yeni yüzey hazır olduğunda yeni kenarı ayırır. Birden fazla pencere/monitörün ara durumunu yönet; değişiklik sırasında klavye odağı veya tüm panel erişimi kaybolmaz.
- Kişiselleştirme işlemi son çalışan düzeni korur; yalnız menü/panel ayarlarına uygulanan varsayılana dön eylemi diğer tercihleri değiştirmez. Otomatik gizleme sonraki adaydır.

> **Settings portalı (22 Ekim):** `xdg-desktop-portal-gtk` 1.15.3 `accent-color` için `NotFound` döndürdüğünden `asena-daemon` kendi Settings arka ucunu sunar (`org.freedesktop.impl.portal.desktop.asena`, `/org/freedesktop/portal/desktop`, `portal.rs`). `org.freedesktop.appearance` altında `color-scheme` ve `accent-color` değerlerini doğrudan Asena tercihlerinden verir, değişince `SettingChanged` yayar; `contrast` ve diğer anahtarlar gtk arka ucuna düşer (`asena-portals.conf`: `org.freedesktop.impl.portal.Settings=asena;gtk`). Vurgu, libadwaita'nın kendi dokuz rengiyle gönderildiği için Flatpak libadwaita uygulamalarında birebir eşleşir. Kurulumda `asena.portal` `/usr/share/xdg-desktop-portal/portals/` altına, etkinleştirme dosyası (`SystemdService=asena-daemon.service`) D-Bus servis dizinine gider. VM'de Flathub Loupe düğmesi Asena morunu aldı ve uygulama açıkken camgöbeğine geçti.

## Tema zinciri
1. **Shell süreçleri:** `AdwStyleManager` ile renk şeması ve vurgu otomatik gelir. Özel CSS `data/style/` altında, yalnızca libadwaita adlandırılmış renkleriyle yazılır.
2. **Wayfire pencere başlıkları:** yalnız sunucu tarafı başlık isteyen istemciler (foot, Qt) için `decoration` eklentisi çizer. Wayfire başlık yazısını hep beyaz çizdiği için başlık renkleri iki temada da koyudur. GTK uygulamaları kendi başlıklarını çizer (`preferred_decoration_mode = client`).
3. **GTK3 uygulamaları:** adw-gtk3 teması (Debian'da yoksa Asena deposunda paketlenir).
4. **Simgeler:** `Asena` (çizgi simgeler, teal klasörler) ve vurgu başına `Asena-<vurgu>` (klasörler, Dosyalar ve Ayarlar). Tema adı vurgu tercihinden türetilir; Asena süreçleri GTK ayarına, gerçek oturumda daemon `org.gnome.desktop.interface icon-theme` anahtarına yazar. Ayrıntı [ICONOGRAPHY.md](ICONOGRAPHY.md).
5. **Qt uygulamaları:** Sonra değerlendirilecek.

Bu zincirin varsayılan ürün görünümü Asena Odak'tır. Kesin GTK3 teması, simge ve imleç seti K14'te seçilir; adw-gtk3 adaydır. Renkli uygulama varlıkları ile sembolik arayüz simgeleri ayrı ailelerdir. Cam malzemesi `org.asenalinux.desktop transparency` açıkken ve yüksek kontrast istenmediğinde kullanılır; aksi halde shell opak malzemeye döner ve daemon Wayfire `blur` eklentisini yüklemez.


## Faz 1 uygulama ayrıntıları

`asena-wayland::toplevel` ayrı bağlantı/thread kullanır. Wayland socket hazır
olması ile komut kanalı asenkron yarışır; boşta periyodik polling yoktur.
Güncellemeler yalnız `done` ile aktarılır, `closed` handle'ı destroy eder.
`asena-services` içindeki Toplevel GObject ve ToplevelModel gerçek gio::ListModel
uygulamasıdır. GTK içermez. AppCatalog, GIO uygulama önbelleği değişince worker'da
yenilenir; eski tarama cevabı yeni nesli ezmez.

GApplication `NON_UNIQUE` çalışır; tek örnek kimliğini zbus public bus adı sağlar.
Aynı adı iki ayrı bağlantıdan almaya çalışılmaz. CLI ve sunucu ortak proxy
sözleşmesini kullanır. ShowOsd değerleri doğrulanır; görsel sunum Faz 2'dir.
Uygulama başlatırken GDK launch context'ten activation token alınır; dosya tabanlı
DesktopAppInfo yüklemesi ve gerçek launch worker'da yapılır. Token worker'ın yerel
GIO launch context'ine aktarılır; global process ortamı değiştirilmez.

Arka plan çözme/ölçekleme worker'da yapılır; ana döngüye yalnız piksel verisi gelir.
Eski istek nesli yeni görüntüyü ezemez. Crossfade 300 ms; eski paintable geçiş
sonunda bırakılır. Kaynak dosya ve çıktı piksel sınırı uygulanır. Solid/eksik
resim semantik tema rengine döner. Bu yüzey blur algoritması değildir.

## Faz 2–3 uygulama sözleşmeleri

Notifications 1.2 ve StatusNotifierWatcher adları mevcut sahibinden devralınmaz.
Bildirim markup'ı ve görüntüleri sınırlandırılıp temizlenir; geçmiş bellekte 100
öğe, popup kuyruğu en fazla üç görünür öğe tutar. Menü/merkez/hızlı ayarlar ortak
surface ile klavye sahipliğini görünürlükte alıp saklanırken bırakır. Kullanım
sayacı dosya IO'su ve uygulama aramaları worker üzerindedir.

Servis modelleri GTK içermeyen GObject'lerdir. PropertyModel ve ObjectModel
başlangıç, PropertiesChanged, ObjectManager ve sahip değişimi olaylarını izler;
eski sahipten gelen sonuçlar işlem nesliyle elenir. Pulse istemcisi ayrı GLib
context/thread kullanır. Ses başarısı yalnız native callback değil, istenen
değerin sonraki snapshot'ta gözlenmesiyle döner. Parlaklık yazısı logind'e gider.
MPRIS ve SNI native D-Bus yollarını kullanır. Ayrı daemon görünüm, Asena compositor
config'i ve tek wlsunset çocuğunu yönetir; dev oturumunda native görünüm/gamma
yazısı kapalıdır. [Gerçek kabul durumu](reports/phase-03.md).

## 21 Ekim 2026 düzeltmeleri ve yeni sözleşmeler

- **Uygulama çatısı:** `AsenaApplication::new` paylaşılan stil, simge teması ve tercihleri
  (`asena_widgets::init`) artık `startup` sinyalinde yükler; daha önce GTK başlatılmadan çağrıldığı için
  bütün uygulamalar açılışta paniğe düşüyordu. `connect_open` dosya açma isteklerini iletir.
  `asena_app::storage` atomik dosya yazımı ve `$XDG_DATA_HOME/asena/<uygulama>` veri dizini sağlar.
- **D-Bus etkinleştirme ortamı:** iç içe geliştirme oturumu kendi özel oturum veri yolunu kullanır.
  `session-autostart.sh` bu veri yoluna `dbus-update-activation-environment` ile (systemd'ye dokunmadan)
  `WAYLAND_DISPLAY`, `XDG_CURRENT_DESKTOP`, `XDG_DATA_DIRS` vb. aktarır. Aksi halde xdg-desktop-portal dış
  compositor'e bağlanıyor ve Firefox gibi uygulamaların dosya/yazdırma diyalogları görünmüyordu.
  `asena-portals.conf` geliştirmede `target/share/xdg-desktop-portal` altına kopyalanır.
- **Layer-shell yüzeyleri ve portal:** shell, portal kullanan API'lere (`gtk::FileLauncher`,
  `gtk::UriLauncher`) üst pencere olarak layer-shell yüzeyi vermez. GTK portal için üst pencereyi
  xdg-foreign ile dışa aktarmaya çalışır; compositor yalnız xdg_toplevel kabul ettiğinden shell protokol
  hatasıyla düşüyordu.
- **Compositor yapılandırması (Wayfire):** `asena-daemon`'un `wayfire` modülü `data/wayfire/wayfire.ini`
  şablonunu ve `org.asenalinux.input` ayarlarını birleştirir: `[input]` altında `xkb_layout`,
  `xkb_options`, `kb_repeat_rate`, `kb_repeat_delay`, işaretçi hızı, doğal kaydırma ve dokunarak tıklama.
  `org.asenalinux.desktop` içindeki `transparency` kapalıysa `blur`, `enable-animations` kapalıysa
  `animate` eklentisi listeden çıkar. Değişiklikten 400 ms sonra dosya atomik olarak değiştirilir ve
  Wayfire kendisi yeniden okur. Gerçek oturumda hedef `~/.config/asena/wayfire.ini`, geliştirmede
  `ASENA_WAYFIRE_CONFIG` geçici kopyasıdır. Kısayollar (`command` eklentisi), Alt+Tab
  (`fast-switcher`), Super+ok tuşlarıyla yaslama (`grid`) ve 180 ms soluklaşma (`animate`) şablondadır.
- **Flatpak:** oturum `XDG_DATA_DIRS` değişkenine kullanıcı ve sistem Flatpak dışa aktarım dizinlerini
  ekler; menü Flatpak uygulamalarını listeler. Portallar `asena-portals.conf` ile gtk (dosya seçici,
  ayarlar) ve wlr (ekran görüntüsü, ekran paylaşımı) arka uçlarına gider; X11 kullanan Flatpak'lar
  Xwayland ile çalışır. Wayfire `wp-security-context-v1` sunmadığından Flatpak uygulamaya olağan Wayland
  soketini verir: uygulamalar çalışır ama compositor sandbox içindeki istemciyi ayrıcalıklı
  protokollerden ayıramaz. labwc 0.8.3 bu ayrımı yapabiliyordu; bu yalıtım farkı açık bir adaydır.
- **Oturum dili:** daemon `org.asenalinux.session language` değerini
  `~/.config/environment.d/90-asena-language.conf` dosyasına yazar; systemd kullanıcı servisleri ve
  oradan başlatılan uygulamalar bir sonraki girişte okur. Geliştirme oturumunda yazılmaz.
- **Ekran görüntüsü:** `asena_core::screenshot` `org.freedesktop.portal.Screenshot` çağırır (Wayfire'da
  xdg-desktop-portal-wlr screencopy ile yakalar), görüntüyü `~/Resimler/Ekran Görüntüleri` altına taşır. Ekran Görüntüsü
  uygulaması ve `asenactl screenshot` aynı yolu kullanır.
- **Simge teması:** `data/icons/Asena` (sembolik) ve `data/icons/hicolor` (uygulama). Asena süreçleri
  vurguya göre `gtk-icon-theme-name` (`Asena` veya `Asena-<vurgu>`) ve `gtk-decoration-layout=appmenu:minimize,maximize,close` ayarlar;
  üçüncü taraf GTK uygulamaları için `data/gsettings/90_asena.gschema.override` paketle kurulur.
- **Vurgu rengi:** libadwaita 1.7'de vurgu portaldan gelir; Asena süreçleri tercih edilen rengi
  `--accent-bg-color: var(--accent-<renk>)` sağlayıcısıyla uygular.
- **Kenar paneli:** `Surface::sidebar` ekranın sondaki kenarında tam yükseklikte, `GtkRevealer` ile
  kayan bir yüzeydir; bildirim merkezi bunu kullanır.
