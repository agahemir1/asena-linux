# Faz 1 — Panel ve görev çubuğu

## Bağlam
Önce oku: `CLAUDE.md`, `docs/COMPONENTS.md` §2.1, §2.6, §2.7, `docs/ARCHITECTURE.md` (IPC, Wayland, iş parçacığı modeli), `docs/VISION.md` (tasarım dili), `docs/ROADMAP.md` Faz 0 notları.

Ortak bağımlılık ve kapsam kapıları `docs/PLANNING.md` içindedir. Uygulamaya geçiş ve Faz 0 kabulü olmadan bu görev başlatılmaz.

`docs/DESIGN.md` Odak sözleşmesini uygula: solda başlat, kompakt panel, derinlikli uygulama ve zarif çizgi simgeler. Panelin gerçek blur yolu K13'te kararlaştırılmış olmalı; dar alan taşma davranışı ile opak erişim modu baştan tasarlanır.

Faz 0'da boş bir panel yüzeyi var. Bu fazda panel gerçek bir görev çubuğuna dönüşüyor: pencereleri görüyor ve yönetiyor, saat gösteriyor, duvar kâğıdı çiziyor ve kısayollardan komut alıyor. Sonraki fazlar (menü, hızlı ayarlar, bildirimler) bu panelin düğmelerine ve D-Bus arayüzüne bağlanacak.

## Hedef
İç içe Asena oturumunda açılan pencereler görev çubuğunda görünür; tıklayarak odaklanır ve küçültülür. Sabitlenmiş uygulamalar başlatılabilir, saat doğru gösterilir, duvar kâğıdı çizilir ve `asenactl` shell'e komut iletir.

## Yapılacaklar
1. **`asena-wayland` — foreign-toplevel istemcisi**
   - Ayrı thread ve ayrı `wl_display` bağlantısı; `zwlr_foreign_toplevel_manager_v1`'e bağlanır.
   - Her toplevel için: kimlik, başlık, `app_id`, durumlar (activated, minimized, maximized, fullscreen), çıkışlar.
   - Olaylar `async_channel` ile ana döngüye; istekler (`activate`, `set_minimized`, `unset_minimized`, `close`) aynı yolla geri.
   - Ana thread tarafında `ToplevelModel`: `gio::ListModel` uygulayan, `Toplevel` GObject'leri tutan model.
   - Compositor protokolü desteklemiyorsa model boş kalır, bir kez uyarı loglanır, çökme olmaz.
2. **Uygulama eşleştirme** (`asena-core::apps`)
   - `app_id` → `.desktop` eşlemesi; şu sırayla dene:
     1. Tam desktop ID
     2. `StartupWMClass`
     3. Küçük harf
     4. Ters DNS'in son parçası (`org.gnome.Nautilus` → `nautilus`)
     5. Flatpak dışa aktarım dizinleri
   - Bulunamazsa genel uygulama simgesi ve `app_id` adı.
   - XDG veri dizinlerini izle; yeni kurulan uygulama yeniden başlatmadan görünsün.
   - Birim testleri: örnek `.desktop` fixture'larıyla.
3. **Görev çubuğu widget'ı**
   - Sabitlenmiş uygulamalar (GSettings `pinned-apps`) + çalışan uygulamalar, uygulamaya göre gruplu. Sırası: önce sabitlenmişler, sonra açılma sırasıyla diğerleri.
   - Gösterge: 1 pencere = bir nokta, 2+ pencere = iki nokta; aktif uygulama vurgulu.
   - Tek tık: pencere yoksa başlat; tek pencere varsa aktifse küçült, değilse odakla; birden çok pencere varsa başlıklarla bir popover listesi aç.
   - Orta tık: yeni örnek başlat.
   - Sağ tık menüsü: Görev çubuğuna sabitle/kaldır, Yeni pencere, Tüm pencereleri kapat (birden çok pencere varsa onay iste).
   - Uygulama başlatma: `gdk::Display::app_launch_context()` ile; böylece xdg-activation token iletilir ve yeni pencere odak alır.
   - Tooltip: uygulama adı ve pencere sayısı.
4. **Saat**: yerel ayara uygun saat ve kısa tarih; zamanlayıcı **dakika sınırına hizalanır** (her saniye tetiklenmez). `clock-show-seconds` açıksa saniyelik güncellenir. Tıklayınca şimdilik boş bir popover açılır (Faz 2'de takvim ve bildirimler gelecek).
5. **Başlat düğmesi** ve **durum düğmesi**: şimdilik yer tutucu; tıklayınca ilgili D-Bus metodunu tetikler ve log yazar.
6. **Çoklu monitör**: her monitörde panel. Monitör takıldığında/çıkarıldığında panel eklenir/kaldırılır. Görev çubuğu varsayılan olarak tüm pencereleri gösterir (`panel-on-all-monitors`).
7. **D-Bus `org.asenalinux.Shell`**: `ToggleMenu`, `ToggleQuickSettings`, `ToggleNotificationCenter`, `ShowOsd(s kind, d value)`, `ReloadStyle`. Arayüz tanımı `asena-core::dbus` içinde, hem sunucu hem istemci oradan kullanılır.
8. **`asenactl`**: COMPONENTS.md §2.7'deki komut yapısı (clap). Henüz karşılığı olmayan komutlar anlaşılır bir "henüz desteklenmiyor" mesajı verir. Shell çalışmıyorsa çıkış kodu 1 ve açıklayıcı mesaj.
9. **Duvar kâğıdı**: her monitörde `Layer::Background` yüzeyi. GSettings `org.asenalinux.desktop` → `background-uri` ve `background-mode` (zoom, fit, center, solid). Resim `gio::spawn_blocking` içinde, monitör boyutuna ölçeklenerek yüklenir. Değişince 300 ms çapraz geçiş. Dosya bulunamazsa düz renk.
10. **Stil**: `data/style/shell.css` — panel, görev çubuğu düğmeleri, göstergeler, popover'lar. `asenactl reload-style` ile CSS diskteki dosyadan yeniden yüklenir (yalnız `--dev` modunda).
11. **labwc kısayolları** (`data/labwc/rc.xml`):
    - Super (bırakınca) → `asenactl menu toggle`. `onRelease` desteklenmiyorsa Super+Space.
    - Super+A → hızlı ayarlar; Super+N → bildirim merkezi.
    - Ses ve parlaklık XF86 tuşları → `asenactl …`
    - Print → `asenactl screenshot screen`
12. **labwc teması**: "Asena" ve "Asena-Dark" `themerc`: libadwaita'ya yakın başlık çubuğu renkleri, düğmeler, 1 px kenar.
13. **Yerleşim altyapısı:** varsayılan alt paneli ortak yön/anchor/exclusive-zone modeliyle kur. Faz 5'te alt/üst/sol/sağ, başlangıç/orta/son, kalınlık/simge boyutu ve monitör tercihi canlı uygulanabilsin. Ekran koordinatlarını widget'lara sabitleme; dikey panel içerik düzenini ayrı ele al. Ayarlar Merkezi kontrolleri bu fazda yazılmaz.

## Kapsam dışı
- Başlat menüsünün içi (Faz 2)
- Hızlı ayarlar, ses ve ağ simgeleri (Faz 3)
- Sistem tepsisi (Faz 3)
- Pencere önizlemeleri, sürükle-bırak sıralama, otomatik gizleme

## Kısıtlar
- `asena-wayland` GTK'ya bağımlı olmaz; yalnızca `glib` (kanal köprüsü için) ve `wayland-*` crate'leri.
- Görev çubuğu durumu tek kaynaktan (`ToplevelModel` + GSettings) türetilir; widget kendi kopyasını tutmaz.

## Dikkat
- foreign-toplevel olayları `done` gelene kadar **atomik değildir**: başlık ve app_id değişikliklerini `done`'da uygula.
- `app_id` boş gelebilir (bazı XWayland pencereleri); bu pencereler yine listelenir.
- Pencere kapanınca `closed` olayından sonra handle'ı `destroy` et; aksi halde sızıntı olur.
- Saat: `glib::timeout_add_seconds_local(60)` saati kaydırır. Bir sonraki dakika başına kalan süreyi hesaplayıp tek seferlik zamanlayıcı kur, sonra yeniden hesapla.
- Panel kısayola tepki verirken klavye odağı almamalı (`KeyboardMode::None`). Odak yalnızca popover'lar açıkken gerekir.

## Kabul kriterleri
- [ ] Odak paneli açık/koyu, cam/opak, dar çalışma alanı ve büyük yazıda incelenmiştir; simgeler gerçek küçük ölçekte okunur ve görev çubuğu taşması durum eylemlerini gizlemez.
- [ ] Nautilus, Ptyxis ve Firefox açıldığında görev çubuğunda doğru simgeyle görünür; kapanınca kaybolur.
- [ ] Tek pencereli uygulamada tık: odakla ↔ küçült; çok pencerede pencere listesi.
- [ ] Sabitleme/kaldırma GSettings'e yazılır ve shell yeniden başlayınca korunur.
- [ ] Saat dakika değişiminden en geç 1 sn sonra güncellenir; boştayken shell CPU kullanımı ~%0 (`top` ile 30 sn gözlem).
- [ ] `asenactl menu toggle` shell loglarında görünür; shell kapalıyken çıkış kodu 1.
- [ ] Duvar kâğıdı GSettings'ten değiştirilince geçişle değişir; olmayan dosyada çökme olmaz.
- [ ] `ToplevelModel` ve app eşleştirme için birim testleri var.
- [ ] Shell `kill -9` ile öldürülünce systemd yeniden başlatır ve görev çubuğu açık pencereleri yeniden listeler.

## Doğrulama
- `scripts/dev-session.sh` içinde: 3 farklı uygulama aç, tıklama davranışlarını dene.
- `gsettings set org.asenalinux.desktop background-uri 'file:///…'`
- `asenactl menu toggle; echo $?`
- `journalctl --user -u asena-shell -f`

## Teslim
Önce planını sun ve onay bekle. Bitince: yapılanlar, kabul kriterlerinin her biri için nasıl doğrulandığı, ekran görüntüsü (mümkünse `grim`), doğrulanamayanlar (çoklu monitör vb.), sonraki adaylar. ROADMAP'i güncelle.
