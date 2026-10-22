# Faz 2 — Başlat menüsü, bildirimler, OSD

## Bağlam
Önce oku: `CLAUDE.md`, `docs/COMPONENTS.md` §2.2, §2.4, §2.5, `docs/VISION.md` (hareket, performans bütçeleri), `docs/ROADMAP.md` önceki faz notları. `asena-shell`'in mevcut yapısını ve `org.asenalinux.Shell` D-Bus arayüzünü incele.

Ortak kurgu için `docs/PLANNING.md`'yi oku. Bu görev ancak uygulamaya geçiş ve Faz 1 kabulü sonrası başlar. Faz 4 öncesi güvenli geçici kilitleyici K04 kararıyla belirlenir.

Görünüm ve uyarlanabilirlik için `docs/DESIGN.md` bağlayıcıdır. Odak başlat menüsü varsayılan olarak liste ağırlıklıdır; dar/geniş alana uyumlu uygulama satırları ve gerektiğinde ayrı sabitlenmiş görünüm kullan. Cam yüzey, metin kontrastı ve tüm pencere yüksekliğine sığma birlikte tasarlanır.

Menü sunumu ile uygulama/arama modeli ayrı kalır. Faz 5'teki kompakt liste/klasik/ızgara seçimi aynı veriyi ve eylemleri kullanır. Panel kenarı değişince menü, bildirim yüzeyleri ve OSD Faz 1'in ortak yerleşim modeline göre konumlanır; sol-alt koordinatlar sabitlenmez.

Panel var ama başlat düğmesi ve saat popover'ı boş. Bu faz, kullanıcının en sık dokunacağı iki yüzeyi yapıyor: uygulama bulup başlatmak (menü) ve uygulamalardan haber almak (bildirimler).

## Hedef
Super tuşuyla 100 ms altında açılan, Türkçe duyarlı arama yapan bir başlat menüsü. `notify-send` ve gerçek uygulamaların bildirimlerini gösteren, geçmişi tutan bir bildirim sistemi. Ses ve parlaklık için OSD yüzeyi.

## Yapılacaklar

### A. Başlat menüsü
1. **Yüzey**: `Layer::Top`, varsayılan alt panelde sol alta/panel üstüne bağlı, `KeyboardMode::Exclusive` (açıkken). Diğer panel kenarlarında ortak yerleşim modeliyle ekran içine açılır. Esc, dışarı tıklama veya Super ile kapanır. Dışarı tıklamayı yakalamak için menü açıkken tam ekran, saydam ve tıklanabilir bir katman kullan.
2. **Düzen** (yukarıdan aşağı):
   - Arama alanı (açılınca odakta)
   - "Sabitlenmiş" uygulama satırları (`menu-pinned-apps`); isteğe bağlı ızgara varsa sütun sayısı alana uyar, sabit altı sütun zorunluluğu yoktur
   - "Tüm uygulamalar" düğmesi → alfabetik liste ve harf başlıkları; harfe tıklayınca harf ızgarası açılır, oradan atlanır
   - Alt şerit: avatar + ad (AccountsService, yoksa `$USER`), güç düğmesi
3. **Arama motoru** (`asena-shell::search`):
   - `SearchProvider` trait'i: `async fn query(&self, q: &Query) -> Vec<SearchResult>`; sonuçta başlık, isteğe bağlı gerçek ayırt edici ikincil bilgi, simge, puan ve eylem bulunur. Her sonuca dolgu alt başlığı üretilmez.
   - Sağlayıcılar:
     - **Uygulamalar:** Name, GenericName, Keywords; tüm alanlar `text::fold` ile.
     - **Hesap makinesi:** girdi sayı ve operatörden oluşuyorsa; sonucu Enter ile panoya kopyalar.
     - **Komut:** `>` önekiyle başlayan girdi kullanıcı kabuğunda çalıştırılır.
     - **Ayarlar panelleri:** boş sağlayıcı; Faz 5'te dolacak.
   - Puanlama: tam eşleşme > önek > kelime başı > alt dize > bulanık. Kullanım sıklığı katkısı: `~/.local/state/asena/usage.json`, logaritmik ağırlık.
   - Her tuşta önceki sorgu iptal edilir; sonuçlar sırayla gelse de liste titremez (sağlayıcı başına en çok 16 ms bekle, sonra birleştir).
   - Klavye: ↑/↓ seçim, Enter başlat, Tab bölüm değiştir. İlk sonuç her zaman seçili.
4. **Güç menüsü** (logind, zbus):
   - Kilitle: Faz 4'e kadar doğrulanmış geçici kilitleyiciye yönlendir. `loginctl lock-session` ancak bu isteği dinleyen çalışan bir kilitleyici varsa yeterlidir; yoksa eylem açıklamalı biçimde devre dışı olur. Kilit hazır onayı olmadan kilitlendi gösterme.
   - Oturumu kapat: onay diyaloğundan sonra ortak logind/oturum servis arayüzüyle yalnız mevcut oturumu sonlandır. Shell içinde `labwc --exit` çağrısı bulunmaz; gerekiyorsa compositor'a özel sonlandırma session/daemon katmanında kalır.
   - Askıya al
   - Yeniden başlat, Kapat: 60 sn geri sayımlı onay diyaloğu
   - `CanSuspend` gibi yetenekler `no` dönerse ilgili düğme gizlenir.
5. **Animasyon**: VISION.md'deki hareket token'ları; `gtk-enable-animations` kapalıysa animasyon yok.
6. **Performans**: menü bir kez oluşturulur ve gizlenip gösterilir, her açılışta yeniden kurulmaz. Uygulama listesi önceden indekslenir. Açılma süresi `tracing` ile ölçülüp debug seviyesinde loglanır.

### B. Bildirimler
7. **Sunucu** (`org.freedesktop.Notifications`, spec 1.2):
   - `GetCapabilities`: `actions`, `body`, `body-markup`, `icon-static`, `persistence`
   - `Notify`, `CloseNotification`, `GetServerInformation`
   - Sinyaller: `NotificationClosed` (doğru reason kodlarıyla), `ActionInvoked`, `ActivationToken`
   - Hint'ler: `urgency`, `image-data` / `image_data`, `image-path`, `desktop-entry`, `transient`, `resident`, `category`
   - Shell başlarken bus adı başka bir daemon'da ise (ör. mako) uyar ve bildirimleri devre dışı bırak; çökme olmaz.
8. **Açılır bildirimler**: `Layer::Overlay`, varsayılan alt panelde sağ altta/panel üstünde, en çok 3 tane. Diğer panel kenarlarında ortak modelle kullanılabilir çalışma alanına yerleşir. Varsayılan süre 5 sn (uygulama `expire_timeout` verirse o kullanılır); kritikler kalıcı. Fare üzerindeyken süre durur. Tıklama varsayılan eylemi çağırır ve bildirimi kapatır; × ile kapatma.
9. **Bildirim merkezi** (saat popover'ı): solda takvim (`gtk::Calendar`, bugün vurgulu), sağda bildirimler. Bildirimler uygulamaya göre gruplu ve yeniden eskiye sıralı. "Tümünü temizle" ve "Rahatsız etme" anahtarı (`org.asenalinux.notifications`).
   - Rahatsız etme açıkken açılır bildirim gösterilmez ama geçmişe yazılır; kritik bildirimler yine gösterilir.
   - Okunmamış sayısı panelde sade bir sayaç olarak görünür; dekoratif renkli rozet kullanılmaz. Dış uygulamanın bildirim içeriği uygulamanın kendi metin kuralları gerekçesiyle yeniden yazılmaz.
10. **Geçmiş**: bellekte son 100 bildirim; `transient` olanlar geçmişe yazılmaz. Diske yazılmaz (gizlilik).

### C. OSD
11. `Layer::Overlay`, varsayılan düzende alt orta, boş girdi bölgesi (tıklama geçirgen). Ortak model panelin ayırdığı alanı hesaba katar; konum değişiminde OSD panel altında kalmaz. Simge + ilerleme çubuğu (ses, parlaklık) veya simge + metin (mikrofon kapalı, klavye düzeni). 1,5 sn sonra solarak kapanır; yeni değer gelince süre sıfırlanır. D-Bus `ShowOsd` ile tetiklenir.

## Kapsam dışı
- Gerçek ses/parlaklık değişimi (Faz 3); OSD yalnızca gösterir
- Uygulama başına bildirim ayarları (Faz 5)
- Kilit ekranı (Faz 4)
- Dosya araması

## Kısıtlar
- Bildirim gövdesindeki markup, Pango'ya verilmeden önce **beyaz listeyle temizlenir**: yalnız `b`, `i`, `u`, `a href` (yalnızca gösterilir, tıklanınca `gtk::UriLauncher` ile açılır). Diğer etiketler metne dönüştürülür.
- Bildirim içeriğinden asla komut çalıştırılmaz.

## Dikkat
- `image-data` biçimi `(iiibiiay)`: genişlik, yükseklik, rowstride, alpha, bits, channels, veri. **Rowstride'ı dikkate al**, `gdk::MemoryTexture` oluştur.
- `replaces_id` verilen bildirim yeni kart açmaz, mevcut kartı yerinde günceller.
- Menüde `KeyboardMode::Exclusive` kapanırken `None`'a döndürülmeli; aksi halde diğer pencereler klavye alamaz.
- Super tuşu menü açıkken yine `asenactl menu toggle` çağırır; toggle durumu shell'de tutulur.
- Türkçe sıralama: "Ç" harfi "C"den sonra, "Ş" "S"den sonra gelir. Alfabetik liste için `fold` değil, yerel ayara duyarlı karşılaştırma kullan (`glib::utf8_collate`).

## Kabul kriterleri
- [ ] Odak menüsü/bildirim yüzeyleri dar/geniş ve kısa çalışma alanında, açık/koyu ve opak/cam görünümde taşmadan kullanılabilir; uzun Türkçe metin ve klavye odağı korunur.
- [ ] Super → menü açılır; "ayar", "AYAR", "Ayar" aynı sonucu verir; "isik" yazınca adında "Işık" geçen uygulama bulunur.
- [ ] `2+2*3` → 8; Enter ile panoya kopyalanır.
- [ ] Menünün açılma süresi logda < 100 ms (ikinci açılıştan itibaren).
- [ ] Sık açılan uygulama aynı sorguda yukarı çıkar.
- [ ] `notify-send -a Test "Başlık" "<b>kalın</b> metin"` doğru gösterilir; `<script>` gibi etiketler metin olarak görünür.
- [ ] `notify-send -u critical …` kendiliğinden kapanmaz; `-A "evet=Evet"` eylemleri çalışır ve gönderene geri döner.
- [ ] Rahatsız etme açıkken açılır bildirim yok, geçmişte var.
- [ ] `busctl --user call org.asenalinux.Shell /org/asenalinux/Shell org.asenalinux.Shell ShowOsd sd volume 0.4` OSD gösterir ve tıklamaları engellemez.
- [ ] Arama puanlaması, markup temizleyici ve image-data dönüştürücü için birim testleri var.

## Teslim
Önce planını sun ve onay bekle. Bitince: yapılanlar, her kabul kriterinin doğrulaması, ölçülen menü açılma süreleri, sonraki adaylar. ROADMAP'i güncelle.
