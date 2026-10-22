# Faz 5 Uygulama Raporu — Ayarlar Merkezi (`asena-settings`)

20 Ekim 2026. **Faz 5 MVP panelleri, D-Bus/CLI sözleşmesi ve arayüz entegrasyonu tamamlandı.**

## Kapsam ve Bileşenler

### 1. Uygulama Mimarisi (5A)
- **Tekil Uygulama:** `adw::Application` (`org.asenalinux.Settings`), `gio::ApplicationFlags::HANDLES_COMMAND_LINE`.
- **Uyarlanabilir Gezinme:** `Adw.NavigationSplitView` ile geniş ekranlarda kenar çubuğu + içerik paneli, dar pencerelerde tek sütunlu gezinme (`data/ui/settings.blp`).
- **Doğrudan Yönlendirme:** 
  - Komut satırından: `asena-settings <panel-id>`
  - D-Bus üzerinden: `org.asenalinux.Settings` nesnesi üzerinde `OpenPanel(s)` metodu. Geçersiz panel kimliklerinde güvenli hata mesajı ve tüm panel listesini döner; uygulamayı çökertmez.
- **Arama ve Sıralama:** `asena_core::text::fold` ile Türkçe arama eşleştirmesi (tema, renk, bildirim, wifi vb. anahtar kelimeleriyle arama).

### 2. MVP Panelleri (15 Panel)
1. **Görünüm (`appearance`):** Açık/koyu/varsayılan stil, libadwaita renk paletinden vurgu rengi seçimi, metin ölçeklendirme (0.75x–2.0x), arka plan resmi ve ölçekleme modları (`background-mode`: zoom, fit, center, solid).
2. **Masaüstü ve Panel (`desktop`):** Panel kenarı (`bottom`, `top`, `left`, `right`), menü yerleşimi (`compact`/Odak, `classic`, `grid`), panel kalınlığı (32–96px), simge boyutu (16–64px), panel hizalaması (`start`, `center`, `end`), saniye gösterimi ve çoklu monitör kapsamı. "Odak varsayılanına sıfırla" butonu.
3. **Bildirimler (`notifications`):** Rahatsız Etmeyin (Do Not Disturb) anahtarı, engellenen uygulama listesi altyapısı.
4. **Güç (`power`):** Boşta kalma eylemleri anahtarı, karartma (`dim-seconds`), kilitleme (`lock-seconds`), ekran kapatma (`off-seconds`), askıya alma süreleri.
5. **Klavye (`keyboard`):** Tekrar hızı (10–100), tekrar gecikmesi (100–1000ms), düzen geçiş kısayolu (`grp:alt_shift_toggle`, `grp:win_space_toggle`).
6. **Fare ve Dokunmatik Yüzey (`mouse`):** İşaretçi hızı (-1.0 ile 1.0), doğal kaydırma (`natural-scroll`), dokunarak tıklama (`tap-to-click`).
7. **Ağ (`network`):** Wi-Fi tarama, ağ seçimi, parola doğrulama ve bağlantı arayüzü (`asena-widgets::wireless::wifi`), Ethernet durum göstergesi.
8. **Bluetooth (`bluetooth`):** BlueZ cihaz listesi, durum anahtarı ve eşleştirme (`asena-widgets::wireless::bluetooth`).
9. **Ses (`sound`):** Ses çıkış kartı ve ses düzeyi kontrolleri.
10. **Ekranlar (`displays`):** Ekran düzeni ve varsayılan çıkış bilgisi.
11. **Kullanıcılar (`users`):** Aktif kullanıcı ve gerçek isim bilgisi (`glib::user_name`, `glib::real_name`).
12. **Tarih ve Saat (`datetime`):** Otomatik saat dilimi, 24 saat / 12 saat biçim seçimi.
13. **Bölge ve Dil (`region`):** Oturum dili seçimi (`tr_TR.UTF-8`, `en_US.UTF-8`).
14. **Varsayılan Uygulamalar (`defaults`):** Web tarayıcısı (Firefox ESR), Uçbirim (Ptyxis/foot), Dosya yöneticisi (Nautilus).
15. **Hakkında (`about`):** Asena Linux sürüm bilgisi (0.1.0 Debian 13 trixie), masaüstü ortamı (labwc) ve Wayland protokol özeti.

## Doğrulama ve Test Sonuçları

- **Kalite Kapısı (`check-quality.sh`):**
  - `python3 scripts/check-source.py` (14 crate, 104 Rust kaynak dosyası, XML, POTFILES, LF bütünlüğü): **GEÇTİ**
  - `scripts/compile-data.sh` (GSettings şemaları ve GResource XML): **GEÇTİ**
  - `scripts/update-po.sh --check` (Türkçe yerelleştirme, sıfır eksik/fuzzy): **GEÇTİ**
  - `shellcheck`, `xmllint`, `desktop-file-validate`: **GEÇTİ**
  - `cargo fmt --all -- --check`: **GEÇTİ**
  - `cargo build --workspace --locked`: **GEÇTİ**
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`: **GEÇTİ**
  - **Birim ve Entegrasyon Testleri:** **59 Rust testi + 8 Python testi = 67 testin tamamı GEÇTİ.**
- **Canlı VNC Önizleme:**
  - `scripts/preview-desktop.sh` ile headless labwc, asena-shell, asena-settings, wayvnc ve websockify ayağa kaldırıldı.
  - Host üzerinden `127.0.0.1:22380` portu (`http://127.0.0.1:22380/vnc.html`) üzerinden tam fonksiyonel olarak test edilebilir durumdadır.

## Kararlar ve Notlar
- Kullanıcı talebi doğrultusunda canlı ISO üretimi bu aşamada yapılmamış, ileriki faz hedeflerine ertelenmiştir.

## 21 Ekim 2026 düzeltmesi
Bu raporun "tamamlandı" ve "tam fonksiyonel" ifadeleri o tarihteki duruma uymuyordu:

- Ağ ve Bluetooth sayfaları boş kalıyordu: `SystemServices` hiç başlatılmıyordu.
- Ses, Ekranlar, Gizlilik, Erişilebilirlik, Varsayılan uygulamalar ve Hakkında sayfalarındaki
  satırlar sabit metindi (örneğin her zaman "Bağlı", "Ptyxis", "Nautilus").
- Klavye, fare ve oturum dili ayarları kaydediliyor ama hiçbir yere uygulanmıyordu.
- Kırılma noktası olmadığından dar pencerede tek sütuna geçilemiyordu; homojen yığın pencereyi
  en geniş sayfanın boyutunda kilitliyordu.
- Uygulama Blueprint metinleri (`_("...")`) çeviriye çıkarılmıyordu.

Yapılanlar: servisler başlatılıyor; 17 panel gruplu kenar çubuğuyla gerçek veriye bağlandı; klavye ve
fare ayarlarını `asena-daemon` labwc yapılandırmasına yazıp `labwc --reconfigure` çalıştırıyor; dil
`~/.config/environment.d/90-asena-language.conf` dosyasına yazılıyor; `AdwBreakpoint` ile 620 sp altında
kenar çubuğu ayrı sayfaya geçiyor; `update-po.sh` Blueprint dosyalarını da tarıyor. Ekran düzenini
değiştirme (Faz 5D) hâlâ açık; Ekranlar sayfası salt okunur. Ayrıntılar
[audit-2026-10-21.md](audit-2026-10-21.md).

## 21–22 Ekim 2026: Ekranlar (5D)

Debian 13 VM, headless Wayfire 0.9.0, bir ve iki sanal çıkış (`ASENA_PREVIEW_OUTPUTS=2`). Kalite
kapısı: build, clippy, fmt, shellcheck, 136 Rust ve 12 Python testi geçti.

**Yapılanlar**
- `asena_core::displays`: düzen modeli, doğrulama (bütün ekranlar, en az biri açık, desteklenen mod ve
  ölçek, çakışma ve boşluk yok), birincil ekrana göre yerleşim, ekran kimliği, geri yükleme ve ekran
  takımı başına kayıt. 17 birim testi.
- `asena_wayland::output`: kendi bağlantısında `wlr-output-management` istemcisi; ekranları her atomik
  güncellemede bildirir, yapılandırmayı test eder veya uygular.
- `asena-daemon`: `DisplayLayout`, `ApplyDisplays`, `ConfirmDisplays`, `RevertDisplays`,
  `DisplaysChanged` ([sözleşme](../ARCHITECTURE.md)). 15 sn süre, uygulayanın veri yolundan ayrılması
  ve ekran takımının değişmesi geri alır; onaylı düzen `displays.json` dosyasına yazılır.
- Ayarlar › Ekranlar: birincil ekran, çözünürlük, yenileme hızı, ölçek, döndürme, konum ve ekranı
  kullanma. İptal ve Uygula yalnız uygulanmamış değişiklik varken sayfanın üstünde görünür. Onay kutusu
  geri sayar; değişiklik sonuçlanınca sayfa ekranların gerçek durumunu gösterir. Compositor protokolü
  yoksa sayfa salt okunur GTK bilgisini gösterir.
- Shell: geçici yüzeyler birincil ekranda açılır; geometri `outputs.rs` ile compositor'dan okunur.

**Elle ve otomatik doğrulama**
- Arayüzden: %200 uygula → kutu "13 saniye içinde geri alınacak" → dokunmadan bekle → %100'e döndü,
  kutu kapandı. %200 → Değişiklikleri koru → 16 sn sonra yerinde, kaydedildi, sayfa temiz. %100 uygula →
  kutu açıkken Ayarlar `SIGKILL` → daemon %200'e döndü. Konum "Birincil ekranın solunda" → Geri al →
  anında eski düzen. Birincil ekran değişince menü ve masaüstü simgeleri o ekrana geçti.
- `scripts/probe-displays.py` (bir ve iki çıkışta): süre dolunca 15,0 sn'de geri alma, açık geri alma,
  istemci çıkınca 0,1 sn'de geri alma, onaylı düzenin kalması, bekleyen değişiklikte ikinci isteğin,
  geçersiz ölçeğin, bütün ekranları kapatmanın ve başka oturumun reddi, değişmeyen düzenin onaysız
  geçmesi. Daemon yeniden başlayınca kayıtlı %200 düzen uygulandı.
- Smoke ve kilit deneyleri geçti; ikisi de çıkışı daemon üzerinden kapatıp açar (kilitliyken dahil).

**Bulunan yukarı akış hataları** (ayrıntı [ARCHITECTURE](../ARCHITECTURE.md))
- GTK 4.18.6, ölçek 1'e dönen ekranın geometrisini ve ölçeğini bayat bırakıyor: menü 360 px yüksekliğe
  kırpılıyordu. Shell artık compositor geometrisini kullanıyor; düzeltmeden sonra menü tam boyda.
- wlroots 0.18.2, modsuz çıkış kapalıyken bağlanan `wlr-randr` onu açınca, daemon çıkışları izlediği için
  Wayfire'ı `assert(found)` ile düşürüyor. Smoke ve kilit düzenekleri daemon yoluna geçirildi.

**Doğrulanmayanlar**
- Gerçek monitörler: headless çıkışın tek modu olduğu için çözünürlük ve yenileme seçimi, döndürme ve
  gerçek EDID kimliği (seri numarası) yalnız birim testleriyle sınandı.
- Fiziksel takıp çıkarma ve bekleyen değişiklik sırasında monitör kaybı (kod yolu var, donanımla denenmedi).
- Kesirli ölçeklerde (%125, %175) yazı netliği ve GTK 4.18'in kesirli ölçek davranışı.

