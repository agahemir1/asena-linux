# Yerel bağlayıcı düzeltmesi

Upstream polkit-agent-rs 0.3.0, MIT. Kaynak crates.io arşividir;
lisans üst depodan ayrıca eklendi. GLib 0.20 ve Debian native polkit API'si korunur.

Yalnız `src/subclass/listener.rs` değiştirildi:

- `g_task_new` tarafından verilen tam sahiplik `from_glib_full` ile alınır.
  Ek referans bırakılmaz.
- Finish, başlangıçta içeriği tanımlanmamış veya null olabilen error çıkışını
  okumaz. Rust sonucunun hatası native GError çıkışına yazılır; null çıkış kabul edilir.
- Finish trait'i `Result<bool, glib::Error>` döndürür. Tip dönüşümü başarısızsa
  panic yerine hata üretilir. Native API veya ABI değişmez.

Bu kaynak upstream'in hatasız olduğu varsayımına dayanmaz. Native kayıt/PAM,
iptal ve görev callback yaşam döngüsü VM'de ayrıca sınanmalıdır. Asena workspace
crate'lerindeki `unsafe_code=forbid` korunur; FFI yalnız bu bağımlılık sınırındadır.
