# Bağımlılıklar

18 Ekim 2026: resmî Debian indeksi ve yayımlanmış crate manifestleri incelendi.
Bu inceleme Cargo çözümlemesi, Linux derlemesi veya geçişli lisans denetimi değildir.
Ham envanter: [Debian paketleri](research/debian13-packages.json),
[crate manifestleri ve güncelleme tarihleri](research/crates.json).

MSRV `1.85.1`, Debian 13 rustc paketindeki upstream sürümdür. Debian paket revizyonu
Cargo `rust-version` alanına yazılmaz. Resolver 3 ve uyumsuz Rust sürümlerinde
`fallback` seçimi kullanılır. Metadata eksikleri ve geçişli bağımlılıklar nedeniyle
Linux üzerinde üretilen Cargo.lock Rust 1.85.1 ile derlendi; dağıtım lisans denetimi ayrı tutulur. Yeni GTK API sınırı yükseltilmedi.

| Bağımlılık | Seçim | Kullanım gerekçesi | Yayımlanmış lisans |
|---|---|---|---|
| gtk4 | =0.9.7, v4_18 | Debian GTK 4.18 arayüzü | MIT |
| libadwaita | =0.7.2, v1_7 | Aynı gtk4 0.9 / GLib 0.20 ailesi | MIT |
| gtk4-layer-shell | =0.5.0 | Aynı GTK ailesi; C 1.0 tabanı, boş panel | MIT |
| glib, gio | =0.20.12 | Ana döngü, kaynak ve GSettings | MIT |
| glib-build-tools | =0.20.0 | GResource derlemesi | MIT |
| zbus | =5.5.0, async-io | İleride asenkron D-Bus; MSRV 1.77 | MIT |
| gettext-rs | =0.7.2, gettext-system | Sistem gettext'i, tek çeviri domain'i | MIT |
| tracing, tracing-subscriber | 0.1, 0.3 | Yapılandırılabilir günlük ve stderr | MIT |
| tracing-journald | 0.3 | Journal kullanılabiliyorsa yerel kayıt | MIT |
| anyhow, thiserror | 1, 2 | Binary bağlamı ve türlenmiş kütüphane hataları | MIT / Apache-2.0 seçenekleri |
| async-channel | 2 | Gelecekte thread / ana döngü köprüsü | MIT / Apache-2.0 seçenekleri |
| unicode-normalization | 0.1 | NFD ve birleşik aksanların güvenli ayrıştırılması | MIT / Apache-2.0 seçenekleri |

Doğrudan lisanslar GPL-3.0-or-later proje lisansıyla kullanılabilir niteliktedir;
dağıtım öncesi lock dosyasındaki tüm geçişli lisanslar ve bildirimler incelenecek.
Crate metadata'sında proje güncellemeleri mevcut; seçilen eski GTK ailesinin uzun
süreli bakım garantisi yoktur. Layer-shell wrapper kendini pasif bakımlı, üretilmiş
ve özellikleri tamamlanmış bir binding olarak tanımlar. Yeni sürüme geçiş otomatik
değildir: GTK ailesi, MSRV ve Debian C API sınırı birlikte yeniden doğrulanır.

zbus ve async-channel workspace sözleşmesinde hazırdır; Faz 0'da sahte D-Bus
metotları veya anlamsız worker süreçleri başlatılmaz. Boş bileşen binary'leri
çalıştırılırsa çevrilebilir bir uygulanmadı hatası ve sıfır olmayan kod verir;
özellikle lock hiçbir zaman kilitlenmiş gibi davranmaz.

Bu klasörde Git deposu bulunmadığından commit oluşturulmadı. Bağımlılık seçimlerinin
gerekçesi bu belgede saklandı; ilk `feat(core)` commit mesajında buraya başvurulmalı.


## Faz 1 eklemeleri

[Kontrol edilen metadata](research/phase1-crates.json): wayland-client 0.31.11 ve
wayland-protocols-wlr 0.3.9 (MIT, MSRV 1.65), async-io 2.6.0 (MIT/Apache-2.0,
1.71), futures-lite 2.6.1 (MIT/Apache-2.0, 1.60), clap 4.5.51
(MIT/Apache-2.0, 1.74). Upstream güncelleme tarihleri, yanked ve lisans alanları
incelendi. Gerekçeler sırasıyla standart toplevel protokolü, hazır socket/kanal
bekleme ve belgelenen CLI grameridir. GTK API sınırı değişmedi. Yeni doğrudan
.desktop crate'i eklenmedi; mevcut GIO XDG uygulama önbelleği kullanılıyor.

Linux `cargo metadata --locked --offline --filter-platform x86_64-unknown-linux-gnu`
179 paket verdi. Eksik lisans alanı yok; 22 paket MSRV metadata'sı bildirmiyor.
Build Rust 1.85.1 ile geçti; metadata eksikliği derleme kanıtıyla karıştırılmaz.
Lisans ifadeleri MIT, Apache-2.0 (bazılarında LLVM exception), Unicode-3.0 ve
seçenekli Unlicense/Zlib ailesindedir; proje crate'leri GPL-3.0-or-later'dır.
Dağıtımda gerekli üçüncü taraf bildirimleri Faz 6'da paketlenir.

## Faz 2 eklemeleri

[Metadata](research/phase2-crates.json): serde 1.0.228 (MSRV 1.56), serde_json
1.0.145 (1.61), quick-xml 0.37.5 (1.56). Serde ailesi MIT/Apache-2.0, quick-xml
MIT lisanslıdır; GPL-3.0-or-later ile kullanılabilir. Upstream projeleri bakım
görüyor; seçilen sürümler sabitlenmiştir, en yeni sürüm oldukları iddia edilmez.
JSON, worker üzerinde kullanım sayacını okumak/yazmak; XML okuyucu, dış kaynaklı
bildirim markup'ını beyaz listeyle ayrıştırmak için eklendi. Native GTK/adwaita
API sınırı yükseltilmedi. Git deposu bulunmadığından gerekçeler commit yerine
burada tutuluyor.

## Faz 3 ses kitaplıkları

[Metadata](research/phase3-crates.json): libpulse-binding 2.30.1 ve
libpulse-glib-binding 2.29.0, MSRV 1.63, MIT/Apache-2.0. Bakımı süren aynı upstream
projesinden gelir; GPL-3.0-or-later ile kullanılabilir. PipeWire'ın Pulse uyumluluk
sunucusuna native olay tabanlı bağlantı ve ayrı GLib context için eklendi.
GLib 0.20 ailesi Cargo çözümünde korundu. Debian libpulse-dev 17.0 kullanılıyor.
GTK/adwaita/layer-shell API sınırları değişmedi. Commit olmadığı için gerekçe
bu belgede saklanıyor.

## Faz 4A kilit ve PAM

[Metadata](research/phase4-crates.json): gtk4-session-lock 0.1.2 (MIT, MSRV 1.70)
GLib 0.20/GTK 0.9 ailesini korur. Upstream'in yeni binding ailesine geçilmedi.
Native C 1.3.0 MIT kaynağı checksum ile sabitlendi; dar API istisnası kullanıcı
tarafından onaylandı. Build ayrıca pkg-config >=1.3.0 ister; sys crate'in >=1
denetimi tek başına yeterli sayılmaz.

pam-client2 0.5.5, 2026'da güncellenen PAM istemci fork'udur (MPL-2.0, GPL-3.0
ile birlikte kullanılabilir). Eski pam-client 0.5.0 karşılaştırıldı, eklenmedi.
`default-features=false` ile CLI parola okuyucusu alınmaz. Kendi ayrıcalıklı
daemon'ımız veya yerel unsafe FFI eklenmeden PAM conversation sağlamak için
seçildi. PAM worker üzerinde çalışır. zeroize 1.8.1 (MIT/Apache-2.0, MSRV 1.60)
uygulamanın tuttuğu cevapları temizlemek içindir. GTK/PAM/native wrapper içindeki
geçici kopyalar ayrı inceleme konusudur; bütün süreç belleğinin silindiği iddia
edilmez. Bu gerekçeler Git deposu olmadığı için commit yerine burada tutulur.

## Faz 4C greetd protokolü

greetd_ipc 0.10.3, Debian 13 greetd ile aynı sürümdeki upstream protokol
tanımları için seçildi. [Metadata](research/greetd-crate.json),
[upstream](https://git.sr.ht/~kennylevinsen/greetd/),
[Debian sözleşmesi](https://manpages.debian.org/trixie/greetd/greetd-ipc.7.en.html).
Crate'in son yayını 2024'tür; Debian'ın bakımı süren greetd paketinin protokol
kaynağıdır. GPL-3.0-only, projenin GPL-3.0 seçeneğiyle uyumludur; dağıtım lisans
bildirimi bu kısıtı korumalıdır. MSRV metadata'sı yok; Rust 1.85.1 derlemesi ve
51 Rust testli kalite kapısı geçti. GTK/GLib bağımlılığı veya ek async runtime getirmez.

Hazır codec yerine aynı Request/Response enum'ları ve sınırlandırılmış worker
çerçevelemesi kullanılır. Mevcut async-io/futures-lite bağımlılıkları worker'da
5 saniyeyle sınırlı bağlantı içindir. Böylece soketten gelen boyut 64 KiB'ı geçmeden kontrol
edilir; parola içeren serileştirme tamponu temizlenir. Socket/PAM bekleyişi GTK
dışındadır. Git deposu olmadığı için ekleme gerekçesi burada tutulur.

## Çalışma zamanı paketleri (21 Ekim 2026)
Yeni Rust crate'i eklenmedi. Aşağıdaki Debian 13 paketleri `scripts/setup-dev.sh` listesine eklendi ve
Faz 6 paket bağımlılıklarına taşınacak:

| Paket | Kullanan | Neden |
|---|---|---|
| `libgtk-4-media-gstreamer` | Oynatıcı | `gtk::MediaFile` için GTK medya arka ucu |
| `gstreamer1.0-plugins-good` | Oynatıcı | Yaygın ses/video biçimleri |
| `libarchive-tools` | Arşiv | `bsdtar` ile ZIP/TAR/7z listeleme ve çıkarma |

`futures-lite` (çalışma alanında zaten vardı) `asena-core`'a ekran görüntüsü portal yanıtını beklemek
için eklendi; `zbus` `asena-screenshot`'a eklendi. İkisi de çalışma alanı sürümüyle aynıdır.

## Compositor geçişi (21 Ekim 2026)
labwc yerine Wayfire kullanılır. Yeni Rust crate'i eklenmedi. `scripts/setup-dev.sh` değişiklikleri:

| Paket | Sürüm | Neden |
|---|---|---|
| `wayfire` | 0.9.0-5 | Compositor; Dual Kawase `blur`, session-lock, foreign-toplevel, xdg-activation eklentileri. MIT lisanslı |
| `xwayland` | 2:24.1.6-1 | X11 uygulamaları ve X11 kullanan Flatpak'lar (`[core] xwayland = true`) |
| `flatpak` | 1.16.6-1~deb13u3 | Flatpak uygulamaları; dışa aktarım dizinleri oturum `XDG_DATA_DIRS` içinde |
| `wlrctl`, `wtype` | Debian 13 | Smoke, greeter ve kilit deneylerinde sanal işaretçi/klavye |

`labwc` paketi geliştirme listesinden çıkarıldı. Faz 6 paketinde `asena-session`, `wayfire` ve
`xwayland` bağımlılığı alır; Wayfire'ın `wf-shell` önerisi gerekmez (`autostart_wf_shell = false`).

## Yamalı wlroots ve geliştirme araçları (22 Ekim 2026)
Yeni Rust crate'i eklenmedi. Kullanıcı onayıyla Debian 13'ün `libwlroots-0.18` paketi yukarı akış
`40640950` düzeltmesiyle `0.18.2-3+asena1` olarak yeniden derlenir (`packaging/wlroots/`,
`scripts/build-wlroots.sh`; MIT lisanslı, yama yukarı akıştan değiştirilmeden alındı). Derleme için
geliştirme VM'sine `apt-get build-dep wlroots`, `devscripts` ve `quilt` kuruldu; bunlar çalışma zamanı
bağımlılığı değildir. Faz 6'da paket Asena deposundan dağıtılır ve sürümü Debian'ın aynı sürümünü geçer.
