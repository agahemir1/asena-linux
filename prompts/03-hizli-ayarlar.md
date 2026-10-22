# Faz 3 — Hızlı ayarlar ve sistem servisleri

## Bağlam
Önce oku: `CLAUDE.md`, `docs/PLANNING.md`, `docs/ARCHITECTURE.md` (servisler, IPC, asenkron model), `docs/COMPONENTS.md` §2.1, §2.3, §2.7, §2.8, `docs/VISION.md` ve `docs/ROADMAP.md` Faz 0–2 notları.

Faz 2 sonunda panel, menü, bildirim merkezi ve gösterim amaçlı OSD vardır. Bu fazda gerçek donanım ve servis durumu bağlanır. Faz 5'teki Ayarlar da aynı servis modellerini kullanacaktır; iş mantığı widget'ların içine yazılmaz.

## Hedef
Görsel ve responsive kabul `docs/DESIGN.md` Odak sözleşmesini izler; çizgi simgeler, cam/opak yüzeyler ve büyük metin davranışları bu fazda uygulanır.

Asena oturumunda ses, mikrofon, parlaklık, Wi-Fi, Bluetooth bağlantısı, güç profili ve görünüm hızlı ayarlardan yönetilir; durumlar başka uygulamalardan değiştiğinde arayüze yansır. Medya ve sistem tepsisi çalışır, eksik donanım shell'i etkilemez.

## Ön koşullar
- Faz 0–2 kabul raporları mevcut ve gerekli kriterler geçmiş olmalı.
- Debian 13'te kullanılacak servis sürümleri, D-Bus arayüzleri ve crate/API uyumluluğu uygulama başlangıcında doğrulanmalı. Bu prompt bir API'nin varlığının kanıtı değildir.
- Faz 4'e kadar güvenli kilitleme sağlamayan bir düğme kullanıcıya çalışır gibi sunulmaz; geçici kilitleyici kararı `docs/PLANNING.md`'de izlenir.

## Yapılacaklar

### A. Ortak servis modelleri (3A)
1. `asena-services` içinde ses, ağ, Bluetooth, güç, parlaklık, MPRIS ve kullanıcı bilgisi için GObject modelleri oluştur. Her model kullanılabilirlik, mevcut durum ve son işlemin sonucunu sunar. Bağlantı beklenirken, servis yokken ve yetki reddedilince durumlar ayırt edilir.
2. Başlangıç durumunu asenkron al; sonrasında servis sinyallerini dinle. D-Bus adının kaybolması durumunda eski nesneleri ve bekleyen işlemleri temizle; geri geldiğinde yeniden bağlan. Normal çalışma için periyodik yoklama ekleme.
3. Bir işlem sürerken ilgili kontrolün yinelenmesini engelle. Arayüzde başarı durumunu servis onayından önce kalıcılaştırma; hatada gerçek durum geri okunur. Eski bir isteğin cevabı yeni seçimi ezmemeli.
4. Kullanıcı ayarlarının tek kaynağı GSettings, donanım durumunun tek kaynağı ilgili sistem servisi olur. Widget, daemon ve CLI aynı işi ayrı ayrı uygulamaz.

### B. Ses, güç ve parlaklık (3B)
5. **Ses:** PipeWire'ın PulseAudio uyumluluk servisine `libpulse-binding` ve `libpulse-glib-binding` ile bağlan. Varsayılan çıkış/giriş, çıkış cihazı seçimi, ses seviyesi, sessize alma ve mikrofon durumu sun. MVP'de ses üst sınırı %100; yükseltme ayrı bir sonraki adaydır. Sunucu kapanıp geldiğinde model toparlanır.
6. **Parlaklık:** dahili ekran arka ışığı mevcutsa logind'in desteklenen arayüzü ve izin modeli üzerinden uygula. Aralıkları donanımın ham değerlerine dönüştür; sıfıra indirilip ekranın kullanılamaz hale gelmesini engelleyen alt sınırı belgeleyerek belirle. `/sys` izinlerini genişletme, setuid yardımcı ekleme. Harici ekran DDC/CI bu fazda yok.
7. **Pil:** UPower üzerinden yüzde, şarj durumu ve servis sağlıyorsa kalan süreyi göster. Bilinmeyen süreyi sıfır dakika olarak gösterme. Pil yokken pil alanını gizle.
8. **Güç profili:** power-profiles-daemon'ın sunduğu profilleri göster. Performans profili mevcut değilse sunma; etkin profil başka yerden değişince güncelle.
9. `asenactl volume up|down|mute`, `mic mute`, `brightness up|down` komutlarını gerçek servislere bağla. Ortak adımlar ses için 5 yüzde puanı, parlaklık için 10 yüzde puanı olsun. OSD, uygulanan değerle güncellensin. Yetki/donanım yoksa CLI açıklayıcı hata ve sıfır olmayan çıkış kodu versin.

### C. Ağ ve Bluetooth (3C)
10. **NetworkManager:** ağ açık/kapalı, Ethernet durumu, Wi-Fi adaptörü, etkin bağlantı, SSID, sinyal ve güvenlik bilgisi sun. Ağları kararlı kimlikle birleştir; liste yenilenirken klavye seçimi kaybolmasın. Tarama yalnız ayrıntı görünümü açıkken veya kullanıcı yenileyince istensin; servisin sınırlamalarına uyulsun.
11. MVP bağlantıları açık ağ ve kişisel parola kullanan Wi-Fi ile sınırlı olsun. Parolayı yalnız desteklenen NetworkManager arayüzüne ilet; log, bildirim veya GSettings'e yazma. SecretAgent gereksinimi ve sistemdeki mevcut ajanla çakışma uygulama öncesinde doğrulansın; gerekiyorsa yalnız bu akış için ajan ekle. Kurumsal ağ ve sertifikalı kimlik doğrulama Faz 5'te de otomatik kapsam kabul edilmez.
12. **BlueZ:** adaptörü aç/kapat, eşleşmiş cihazları listele, bağlan/bağlantıyı kes. Yeni cihaz tarama ve eşleştirme Faz 5'e aittir. Birden fazla adaptör varsa etkin adaptörü göster; adaptör kaybolduğunda bekleyen işlemi sonlandır.

### D. Hızlı Ayarlar, medya ve tepsi (3D)
13. Panel durum düğmesine bağlı, panel üstünde açılan tek Hızlı Ayarlar yüzeyi yap. Ses/parlaklık kaydırıcıları, Wi-Fi, Bluetooth, koyu mod, gece ışığı, Rahatsız Etme, güç profili ve mikrofon kontrollerini bağla. Yetenek yoksa ilgili öğe gizlenir. Klavye ile gezinme, Esc ve dışarı tıklama ile kapatma çalışır; kapanınca klavye odağı bırakılır.
14. Rahatsız Etme, Faz 2'nin aynı GSettings anahtarını kullanır. Ekran görüntüsü ve Ayarlar kısayolları karşılıkları uygulanana kadar devre dışı ve açıklamalı olur. Güç menüsü Faz 2 ile paylaşılır.
15. **MPRIS:** oturum veri yolundaki oynatıcıları izle. Önce çalan, sonra son etkileşilen oynatıcıyı seç; başlık, sanatçı ve desteklenen oynat/duraklat/ileri/geri eylemlerini göster. Kapak yüklemesi asenkron ve boyut sınırlı olsun; ağdan kapak indirme MVP dışıdır. Kaybolan oynatıcı kartı temizlenir.
16. **Sistem tepsisi:** `org.kde.StatusNotifierWatcher` ve StatusNotifierItem desteği ekle. Simge pixmap boyutlarını doğrula, değişiklik sinyallerini dinle, Activate/SecondaryActivate/ContextMenu ve dbusmenu akışını destekle. Ayrı watcher zaten varsa desteklenen biçimde ona host olarak bağlan; onun adını zorla alma. Tek bozuk öğe tüm tepsiyi durdurmamalı.

### E. `asena-daemon` ve görünüm (3E)
17. Daemon'ı systemd user servisi olarak başlat; shell'den bağımsız çalışsın. `org.asenalinux.desktop` tercihlerini desteklenen `org.gnome.desktop.interface` anahtarlarına yansıt. Sonsuz geri beslemeyi engelle; varsayılanlar yalnız ilk kurulumda uygulansın.
18. Compositor'a özel yapılandırma üretimi yalnız daemon'ın `wayfire` modülünde olsun (21 Ekim'e kadar labwc). `~/.config/asena/wayfire.ini` dosyasını atomik yaz; kullanıcının kendi Wayfire yapılandırmasına dokunma. Yeniden yapılandırma başarısızsa son çalışan dosyayı koru.
19. **Gece ışığı:** MVP'de tek bir `wlsunset` çocuk sürecini yönet. Aç/kapa ve sabit sıcaklık destekle; sıcaklık sınırlarını doğrula. Zamanlama ve konuma göre gün batımı sonraki adaydır. Protokol/çıkış desteklemiyorsa kontrolü gizle; uygulama kapanırken gamma ayarını normale döndür.
20. **Klavye göstergesi:** TR/TR-F/US bilgisinin tercih edilen düzeni mi, gerçek etkin düzeni mi gösterdiğini ayır. Faz 0 protokol envanterine göre gerçek etkin grup için desteklenen kaynak belirle; compositor'a özgü erişim gerekiyorsa daemon'ın labwc modülü üzerinden taşı. Bu kaynak doğrulanmadan tercih değerini etkin durum diye sunma. TR-Q, TR-F ve US geçişlerini OSD'ye bağla.

## Uygulama sırası
`3A → 3B → 3C → 3D → 3E`. Her alt bölüm ayrı incelenebilir değişiklik olarak teslim edilir; hepsi geçmeden Faz 3 ve M1 tamamlanmaz.

## Kapsam dışı
- VPN, hotspot, uçak modu, kurumsal Wi-Fi, Bluetooth eşleştirme arayüzü.
- Boşta kalma, kapak ve uyku öncesi güvenli kilitleme (Faz 4).
- Yerli PipeWire, gamma-control ve idle protokol uygulamaları.
- Ses yükseltme, DDC/CI ve gece ışığı zamanlaması.

## Kısıtlar ve dikkat
- `asena-services` GTK widget'ı import etmez. PulseAudio entegrasyonu da ana döngüyü bloklamaz.
- Parola ve cihaz kimlikleri hata raporlarında gereksiz yere yer almaz. Süreçler shell komut dizgesiyle oluşturulmaz; program ve argümanlar ayrı verilir.
- Oturumun portal ortamı gerçek oturum açılışında sınanır; iç içe oturumdaki başarı, Flatpak desteğinin kanıtı sayılmaz.
- Güç ve ses komutları için `asena-core::dbus` sözleşmesini güncelle; Faz 1 komut yapısını bozma.

## Kabul kriterleri
- [ ] Hızlı Ayarlar Odak tasarımına uyar; dar/kısa alanda kontroller erişilebilir, cam üzerinde metinler okunaklı ve opak uyumluluk modu kullanılabilirdir.
- [ ] Ses ve mikrofon hem arayüzden hem CLI'dan değişir; başka bir ses uygulamasındaki değişiklik 1 sn içinde görünür.
- [ ] Ses servisi, NetworkManager veya BlueZ yeniden başladığında shell çökmez ve model yeniden bağlanır.
- [ ] Parlaklık desteklenen donanımda değişir; desteklenmeyen cihazda öğe gizlenir ve CLI durumu açıklar.
- [ ] Açık ve kişisel parolalı Wi-Fi'ye bağlanılır; yanlış parola sonrası tekrar deneme mümkündür ve parola loglanmaz.
- [ ] Eşleşmiş bir Bluetooth cihazı bağlanır/ayrılır; adaptör çıkarıldığında eski durum gösterilmez.
- [ ] Pil olmayan makinede pil alanı yoktur; sunulmayan güç profili seçilemez.
- [ ] MPRIS oynatıcı kapanınca kart temizlenir; desteklenmeyen eylemler etkin değildir.
- [ ] En az iki farklı StatusNotifierItem ve bir dbusmenu menüsü denenmiştir; bozuk/kapanan öğe tepsiyi etkilemez.
- [ ] Koyu mod ve vurgu rengi shell'e, GTK uygulamasına ve desteklenen portal üzerinden Flatpak uygulamasına yansır; portal eksikleri kaydedilir.
- [ ] Gece ışığında ikinci süreç sızıntısı yoktur; kapatınca ekran normal rengine döner.
- [ ] Etkin klavye düzeni ile gösterge uyuşur; doğrulanamayan kaynak açık bir engel olarak kaydedilir.
- [ ] Hızlı Ayarlar klavyeyle kullanılabilir; donanım olmadan veya servisler kapalıyken shell çalışır.
- [ ] Dönüşümler, servis kaybolma/geri gelme ve eski cevapların elenmesi için anlamlı testler vardır; genel kalite kontrolleri geçer.

## Doğrulama
- Debian 13 gerçek oturumunda ve iç içe oturumda ayrı rapor tut. Servisleri durdurma/yeniden başlatma deneylerini yalnız geliştirme VM'sinde yap.
- `asenactl volume up`, `asenactl mic mute`, `asenactl brightness down` ile gerçek durumu ve OSD'yi karşılaştır.
- MPRIS, tepsi ve Bluetooth için kullanılan uygulama/cihazları ve sürümlerini yaz. Donanım yoksa ilgili maddeyi doğrulanmadı olarak bırak.
- `cargo build`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `cargo test --workspace`.

## Teslim
Uygulamaya geçiş onaylandıktan sonra önce bu fazın alt bölüm planını sun. Bitince: yapılanlar, her kriterin kanıtı, eksik servis/donanım davranışları, portal ve klavye kaynağı kararları, M1 durumu ve sonraki adaylar. `docs/ROADMAP.md` yalnız kanıtlanan ilerlemeyle güncellenir.
