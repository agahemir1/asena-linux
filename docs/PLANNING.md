# Asena — Uygulama öncesi faz kurgusu

## Çalışma durumu
19 Ekim 2026 ek yönlendirmesi: kullanıcı Faz 0'dan sonra diğer fazların da
sırayla uygulanmasını istedi. Genel uygulama onayı her fazda tekrar istenmez;
kabul, güvenlik ve API koşulları devam eder. Compositor değişikliği onayı verilmedi.

18 Ekim 2026 açık kullanıcı onayıyla **Faz 0 uygulaması başladı**. Workspace, ortak çekirdek, boş panel, oturum ve CI kaynakları oluşturuldu. T0 bitmeden altyapı uygulamasına ve yeniden onay beklemeden ilerlemeye bu faz için izin verildi. Faz 1 pencere/uygulama modelleri, görev çubuğu, saat, arka plan ve IPC uygulaması başladı; kaynak oluşturmak kabul kanıtı değildir. ROADMAP durumları uygulama ve doğrulama durumunu gösterir.

Resmî Debian indeksi ve crate manifestleri incelendi; kurulu sistemde `apt policy`, Cargo ve `wayland-info` kanıtından ayrı tutulur. WSL yoktur. Kullanıcının ek onayıyla QEMU 11.1.2 / Debian 13.7 VM'si açıldı ve SSH erişimi doğrulandı. Kullanıcı GitHub Actions yerine şimdilik yerel Linux kontrolleriyle ilerlemeyi seçti; dış CI koşusu doğrulanmış sayılmayacak. Sonuçlar [Faz 0 raporuna](reports/phase-00.md) işlenir.

**Onaylanan tasarım yönü: Odak.** Kullanıcı derinlikli uygulama simgeleri, zarif çizgi simgeleri ve buzlu cam blur hedefini seçti. Ayrıntılar [DESIGN.md](DESIGN.md), uygulama öncesi tasarım hazırlığı [T0-tasarim.md](../prompts/T0-tasarim.md) içindedir. T0, Faz 0–7 numaralarını değiştirmez.

**Ek kullanıcı kararı:** simgeler gerçek nesne hacmi ve yüksek işçilikle hazırlanacak; önceki kutu/çizgi örnekleri kalite onayı değildir. [ICONOGRAPHY.md](ICONOGRAPHY.md) ilk dört simgenin üretim ve inceleme kapısını tanımlar. Menü tipi ve panelin dört kenarı Ayarlar Merkezi'nde değiştirilebilir olacak; bu kişiselleştirme Faz 5'in 1.0 kapsamındadır.

**Arayüz metni ve düzen kararı:** kullanıcı dolgu açıklamalar, em dash, dekoratif renkli rozetler, gerekçesiz üçlü kart düzenleri ve sürekli başlık/alt başlık kalıbı istemiyor. Bu tercih DESIGN'de bütün ekranlar için kabul kuralıdır; gelecekteki taslaklarda uygulanır. Gerçek hata açıklaması ve ayırt edici ikincil veri gerektiği yerde korunur.

## Fazların bağımlılıkları ve çıkış koşulları

| Faz | Giriş | İncelenebilir çıktı | Sonraki faza geçiş |
|---|---|---|---|
| 0 | T0 tasarım hazırlığı, onaylanmış mimari ve Linux geliştirme ortamı | Workspace, bağımlılık/protokol envanteri, boş panel, CI | Derleme ve gerçek labwc panel kanıtı |
| 1 | Faz 0 | Görev çubuğu, saat, arka plan, CLI/D-Bus | Pencere yönetimi, sabitleme ve monitör yaşam döngüsü |
| 2 | Faz 1 | Menü, arama, bildirim sunucusu/merkezi, OSD | Türkçe arama, bildirim ve klavye davranışı |
| 3 | Faz 2 | Paylaşılan servis modelleri, hızlı ayarlar, tepsi, daemon | Donanım/servis hatalarına dayanıklı M1 |
| 4 | Faz 3 ve doğrulanmış session-lock yolu | Kilit, boşta kalma/uyku, greeter, polkit | Kilit/kimlik doğrulama ve uyku güvenliği |
| 5 | Faz 4 | MVP panelleri, ortak arama, ekran geri alma | Tüm MVP panelleri ve M2 |
| 6 | M2 ve dağıtım kararları | .deb, imzalı test deposu, amd64 UEFI ISO, kurulum, welcome | VM'de kurulum/güncelleme ile M3 sürüm adayı |
| 7 | M3 | Ekran görüntüsü, erişilebilirlik, performans ve kapanış raporu | Paketlenmiş çıktıyla bütün 1.0 koşulları / M4 |

Fazlar sırayla uygulanır. Büyük fazların alt grupları prompt'larında sıralıdır; alt grubun bitmesi tüm fazı kapatmaz. Bir sonraki fazın gerektirdiği koşul kanıtlanmadıysa açık iş olarak kalır.

## Sorumluluk sınırları

| Alan | Tek sorumlu / durum kaynağı | Kullananlar |
|---|---|---|
| Donanım ve sistem servis durumu | `asena-services`, ilgili sistem servisinin sinyalleri | Shell, settings, daemon |
| Kullanıcı tercihleri | `org.asenalinux.*` GSettings şemaları | Shell, daemon, settings, welcome |
| Wayfire yapılandırması | Daemon'ın `wayfire` modülü; şablon `data/wayfire/wayfire.ini`, ilk kopyayı session yapar | Shell compositor'a özel komut çalıştırmaz |
| Geçici ekran düzeni ve 15 sn geri alma | Daemon; Wayland işlemleri `asena-wayland` üzerinden | Settings yalnız talep/onay yollar |
| Kilit ve kilit hazır onayı | `asena-lock`, compositor onayı | CLI, menü, daemon uyku akışı |
| Yetkilendirme | Mevcut sistem servisi + polkit; ajan yalnız kimlik doğrular | Settings, shell/daemon işlemleri |
| Oturum açma | greetd; greeter yalnız IPC istemcisi | `asena-greeter`, `asena-session` |
| Bildirim geçmişi | Shell belleği, son 100; transient hariç | Bildirim merkezi, panel |
| Paket kurma ve güncelleme | Debian/APT | ISO, Calamares, paketler |

## Uygulamadan önce çözülmesi gereken kararlar

Bir hedef veya öneri, onaylanmış ürün kararı sayılmaz. Aşağıdaki durumlar planlama sırasında açık tutulur; ilgili kapıda seçenek, öneri ve kullanıcı kararı dosyaya yazılır.

| Kimlik | Karar / doğrulama | En geç | Durum |
|---|---|---|---|
| K01 | Lisans ve üçüncü taraf uyumu | Faz 0 manifest/lisans yazımı | Onaylanan Faz 0 kapsamıyla GPL-3.0-or-later manifest ve LICENSE uygulandı; doğrudan crate lisansları kaydedildi, geçişli/dağıtım denetimi açık |
| K02 | Debian 13 paket sürümleri, Rust MSRV, crate sürüm eşlemesi ve protokol envanteri | Faz 0 | Debian 13.7 kurulu APT/native/Rust ve Wayland globals doğrulandı; Linux kalite ve iç içe panel geçti, fiziksel donanım ayrı |
| K03 | GTK session-lock entegrasyonu; ayrı binding/backport ve gerekirse güvenlik FFI sınırı | Faz 0 araştırma, Faz 4 öncesi karar | Kullanıcı yalnız C 1.3.0 backport'u onayladı. VM'de ayrı prefix derlemesi, 53 upstream test ve panel regresyonu geçti; GTK 4.18/adwaita 1.7 korunuyor; compositor 21 Ekim'de Wayfire oldu. Kilit/PAM güvenlik kabulü açık |
| K04 | Faz 2–3 için geçici güvenli kilitleyici; yoksa kilit/uyku eylemini devre dışı bırakma | Faz 2 | Açık |
| K05 | Gerçek etkin klavye grubunun izlenmesi; tercih değeriyle karıştırmayan kaynak | Faz 3 | Doğrulanacak |
| K06 | Settings portalında renk şeması/vurgu aktarımı; eksikte backend ihtiyacı | Faz 3 | Uygulandı (22 Ekim): gtk backend `accent-color` vermediği için `asena-daemon` kendi Settings arka ucunu sunar; VM'de Flatpak libadwaita uygulaması vurguyu canlı izledi |
| K07 | Logo, duvar kâğıdı ve marka hakları | T0 tasarım, Faz 6 dağıtım | Açık; Odak yönü ve sol hizalama kullanıcı tarafından seçildi |
| K08 | Depo barındırma, alan adı/D-Bus kimliği, anahtar sahibi ve yayın yetkisi | Faz 6B | Açık; kimlik varsayımı `org.asenalinux.*` |
| K09 | Kurulum kapsamı: amd64/UEFI/boş disk önerisi; Secure Boot, dual boot ve şifreleme kararı | Faz 6C öncesi | Öneri; ek destek onaysız dahil edilmez |
| K10 | Pencere yakalama için gerçek geometri kaynağı ve Debian 13 protokol desteği | Faz 0 araştırma, Faz 7A öncesi karar | Açık |
| K11 | Pano geçmişinin sonraya bırakılması; Faz 7'de yalnız görüntü kopyalama | Planlama kapanışı | Mevcut COMPONENTS kapsamı korunuyor |
| K12 | `asena-session` Cargo/POSIX görev ayrımı | Faz 0 | Uygulandı: Cargo binary gömülü POSIX betiğini çağırır; betik Wayfire'ı başlatır (21 Ekim'e kadar labwc), dev başlatıcı süreç grubunu yönetir; iç içe başlangıç ve süreç/ortam temizliği denendi; normal giriş açık |
| K13 | Onaylı gerçek blur hedefi için compositor/protokol/GTK yolu; bakım, performans ve uyumluluk | T0 araştırma, Faz 0 doğrulama; Faz 1 öncesi karar | **Karar verildi (21 Ekim):** kullanıcı onayıyla tek compositor Wayfire 0.9.0; Dual Kawase `blur` eklentisi. VM deneyi ve ölçümler [blur-options.md](research/blur-options.md), geçiş [wayfire-migration.md](reports/wayfire-migration.md). Gerçek GPU ölçümü, üretim opaklık değerlerinin görsel kabulü ve Flatpak `wp-security-context-v1` farkı açık |
| K14 | Asena Odak varsayılan GTK, uygulama/çizgi simgesi ve imleç setleri, lisans ve Flatpak eşlemesi | T0, Faz 0 uyumluluk; Faz 6 paketleme | Görsel yön onaylı, somut paketler açık |
| K15 | Menü tipi ve panel konumunun Ayarlar Merkezi'nden değişimi; ortak yerleşim, boyut sınırları ve önizleme | T0 tasarımı, Faz 1–2 altyapısı, Faz 5 teslimi | Özellik onaylı; son ekran ve ölçüler tasarlanacak |
| K16 | Ekran kimliği, geri alma sahipliği ve kayıt | Faz 5D öncesi | Uygulandı (22 Ekim): kimlik üretici, model ve seri numarası, seri yoksa bağlantı noktası; tek sahip daemon; onaylı düzen ekran takımı başına `displays.json`. Gerçek monitör kabulü açık |
| K17 | Debian wlroots 0.18 paketine yukarı akış düzeltmesi | Faz 6 paketleme | Kullanıcı onayladı (22 Ekim): `packaging/wlroots/` yaması ve `scripts/build-wlroots.sh`; geliştirme VM'sine `0.18.2-3+asena1` kuruldu. Asena deposuna yayın Faz 6B'de |
| K18 | Harf kısayollarının klavye düzenine göre yeri | Faz 3 tercihleri | Uygulandı (22 Ekim): Wayfire fiziksel tuşa baktığı için daemon harf kısayollarını ilk düzende o harfi yazan tuşa çevirir (TR-F'de Super+A → `KEY_F`, Super+N → `KEY_I`). İkinci düzene geçmek kısayolları taşımaz. Gerçek klavyeyle denenmedi |

Kullanıcının Faz 0 onayı altyapının açık tasarım ayrıntılarıyla ilerlemesine izin verir. K03 güvenlik uygulaması Faz 4 öncesinde, K13 üretim panel/menü malzemesi öncesinde ayrıca çözülür. Blur isteği yalnız opak geri dönüş çizilerek tamamlanmış sayılmaz. Paket metadata'sı kurulu Debian raporunun yerini almaz. İlave bağımlılık ve API kararları bakım/lisans kontrolünden geçer. Compositor 21 Ekim kullanıcı kararıyla Wayfire oldu (K13).

## Fazlar arası kapsam ayrımları
- Faz 2 güç menüsü logind ve ortak servis arayüzünü kullanır; shell'de compositor'a özel çıkış komutu bulunmaz. Geçici kilit için `loginctl lock-session` çağrısı tek başına çalışan bir kilitleyici kanıtı değildir.
- Faz 3 Bluetooth eşleşmiş cihazları yönetir; yeni eşleştirme ve Agent1 Faz 5'tedir. Wi-Fi MVP'si açık/kişisel parolalı ağdır; kurumsal ağ otomatik olarak Faz 5'e taşınmaz.
- Faz 3 gece ışığı sabit sıcaklık/aç-kapa sağlar; zamanlama ve otomatik gün batımı ayrı sonraki adaydır.
- Faz 5 uygulama başına bildirim engeli ekler; DND'nin kritik bildirim istisnası bu açık engeli geçersiz kılmaz.
- Faz 3'te Ayarlar ve ekran görüntüsü kısayolları, karşılıkları Faz 5/7'de gelene kadar açıklamalı devre dışı olur.
- Faz 6 sürüm adayıdır. Faz 7'de eklenen screenshot paketi aynı dağıtım hattıyla yeniden ISO'ya alınır.
- Pano geçmişi, ekran kaydı, kendi dosya yöneticimiz/yazılım merkezimiz/kurulum aracımız 1.0 kapsamı dışındadır.

## Her faz için kabul raporu
Her faz için `docs/reports/phase-NN.md` oluşturulur. Faz 0 raporu kaynak kontrolleri ile Linux/CI doğrulamasını ayrı izler.

Rapor: değişen kapsam ve kararlar; kriter başına geçti/kaldı/doğrulanmadı; kullanılan Debian/paket/donanım koşulları; komut/ölçüm/ekran kanıtı; açık engeller ve sonraki adaylar içerir. Donanım yokluğu başarılı test sayılmaz. CI veya manuel deney çalıştırılmadan temiz geçti yazılmaz.

Genel kalite kapısı: build, clippy, fmt, mantık testleri, çeviri ve ilgili manuel deney. Paket/ISO fazları ayrıca kurulum, yükseltme ve güvenli geri alma kanıtı ister. Güvenlik/veri kaybı sorunu açıkken ilgili kilometre taşı tamamlanmaz.

## Sonraki çalışma adımı
Faz 0'ın Linux kalite ve gerçek oturum kabulünü tamamla; ardından onaylanan diğer fazlara giriş koşullarını değerlendirerek sırayla ilerle. K13/K14 ve güvenlik/API kapıları açıkça izlenir; compositor değişikliği kendiliğinden yapılmaz.
