# Faz 0 — Geliştirme ortamı ve proje iskeleti

## Bağlam
Önce oku: `CLAUDE.md`, `docs/VISION.md`, `docs/ARCHITECTURE.md`, `docs/COMPONENTS.md`.

Bu dosya planlama aşamasında görev tarifidir; uygulamaya geçiş istenmeden çalıştırılmaz. Önce `docs/PLANNING.md` içindeki K01–K03 karar kapılarını ve Linux ortamını netleştir.

T0 tasarım hazırlığını ve `docs/DESIGN.md` sözleşmesini oku. K13 için gerçek blur/protokol/GTK uyumluluğunu araştır; K14 tema/simge eşlemesini doğrula. Uygulama araştırması compositor değişikliğini kendiliğinden onaylamaz.

Asena'ya sıfırdan başlıyoruz. Bu fazda kullanıcıya görünen hiçbir özellik yazılmaz. Amaç, sonraki yedi fazın üzerine kurulacağı zemini doğru kurmak: varsayımları gerçek Debian 13 ortamında doğrulamak, workspace'i kurmak ve iç içe çalışan bir geliştirme oturumu hazırlamak.

## Hedef
Temiz bir Debian 13 ortamında tek komutla kurulan, derlenen, CI'dan geçen ve `scripts/dev-session.sh` ile iç içe labwc içinde altta boş bir Asena paneli gösteren bir workspace.

## Yapılacaklar
1. **Ortamı doğrula** ve sonuçları `docs/ARCHITECTURE.md`'deki tablolara işle:
   - `apt policy` ile Debian 13 sürümleri: labwc, libgtk-4-dev, libadwaita-1-dev, libgtk4-layer-shell-dev, rustc, cargo, greetd, cage, xdg-desktop-portal-wlr, xdg-desktop-portal-gtk, blueprint-compiler, fonts-inter, fonts-jetbrains-mono, wlsunset, swayidle, grim, slurp.
   - `docs/COMPONENTS.md` §4'teki varsayılan uygulamaların Debian 13 paket adları; olmayanları işaretle.
   - İç içe labwc içinde `wayland-info` ile desteklenen protokolleri listele; ARCHITECTURE.md'deki protokol tablosunu düzelt.
   - gtk4-layer-shell 1.0.4'ün ext-session-lock desteği var mı? Sonucu Faz 4 için not et.
2. **`scripts/setup-dev.sh`**: Debian 13 geliştirme bağımlılıklarını kuran idempotent betik. Rust'ı Debian paketinden kurar; rustup varsa ona dokunmaz.
3. **Cargo workspace** (ARCHITECTURE.md'deki yapı):
   - Tüm crate'ler oluşturulur; şimdilik boş ama derlenebilir.
   - `[workspace.package]`: `rust-version` = Debian 13 rustc, `license = "GPL-3.0-or-later"`.
   - `[workspace.dependencies]`: gtk4 (`v4_18`), libadwaita (`v1_7`), gtk4-layer-shell, zbus, tracing, tracing-journald, anyhow, thiserror, gettext-rs, async-channel.
   - `[workspace.lints]`: `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"` (testlerde `#[allow]`), `unsafe_code = "forbid"` (FFI gereken crate'lerde gerekçeli istisna).
4. **`asena-core`**:
   - `logging::init(app_name)`: journald varsa journald, yoksa stderr.
   - `i18n::init()`: gettext text domain `asena`.
   - `settings`: GSettings şeması yoksa anlaşılır hata veren yardımcılar.
   - `text::fold(&str) -> String`: arama için normalize etme.
     - `İ`, `I`, `ı`, `i` → `i`
     - `ç`→`c`, `ğ`→`g`, `ö`→`o`, `ş`→`s`, `ü`→`u` (büyük harfleri dahil)
     - Ardından küçük harfe çevir ve diğer birleşik aksanları at.
     - Kapsamlı birim testleri yaz.
5. **`asena-shell`** iskeleti: `adw::Application` (`org.asenalinux.Shell`); gtk4-layer-shell ile her monitörün altına sabitlenen, 48 px yüksekliğinde, exclusive zone'lu boş bir panel yüzeyi. İçinde sadece "Asena" etiketi olsun. Stil `data/style/shell.css`'ten gresource ile yüklenir.
6. **`asena-session`**: ortam değişkenlerini ayarlar, labwc'yi `-C <config-dir>` ile `exec` eder. `--dev` bayrağıyla depodaki `data/labwc` ve `target/` kullanılır.
7. **`data/`**:
   - `labwc/`: minimal `rc.xml` (Super+Enter → terminal), `autostart`, `environment`, taslak "Asena" `themerc`.
   - `sessions/asena.desktop`
   - `systemd/`: `asena-session.target`, `asena-shell.service`
   - `schemas/org.asenalinux.shell.gschema.xml`: şimdilik `pinned-apps`
   - `portals/asena-portals.conf`
8. **`scripts/dev-session.sh`**: şemaları `target/schemas`'a derler, workspace'i derler, mevcut Wayland oturumu (veya WSLg) içinde iç içe labwc açar ve `asena-shell`'i başlatır. Kapatınca süreç bırakmaz.
9. **Çeviri altyapısı**: `po/POTFILES`, `po/LINGUAS`, `scripts/update-po.sh` (xgettext ile Rust kaynaklarından `.pot` üretir), boş `po/tr.po`.
10. **CI**: GitHub Actions, `debian:trixie` konteyneri: bağımlılıklar → fmt check → clippy → test.
11. **`README.md`**: projenin ne olduğu, geliştirme ortamı kurulumu, `dev-session.sh` kullanımı. Kısa tut.
12. **`.gitignore`**, **`rustfmt.toml`** (varsayılanlar yeterliyse oluşturma).

## Kapsam dışı
- Panel içeriği, görev çubuğu, saat (Faz 1)
- Paketleme, ISO (Faz 6)
- labwc temasının son hali (Faz 1'de olgunlaşır)

## Kısıtlar
- `asena-session` ve betikler POSIX/Bash. Debian'a özgü yollar dışında dağıtım varsayımı yapma.
- Geliştirme oturumu kullanıcının gerçek `~/.config/labwc`'sine **dokunmaz**.

## Dikkat
- `"İ".to_lowercase()` Rust'ta `"i̇"` (i + U+0307) verir. `fold` içinde `İ`/`I`/`ı` küçük harfe çevirmeden **önce** işlenmeli.
- gtk4-layer-shell, pencere `realize` edilmeden **önce** `init_layer_shell()` ister.
- Monitör listesi `gdk::Display::monitors()` ile alınır (bir `ListModel`). Panel her monitör için ayrı pencere olarak açılır; `items-changed` dinlenir.
- WSLg'de iç içe labwc GPU sorununda `WLR_RENDERER=pixman` ile açılır; bunu betiğe ipucu olarak ekle.

## Kabul kriterleri
- [ ] Temiz bir Debian 13'te `scripts/setup-dev.sh && cargo build --workspace` hatasız tamamlanır.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` ve `cargo test --workspace` temiz.
- [ ] `scripts/dev-session.sh` iç içe labwc açar; altta 48 px panel görünür ve açılan pencereler panelin altına girmez.
- [ ] `fold` testleri: `fold("İSTANBUL") == "istanbul"`, `fold("Işık") == "isik"`, `fold("ÇAĞRI") == "cagri"`, `fold("Ayarlar") == fold("AYARLAR")`.
- [ ] ARCHITECTURE.md'deki sürüm ve protokol tabloları gerçek değerlerle güncellenmiş.
- [ ] CI yeşil.

## Teslim
Önce planını sun ve onay bekle. Bitince şunları raporla:
1. Doğrulanan paket sürümleri ve belgelerden sapmalar
2. labwc'nin desteklemediği ama listede olan protokoller (varsa)
3. Faz 4 için ext-session-lock durumu
4. Debian 13'te bulunmayan uygulama paketleri
5. Açık sorular

`docs/ROADMAP.md`'de Faz 0'ı yalnız tüm kabul kriterleri kanıtlandıysa ✅ yap ve faz notunu ekle. Doğrulanamayan kriter varsa 🟨 bırak; eksikleri kabul raporuna yaz.
