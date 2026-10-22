# Asena Application Framework (`crates/asena-app`)

Asena Desktop ekosistemindeki tüm uygulamaların kullandığı birleşik uygulama çatısının teknik API referansı ve kullanım sözleşmesidir.

---

## 1. Modül Mimarisi

`crates/asena-app` aşağıdaki temel modüllerden oluşur:

```
crates/asena-app/
├── Cargo.toml
└── src/
    ├── lib.rs              # Modül dışa aktarımları ve init
    ├── application.rs      # AsenaApplication yapıcısı ve yaşam döngüsü
    ├── window.rs           # AsenaWindow ve adaptif breakpoint yönetimi
    ├── headerbar.rs        # AsenaHeaderBar yardımcı bileşeni
    ├── states.rs           # EmptyState, LoadingState, ErrorState bileşenleri
    ├── dialogs.rs          # AboutDialog, ErrorDialog, ConfirmDialog yardımcıları
    ├── task.rs             # Arka plan thread ve asenkron işlem koşturucusu
    └── i18n.rs             # Yerelleştirme ve gettext makroları
```

---

## 2. Temel Tipler ve API'ler

### 2.1 `AsenaApplication`
`adw::Application` üzerinde standart yapılandırmayı sağlayan sarmalayıcıdır:

```rust
pub struct AsenaApplication {
    app: adw::Application,
}

impl AsenaApplication {
    pub fn new(app_id: &str, flags: gio::ApplicationFlags) -> Self;
    pub fn setup_default_actions(&self);
    pub fn run(&self) -> glib::ExitCode;
}
```
- **Otomatik İşlemler:**
  - `asena_widgets::init()` çağrısı ile stil ve CSS token'larını yükler.
  - `gettextrs::setlocale` ve `bindtextdomain` ile dil ortamını başlatır.
  - Standart `app.quit` (Ctrl+Q), `app.about`, `win.close` (Ctrl+W) eylemlerini bağlar.

### 2.2 `AsenaWindow` ve `AsenaWindowExt`
Standart pencere kabuğunu, toast overlay'ini ve adaptif kırılma noktalarını sunar:

```rust
pub struct AsenaWindowConfig {
    pub title: &'static str,
    pub icon_name: &'static str,
    pub default_width: i32,
    pub default_height: i32,
    pub min_width: i32,
    pub min_height: i32,
}

pub trait AsenaWindowExt {
    fn show_toast(&self, message: &str);
    fn show_toast_with_action(&self, message: &str, action_name: &str, label: &str);
    fn toast_overlay(&self) -> adw::ToastOverlay;
}
```

### 2.3 `AsenaHeaderBar`
Ortak başlık çubuğu düzenini tek satırda oluşturur:

```rust
pub struct AsenaHeaderBarBuilder;

impl AsenaHeaderBarBuilder {
    pub fn new() -> Self;
    pub fn title(self, title: &str) -> Self;
    pub fn subtitle(self, subtitle: &str) -> Self;
    pub fn show_back_button(self, visible: bool) -> Self;
    pub fn show_search_button(self, active: bool) -> Self;
    pub fn primary_menu(self, menu_model: &gio::MenuModel) -> Self;
    pub fn build(self) -> adw::HeaderBar;
}
```

### 2.4 Durum Bileşenleri (`states`)
Boş, yükleniyor ve hata durumlarını tutarlı görsel dilde sunar:

```rust
pub struct EmptyState;
impl EmptyState {
    pub fn new(icon: &str, title: &str, description: &str) -> adw::StatusPage;
    pub fn with_action(icon: &str, title: &str, description: &str, action_label: &str) -> adw::StatusPage;
}

pub struct LoadingState;
impl LoadingState {
    pub fn new(message: &str) -> gtk::Box;
}

pub struct ErrorState;
impl ErrorState {
    pub fn new(title: &str, error_message: &str) -> gtk::Box;
    pub fn with_retry<F: Fn() + 'static>(title: &str, error_message: &str, retry: F) -> gtk::Box;
}
```

### 2.5 Standart Diyaloglar (`dialogs`)
- `show_about(parent: &gtk::Window, meta: &AppMetadata)`: Tam donanımlı AdwAboutDialog açar.
- `show_error(parent: &gtk::Window, title: &str, message: &str)`: Standart modal hata uyarısı sunar.
- `confirm(parent: &gtk::Window, title: &str, body: &str, destructive: bool, on_confirm: F)`: İki adımlı onay diyalogu.

### 2.6 Asenkron Görev Koşturucu (`task`)
UI thread'ini dondurmadan arka plan işlerini koşturur:

```rust
pub fn spawn_background<T, F, C>(work: F, callback: C)
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
    C: FnOnce(Result<T, String>) + 'static;
```

---

## 3. Yeni Uygulama Şablonu

Her yeni Asena uygulaması bu çatıyı şu şekilde kullanır:

```rust
use asena_app::prelude::*;
use gtk::prelude::*;

fn main() -> glib::ExitCode {
    let app = AsenaApplication::new("org.asenalinux.SampleApp", gio::ApplicationFlags::FLAGS_NONE);
    app.connect_activate(|app| {
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Sample App"))
            .icon_name("org.asenalinux.SampleApp")
            .default_size(800, 500)
            .build();
        window.present();
    });
    app.run()
}
```
