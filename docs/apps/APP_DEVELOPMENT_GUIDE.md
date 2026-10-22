# Asena Yeni Uygulama Geliştirme Rehberi (App Development Guide)

Yeni bir Asena Desktop uygulaması geliştirirken izlenmesi gereken standart mimari, kod yapısı ve kalite kapısı kurallarını içerir.

---

## 1. Yeni Uygulama Başlatma Adımları

### Adım 1: Workspace Crate'i Oluşturma
Her Asena uygulaması `crates/asena-<ad>` klasöründe yer alır. `Cargo.toml` dosyası mutlaka workspace alanlarını devralmalıdır:

```toml
[package]
name = "asena-sample"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
asena-core.workspace = true
asena-app.workspace = true
asena-widgets.workspace = true
adw.workspace = true
gtk.workspace = true
gio.workspace = true
glib.workspace = true
gettext-rs.workspace = true
anyhow.workspace = true
tracing.workspace = true

[lints]
workspace = true
```

### Adım 2: Kalite Kontrol ve POTFILES Kaydı
- `scripts/check-source.py` içindeki `expected` kümesine uygulamanın crate adı eklenir.
- Eklenen tüm `.rs` dosyaları alfabetik sırayla `po/POTFILES` listesine yazılır.
- Kodda kullanılan her kullanıcı metni `gettext("...")` içine alınır.

### Adım 3: Masaüstü Entegrasyonu (.desktop)
`data/applications/org.asenalinux.Sample.desktop` dosyası oluşturulur:
```ini
[Desktop Entry]
Name=Asena Sample
Comment=Sample Application for Asena Desktop
Exec=asena-sample
Icon=org.asenalinux.Sample
Terminal=false
Type=Application
Categories=Utility;GTK;
StartupNotify=true
```

---

## 2. Zorunlu Kodlama Kuralları

1. **Panik Yasak (`unwrap_used = "deny"`):**
   - Kod tabanında `unwrap()` veya `expect()` kullanılamaz.
   - Sonuçlar `match`, `if let` veya `?` operatörüyle yönetilir; hata durumunda arayüzde `AsenaErrorState` gösterilir.
2. **Ana Thread'i Asla Bloklama:**
   - Disk IO, ağ çağrısı, arşiv açma vb. işlemler için `asena_app::task::spawn_background` kullanılır.
3. **Türkçe Desteği:**
   - Arama ve filtrelemelerde doğrudan `.to_lowercase()` kullanılamaz (`İ` -> `i̇` hatası). Daima `asena_core::text::fold()` fonksiyonu çağrılır.
   - Listelemede Türkçe harf sırası gözetilir.
4. **Ortak Bileşenlerin Yeniden Kullanımı:**
   - Özel buton, özel başlık çubuğu veya özel boş sayfa tasarımı yapmak yerine `asena-app` içindeki `AsenaHeaderBar`, `EmptyState`, `AsenaWindow` ve `dialogs` bileşenleri kullanılır.
5. **Arayüz Metni İlkeleri (`docs/DESIGN.md`):**
   - Em dash (`—`) kullanılmaz; kısa doğrudan cümleler kurulur.
   - Başlıkların altına tanıtım sloganları veya ayarın adını tekrarlayan dolgu açıklamalar yazılmaz.
   - Sahte veri veya çalışmayan mockup buton bırakılmaz.

---

## 3. Doğrulama ve Kabul Testi

Uygulamanın tamamlandığı kabul edilmeden önce şu komutlar Debian ortamında hatasız geçmelidir:
```bash
python3 scripts/check-source.py
bash scripts/compile-data.sh
bash scripts/update-po.sh --check
cargo fmt --all -- --check
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```
