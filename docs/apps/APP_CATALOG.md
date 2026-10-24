# Asena Uygulama Kataloğu

Bu katalog yalnız kodda bulunan ve çalışan işlevleri listeler. Planlanan işler
[APP_ROADMAP.md](APP_ROADMAP.md) içindedir. Ortak çatı: `crates/asena-app`
([APP_FRAMEWORK.md](../architecture/APP_FRAMEWORK.md)).

## Görünür uygulamalar

### Dosyalar (`crates/asena-files`)
- Konumlar: Ev, XDG klasörleri (Masaüstü, Belgeler, İndirilenler, Müzik, Resimler, Videolar), Çöp,
  Bilgisayar ve bağlı sürücüler (`gio::VolumeMonitor`).
- Klasör içeriği `gtk::DirectoryList` ile eşzamansız okunur ve izlenir; klasörler önce, Türkçe
  yerel ayara duyarlı sıralama; ada, tarihe, boyuta göre sıralama.
- Geri/ileri/üst gezinme, klasörde arama (`asena_core::text::fold`), gizli dosyalar (Ctrl+H).
- Açma, birlikte açma, yeniden adlandırma, yeni klasör, konumu kopyalama, çöpe taşıma.
- Çöp görünümünde geri yükleme, kalıcı silme ve çöpü boşaltma (onaylı).
- Dar pencerede kenar çubuğu ayrı sayfaya geçer.

### Ayarlar (`crates/asena-settings`)
- Gruplu kenar çubuğu: Kişiselleştirme, Bağlantılar, Aygıtlar, Sistem; Türkçe katlamalı arama.
- Görünüm (renk şeması, vurgu rengi, metin ölçeği, duvar kâğıdı seçimi), Masaüstü ve panel,
  Bildirimler, Ağ (Wi-Fi listesi ve bağlanma, kablolu durum), Bluetooth, Ekranlar (birincil ekran,
  çözünürlük, yenileme, ölçek, döndürme, konum; 15 sn onaylı geri alma), Ses (çıkış aygıtı, ses düzeyi, sessiz, mikrofon), Klavye (düzen ve yineleme),
  Fare ve dokunmatik yüzey, Güç (pil ve boşta kalma), Kullanıcılar, Bölge ve dil, Tarih ve saat
  (timedated üzerinden otomatik saat), Varsayılan uygulamalar (MIME varsayılanları), Gizlilik
  (son dosyaları ve çöpü temizleme), Erişilebilirlik (metin ölçeği, animasyonlar), Hakkında
  (os-release, çekirdek, işlemci, bellek, disk).
- Klavye/fare ayarlarını `asena-daemon` Wayfire yapılandırmasına yazar; oturum dili bir sonraki girişte
  `environment.d` üzerinden geçerli olur.
- `asena-settings <panel>` ve D-Bus `org.asenalinux.Settings.OpenPanel(s)`.

### Metin Düzenleyici (`crates/asena-text`)
- Sekmeler, GIO ile eşzamansız açma/kaydetme, farklı kaydetme, UTF-8 doğrulama, 32 MB sınırı.
- Kaydedilmemiş değişiklik işareti ve kapatmada onay, belgede bulma, satır/sütun göstergesi, satır kaydırma.

### Notlar (`crates/asena-notes`)
- Notlar `~/.local/share/asena/notes/*.md` dosyalarıdır; başlık ilk satırdan alınır.
- Yazarken otomatik kayıt (atomik yazma), arama, yeni not, çöpe taşıma, klasörü açma.

### Takvim (`crates/asena-calendar`)
- Ay görünümü, etkinlikli günlerin işaretlenmesi, gün listesi.
- Etkinlik ekleme, düzenleme, silme (tüm gün veya saatli, konum).
- Depolama `~/.local/share/asena/calendar/events.ics`; ICS içe ve dışa aktarma (RRULE açılmaz).

### Saat (`crates/asena-clock`)
- Dünya saatleri: IANA saat dilimleri (`zone1970.tab`), yaz saati dahil gerçek saat ve fark.
- Kronometre (turlarla) ve bildirimle biten zamanlayıcı.

### Hesap Makinesi (`crates/asena-calculator`)
- Temel, gelişmiş (trigonometri, logaritma, faktöriyel, üs, mod) ve programcı (bit işlemleri,
  HEX/DEC/OCT/BIN) tuş takımları; klavye girişi; bellek; geçmiş.
- Uzunluk, kütle, sıcaklık, hız ve veri boyutu dönüştürücü.

### Fotoğraflar (`crates/asena-photos`)
- Resimler klasöründeki görsellerin küçük resim ızgarası (işçi iş parçacığında çözülür, EXIF yönü uygulanır).
- Görüntüleyici: önceki/sonraki, arka plan yapma, Dosyalar'da gösterme, çöpe taşıma; klasör seçme.

### Oynatıcı (`crates/asena-player`)
- Müzik ve Videolar klasörlerinden kitaplık, arama, sıradaki/önceki, dosya açma.
- Oynatma `gtk::MediaFile` + GStreamer arka ucu (`libgtk-4-media-gstreamer`).

### Arşiv (`crates/asena-archive`)
- `bsdtar` (libarchive-tools) ile ZIP, TAR, 7z ve diğer biçimleri listeleme.
- Çıkarma öncesi bütün yollar zip-slip denetiminden geçer; yeni bir alt klasöre çıkarılır.

### Ekran Görüntüsü (`crates/asena-screenshot`, kimlik `org.asenalinux.Capture`)
- `org.freedesktop.portal.Screenshot` ile yakalama, gecikme, seçimi portala bırakma.
- `~/Resimler/Ekran Görüntüleri` altına kaydetme, panoya kopyalama, farklı kaydetme.
- Aynı servis `asenactl screenshot` ve Print tuşu tarafından kullanılır.

### Yazılım (`crates/asena-store`, kimlik `org.asenalinux.Store`)
- Keşfet: kaydırılabilir vitrin, gerçek ekran görüntülü uygulama rafları ve kategori kısayolları.
  Kartlar uygulama ayrıntısını, kategori ve “Tümünü gör” düğmeleri tam kategori listesini açar.
  Arama, Kurulu ve Güncellemeler korunur; dar pencerede alt sekme çubuğu kullanılır.
- Katalog: Debian DEP-11 ve etkinse Flathub (AppStream). AppStream bileşeni olmayan paket listelenmez;
  arama `asena_core::text::fold`, listeler yerel ayara göre sıralı.
- Debian paketleri `org.asenalinux.Packages` servisiyle kurulur ve kaldırılır: önce değişiklik özeti,
  sonra polkit parolası. Flatpak uygulamaları kullanıcı kurulumudur; önce istedikleri izinler gösterilir.
- Sistem güncellemeleri indirilir ve yeniden başlatmada kurulur; Flatpak güncellemeleri oturum içinde.
  Yarım kalan dpkg işi için onarım şeridi.
- Ekran görüntüleri Keşfet ve ayrıntı sayfalarında HTTPS ile indirilip önbelleğe alınır.
  Eşzamanlı bağlantılar ve yanıt boyutu sınırlıdır; ağ yoksa gerçek uygulama simgesi kalır.
  Vitrin editoryal seçimdir; indirme, puan veya popülerlik verisi üretilmez.

## Gizli uygulamalar
Uçbirim, Kamera, Belgeler ve Hava Durumu menüde görünmez (`NoDisplay=true`). Elle
başlatılırsa yalnız uygulamanın henüz hazır olmadığını ve varsa alternatifi söyleyen bir pencere açar.

## 23 Ekim 2026 uygulama devamı

- **Sistem İzleyici** (`asena-monitor`, `org.asenalinux.SystemMonitor`): `/proc` üzerinden süreç listesi, CPU/bellek/ağ/disk kullanımı, süreç sonlandırma; Ctrl+Shift+Esc.
- **Yedekleme** (`asena-backup`, `org.asenalinux.Backup`): harici dizine rsync, Linux dosya sisteminde sabit bağlantılı geçmiş, en yeni on yedek, geri yükleme ve günlük kullanıcı zamanlayıcısı.
- **Diskler** (`asena-disks`, `org.asenalinux.Disks`): UDisks2 üzerinden diskler, bölümler, sağlık, bağlama/çıkarma ve onaylı biçimlendirme. Bölüm tablosu düzenleme yok.
- **Uçbirim**: VTE GTK4, gerçek kabuk ve sekmeler.
- **Belgeler**: poppler-utils, PDF sayfaları, yakınlaştırma ve Türkçe katlamalı arama.
- **Kamera**: GStreamer kamera seçimi, JPEG fotoğraf ve WebM video; donanım olmadığında boş durum.
- **Hava Durumu**: Open-Meteo HTTPS tahmin/yer arama, yerel önbellek ve kaynak atfı.
- **Ayarlar**: Uygulamalar (Flatpak/portal izinleri), Yazıcılar (CUPS/cups-pk-helper), Kurtarma.
- **Yazılım**: `.deb` dosya tanıtıcısını yetkili paket servisine verir, planı onaylatır; `.flatpakref` kullanıcı kurulumunu destekler.

Kaynak uygulaması, gerçek donanım veya görsel kabulün geçtiği anlamına gelmez.
Deney sonuçları [apps-recovery raporunda](../reports/apps-recovery.md) tutulur.
