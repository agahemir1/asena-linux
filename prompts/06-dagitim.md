# Faz 6 — Paketleme, depo, ISO ve Hoş geldin

## Bağlam
Önce oku: `CLAUDE.md`, `docs/PLANNING.md`, `docs/ARCHITECTURE.md`, `docs/COMPONENTS.md` §2.13, §3–4, `docs/VISION.md` ve Faz 0–5 kabul raporları.

Bu faz çalışan masaüstünü kurulup güncellenebilen dağıtıma çevirir. Önce paketler, sonra test deposu, sonra canlı ortam ve kurulum gelir. Faz 7 henüz bitmediği için bu fazın çıktısı 1.0 değil, sürüm adayıdır.

## Hedef
Varsayılan ürün görünümü `docs/DESIGN.md` içindeki Asena Odak'tır. K14'te seçilen GTK/simge/imleç setleri lisanslarıyla paketlenir; K13'te kararlaştırılan blur desteği temiz kurulumda da doğrulanır.

Debian 13 amd64 üzerinde Asena paketleri kurulup yükseltilebilir; test deposundan doğrulanmış güncelleme alınabilir. Canlı ISO VM'de açılır, Calamares ile boş sanal diske kurulur ve kurulan sistem Asena oturumuna girer.

## Ön koşullar ve karar kapıları
- Faz 0–5 tamamlanmış, M2 raporu kabul edilmiş olmalı. Güvenlik veya veri kaybı riski taşıyan açık engelle ISO yayımlanmaz.
- İlk hedef **amd64 ve UEFI** geliştirme/kurulum denemesidir. BIOS, başka mimariler ve Secure Boot desteği ayrı karar ve doğrulama ister; destekleniyor diye varsayılmaz.
- Lisans, dağıtılacak marka/duvar kâğıdı hakları, depo barındırma, imza anahtarı sahipliği ve güncelleme politikası `docs/PLANNING.md` karar kaydında netleşmeli.
- MVP kurulum senaryosu boş sanal diskte tek sistemdir. Dual boot, disk şifreleme ve mevcut bölümleri koruyan kurulum destek beyanı ayrı test planı gerektirir. Bu sınır kurulum arayüzü ve sürüm notlarında tutarlı olmalı.

## Yapılacaklar

### A. Debian paketleri (6A)
1. `packaging/debian/` altında source package ve binary package ayrımını oluştur. En az shell, ortak veri/şemalar, session, daemon, lock, greeter, polkit ajanı, settings ve welcome bileşenlerinin dosya sahipliği/bağımlılığı tanımlansın. Tek source package'tan birden fazla binary package tercihinin gerekçesini yaz.
2. `asena-desktop` metapaketi gerekli masaüstünü ve seçilen varsayılan uygulamaları toplar. Faz 0 paket envanterinde bulunmayan uygulama varsa belgelenen yedek kullanılır; sessizce harici depo eklenmez. `asena-screenshot` Faz 7'de paket listesine katılacaktır.
3. Şemalar, gettext katalogları, ikonlar, portal dosyası, Wayland session kaydı, PAM ve systemd user unit'lerini Debian yollarına yerleştir. Derleme ağ kullanımı politikasını ve Cargo bağımlılıklarının kilit/vendor stratejisini belgele; temiz paket derlemesi için gerekli tüm kaynaklar sağlansın.
4. Bakım betikleri idempotent olsun. Kurulum kullanıcının GSettings değerlerini veya `~/.config` dosyalarını değiştirmez. Şema derleme ve ikon önbelleği için uygun paket yardımcılarını/tetikleyicileri kullan; sistemde yeni ayrıcalıklı komut yolu açma.
5. greetd/PAM dosyaları için conffile ve mevcut display manager ile birlikte kurulum davranışını belirle. Yeni giriş yöneticisini etkinleştirmek açık yönetici seçimi olur; paket kurulumu mevcut oturumu kesmez.
6. Sürüm numarası, bağımlılık alt sınırları ve en az önceki sürümden yükseltme senaryosunu tanımla. Kullanıcı ayarı şema geçişleri veri koruyarak yapılır. Paket kaldırma ve purge sırasında kullanıcı belgeleri/kişisel dosyalar silinmez.

### B. İmzalı test deposu (6B)
7. APT depo üretimi için bakımı süren bir araç seç; `InRelease`/Release imzası, mimari, dağıtım ve bileşen yapısını belgele. Depo anahtarı ayrı keyring paketiyle taşınır; kaynak kaydı `Signed-By` kullanır.
8. Özel anahtarlar depo, ISO veya CI çıktısına girmez. İmzalama ayrı yetkili adım olur. Anahtar yenileme, iptal ve kayıp senaryosunu, çevrimdışı yedeğin sahibini yaz. `trusted=yes` veya küresel güven ekleme kullanılmaz.
9. Önce yerel/izole test deposu hazırla. Gerçek alan adı, hesap veya sunucuya yükleme ve herkese açık yayın ayrı yayımlama onayıyla yapılır; bu faz için yerel kanıtlar yeterli hazırlık çıktısıdır, dış yayın yapılmadıysa raporda açıkça yazılır.

### C. Canlı ISO (6C)
10. `iso/` altında live-build yapılandırmasını oluştur: Debian 13 tabanı, amd64/UEFI, Asena paketleri, firmware politikası, locale seçenekleri, TR-Q/TR-F/US ve portal/PipeWire servisleri. Secure Boot doğrulanmadıysa gereklilik olarak sunma.
11. Canlı kullanıcı yalnız canlı ortamda bulunur; parola/otomatik giriş gibi canlı oturum tercihleri kurulu sisteme taşınmaz. Kurulum için normal sistemden ayrı, gerekçeli ayrıcalık politikası kullan.
12. Kullanılan kaynak/araç/paket sürümleri, kaynak commit'i ve ISO SHA-256 özetini kaydet. Aynı girdilerle yeniden üretim deneyini yap; bit düzeyinde aynı değilse sebep ve sınırlar raporlanır. Doğrulanmadan tam yeniden üretilebilirlik iddiası yoktur.

### D. Calamares ve kurulu sistem (6D)
13. Calamares için dil, klavye, saat dilimi, boş disk bölümleme (ESP, kurtarma bölümü ve ext4 kök; K24, K30), kullanıcı oluşturma, özet ve kurulum akışını yapılandır. Kurulumun sonunda canlı imaj kurtarma bölümüne kopyalanır ve GRUB'a "Asena Kurtarma" girdisi eklenir. Desteklenmeyen senaryolar kurulumun başarıyla tamamlanacağı izlenimini vermemeli. Disk seçimi ve silme özeti açık olmalı.
14. Kurulum işlemini yalnız geçici VM'nin açıkça tanımlı boş sanal diskinde dene. Hata ve iptal günlüklerini koru; host diski, bağlı kişisel disk veya gerçek kullanıcının sistemini kurulum hedefi yapma.
15. Kurulumdan sonra canlı kullanıcı, geçici parolalar/kurallar, canlı otomatik giriş, kurulum günlüklerindeki sırlar ve canlı ortama özel APT kayıtları temizlenir. UEFI önyükleme ve kurulan kullanıcının greetd → Asena oturumu doğrulanır.
16. Ağa bağlı ve çevrimdışı kurulumun destek durumunu açıkla. MVP ISO gerekli masaüstü paketlerini taşıyarak çevrimdışı temel kurulumu tamamlamalı; depo güncellemesi ağ olduğunda ayrı yapılır.

### E. Hoş geldin ve dağıtım belgeleri (6E)
17. `asena-welcome` kullanıcı başına ilk normal oturumda açılır; greeter hesabında/canlı oturumda otomatik ilk kurulum tamamlandı işareti yazmaz. Atla ve daha sonra aç seçenekleri bulunur.
18. Adımlar: görünüm → panel hizası → isteğe bağlı Flathub → temel kısayollar → bitti. Mevcut GSettings değerlerini kullan; atlanan adımlar kullanıcı ayarını sıfırlamaz. Flathub eklemek açık seçime bağlıdır; ağ hatasında tekrar deneme/atlama mümkündür ve komut shell dizgesiyle çalıştırılmaz.
19. Kurulum, yükseltme, kaldırma, display manager geri alma, desteklenen donanım/önyükleme ve bilinen sorunlar belgelerini yaz. Lisans metinleri, üçüncü taraf bildirimleri ve GPL kapsamında kaynak erişimi yöntemi dağıtım çıktısında bulunur.
20. CI'a temiz Debian paket derlemesi, lintian, izole kur/yükselt/kaldır ve mümkün olduğunda ISO VM smoke adımlarını ekle. Yayın kimlik bilgileri veya otomatik yayın varsayılanı ekleme.

## Uygulama sırası
`6A → 6B → 6C → 6D → 6E`. Paket yükseltmesi geçmeden ISO kurulumuna, VM kurulumu geçmeden sürüm adayına geçilmez.

## Kapsam dışı
- Yeni compositor, kendi kurulum sihirbazımız, yeni paket yöneticisi veya uygulama mağazası.
- Başka mimariler, doğrulanmamış Secure Boot, dual boot ve disk şifreleme desteği.
- Faz 7'den önce 1.0 ilanı ve kullanıcı onayı olmadan dış yayın.

## Kabul kriterleri
- [ ] Kurulu/canlı sistemde Asena Odak varsayılan görünümü, simge/çizgi aileleri, desteklenen blur yolu ve opak erişim modu çalışır; paketlerde geliştiricinin kişisel tema dosyalarına bağımlılık yoktur.
- [ ] Temiz Debian 13'te paket derlenir; build bağımlılıkları, lisanslar ve dosya sahipliği eksiksizdir.
- [ ] Kurulum, önceki sürümden yükseltme, kaldırma ve tekrar kurma denenmiştir; kullanıcı tercihleri korunur.
- [ ] APT test deposunun imzası doğru keyring ile doğrulanır; yanlış anahtar/değiştirilmiş metadata reddedilir.
- [ ] ISO UEFI VM'de açılır; canlı ortamda panel, ağ, ses, bildirim ve settings smoke kontrolleri geçer.
- [ ] Boş sanal diske çevrimdışı kurulum tamamlanır; ISO çıkarıldıktan sonra kurulu sistem açılır.
- [ ] Kurulan normal kullanıcı giriş yapabilir, kilitleyebilir, çıkış yapabilir ve yeniden başlatabilir.
- [ ] Canlı kullanıcı/otomatik giriş ve geçici yetki kuralları kurulu sistemde bulunmaz.
- [ ] İlk açılışta Hoş geldin bir kez açılır; Flathub açık seçim olmadan eklenmez, çevrimdışı akış tamamlanır.
- [ ] SHA-256, paket listesi, kaynak referansı, lisans/kaynak erişimi ve bilinen sorunlar çıktı paketinde vardır.
- [ ] Dış yayın durumu ve desteklenmeyen kurulum senaryoları açıkça yazılmıştır; M3 için yerel kurulum/güncelleme kanıtı mevcuttur.

## Doğrulama
- Temiz Debian build ortamı ve anlık görüntülü UEFI VM kullan; kullanılan disk dosyası/VM kimliğini kayıt altına al.
- En az bir önceki sürüm → yeni sürüm güncellemesini test deposuyla dene; kaldırıp tekrar kurmada kullanıcı ayarlarını karşılaştır.
- Ağ kapalı ISO kurulumu ve ağ açık depo güncellemesini ayrı senaryolarla raporla. Paket/ISO test komutları ve sonuçları dağıtım belgesinde tutulur.
- Genel Rust kalite kontrollerine ek olarak Debian paket kontrollerini çalıştır; beklenen lintian istisnaları gerekçelendirilir.

## Teslim
Uygulama onayından sonra önce paket/depo/ISO planını sun. Bitince .deb ve ISO konumları, sürüm ve özetleri, test raporu, güncelleme/kurtarma yönergeleri, yayınlanma durumu ve açık dağıtım kararları verilir. M3 ve yol haritası yalnız kanıtlanan sonuçla güncellenir.
