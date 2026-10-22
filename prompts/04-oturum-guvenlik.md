# Faz 4 — Oturum ve güvenlik

## Bağlam
Önce oku: `CLAUDE.md`, `docs/PLANNING.md`, `docs/ARCHITECTURE.md` (oturum, süreçler, protokoller), `docs/COMPONENTS.md` §2.8–2.11, `docs/VISION.md` ve Faz 0–3 raporları.

Faz 3 sonunda shell ve servisler günlük kullanıma yaklaşır. Bu faz güven sınırlarını kurar: oturumu kilitlemek, uykuya güvenli geçmek, oturum açmak ve yetkili işlem istemek ayrı süreçlerin sorumluluğudur.

## Hedef
Görsel/uyarlanabilir sözleşme `docs/DESIGN.md` içindedir. Kilit/greeter arkasında özel oturum içeriğinin görünürlüğü güvenlik önceliğiyle ele alınır; cam efekti bilgi sızdırmak için gerekçe değildir.

Asena oturumu güvenilir biçimde kilitlenir; uyku öncesinde kilit hazırdır. greetd ile giriş ve polkit kimlik doğrulaması masaüstünün tasarım dilini kullanır. Shell çökmesi kilidi veya yetkilendirme ajanını devre dışı bırakmaz.

## Ön koşullar ve karar kapıları
- Compositor protokol raporunda `ext-session-lock-v1` bulunmalı (Wayfire'da `session-lock` eklentisi; 21 Ekim'e kadar labwc). gtk4-layer-shell'in panel desteği, kilit desteği olarak kabul edilmez.
- GTK ile session-lock yüzeyi oluşturma yolu, Debian 13 paketleri, Rust binding'leri, lisans ve mevcut API sınırlarıyla birlikte doğrulanır. Ayrı kütüphane/backport gerekiyorsa önce somut seçenek ve maliyet raporlanır; yeni API sessizce kullanılmaz.
- PAM ve polkit FFI gerekiyorsa yalnız dar bir modülde, gerekçeli lint istisnasıyla kullanılabilir. Güvenlik sınırı ve hata davranışı incelenmeden kendi başına yeni ayrıcalıklı daemon eklenmez.
- Kilit ve greeter deneyleri iç içe oturuma ek olarak kurtarma erişimi bulunan ayrı VM/TTY üzerinde yapılır.

## Yapılacaklar

### A. Kilit ekranı ve PAM (4A)
1. `asena-lock`, `ext-session-lock-v1` üzerinden kilit talep eder; compositor'ın kilitlendi onayı gelmeden başarı bildirmez. Aynı oturumda ikinci çağrı mevcut kilide yönlenir; ikinci kilitleyici kurulmaz.
2. Her monitörde kilit yüzeyi oluştur. Monitör ekleme/çıkarma, ölçek ve döndürme değişimlerini işle. Parola girişi odaktaki yüzeydedir; diğer ekranlar içerik sızdırmadan kaplı kalır.
3. Saat/tarih, avatar/ad, parola, Caps Lock uyarısı ve etkin klavye düzeni göster. TR-Q/TR-F/US girişlerini dene; etkin düzenin kaynağı Faz 3'ün doğrulanmış yolunu kullanır.
4. PAM işlemlerini GTK ana döngüsünden ayır. `/etc/pam.d/asena-lock` ile Debian'ın ortak kimlik doğrulama zincirine bağlan; kullanıcının özel PAM kurallarını değiştirme. Conversation, iptal, yanlış parola ve gecikmeli cevapları işle.
5. Parola loglanmaz, telemetriye veya D-Bus'a taşınmaz; gereksiz kopyası tutulmaz. Geç gelen PAM sonucu yeni denemeyi veya başka kullanıcı durumunu açamaz. Kilidi kaldırma yalnız geçerli kimlik doğrulamasından sonra yapılır.
6. Yanlış parolada çevrilebilir hata ve hareket azaltma tercihine uyan geri bildirim göster. Deneme sınırı/gecikmesi PAM politikasıyla uyumlu olsun; yoğun denemeyle arayüz donmasın.
7. Kilitleyici çöktüğünde compositor'ın kilitli kaldığını doğrula. Sonraki girişte güvenli kurtarma davranışını belgele; shell'i yeniden başlatmayı kilit kaldırma yolu yapma.

### B. Boşta kalma, kapak ve uyku (4B)
8. Daemon'a `org.asenalinux.power` şemasını ve doğrulanan süre modelini ekle: karart → kilitle → ekranı kapat → askıya al. Devre dışı değerleri ve süre sırası açıkça tanımlansın; geçersiz kombinasyonlar reddedilsin.
9. MVP'de `swayidle` tek çocuk süreç olarak yönetilir. Ayarlar değişince eski süreç temizlenir. Karartılan değer etkinlik gelince geri yüklenir; kullanıcının arada değiştirdiği parlaklık ezilmez. Dış araçların desteklediği olaylar Debian sürümünde doğrulanır.
10. logind'in uyku olaylarını ve **geciktirme** inhibitörünü kullan. Kilitleyicinin hazır onayı alınmadan inhibitör bırakılmaz; inhibitörün sonlu süresi hesaba katılır. Hata/zaman aşımında yanlış başarı raporlama; elle başlatılan uykuda işlem durdurulsun, dışarıdan başlatılan uykuda engellemenin garantisi ve sınırlaması ayrıca raporlansın.
11. Kapak davranışında tek sahip belirle: logind ile daemon aynı eylemi iki kez yapmasın. Harici ekran/dock, AC/pil ve logind yetenekleri hesaba katılsın. Dizüstü donanımı olmadan kapak desteğini doğrulanmış sayma.
12. `asenactl lock`, menü, güç menüsü ve kısayollar aynı kilitleme yolunu kullanır. Oturum kapatma mevcut oturumu hedefler; başka kullanıcıların oturumlarına dokunmaz.

### C. greetd ve greeter (4C)
13. `asena-greeter` cage içinde, ayrı greeter hesabıyla çalışır. greetd IPC durum makinesi: kullanıcı seçimi → oturum oluşturma → kimlik doğrulama mesajları → oturum başlatma; iptal ve hata akışlarını da kapsa.
14. AccountsService kullanıcı listesi, elle kullanıcı adı girişi, son kullanıcı tercihi, klavye düzeni ve güç düğmeleri ekle. Sistem hesaplarını listeleme. Son kullanıcı adı dışında kimlik doğrulama verisi saklama.
15. Oturum seçimi yalnız geçerli, okunabilir Wayland session kayıtlarından gelir. `.desktop` `Exec` alanını gelişigüzel kabuk metni gibi çalıştırma. Ortamı greetd oturum başlatma sözleşmesine göre taşı; kullanıcı kimliğini greeter kendisi değiştirmez.
16. Örnek greetd ve cage yapılandırmasını `data/` altında hazırla. Kurulumda mevcut display manager otomatik değiştirilmez; etkinleştirme ve TTY'den geri alma adımları belgelenir. Gerçek sisteme geçişten önce VM'de denenir.

### D. Polkit ajanı ve oturum yaşam döngüsü (4D)
17. `asena-polkit-agent`, libpolkit-agent-1 üzerinden mevcut oturum için kaydolur. İstek açıklaması, desteklenen kimlik seçimi, parola, iptal ve yeniden deneme akışları sağlanır. İstek iptal edilince parola alanı ve ilgili işlem temizlenir; keyfi bir komut alıp çalıştıran arayüz sunulmaz.
18. Geçici ajan varsa iki ajan yarışmasın. Kayıt başarısızlığında durumu logla ve kullanıcıya bildirme yolunu tanımla; yetkilendirme olmadan ayrıcalıklı işlem başarılı görünmesin.
19. user target/servis bağımlılıklarını tamamla. `asena-shell`, daemon ve polkit ajanı ayrı yaşam döngülerine sahip olur; shell yeniden başlaması kilidi etkilemez. Oturum kapanınca servisler, inhibitör FD'leri ve yardımcı süreçler bırakılır.
20. Kilitliyken shell bildirimleri/menüsü görünür veya odaklanabilir olmamalı. Greeter başka kullanıcının oturum bildirimlerine, pano içeriğine veya avatar dışındaki özel verisine erişmez.

## Uygulama sırası
`4A → 4B → 4C → 4D`. 4A'nın güvenlik kriterleri geçmeden otomatik uyku/kilit akışı varsayılan yapılmaz.

## Kapsam dışı
- Parmak izi, yüz tanıma, otomatik giriş, uzaktan giriş ve kilit ekranında medya/bildirim içerikleri.
- Disk şifreleme kimlik doğrulaması; bunun kurulum kapsamı Faz 6'da karara bağlanır.
- greetd veya PAM'ın kendi yerini alan kimlik doğrulama altyapısı.

## Kabul kriterleri
- [ ] Giriş/kilit/polkit ekranları Odak çizgi/malzeme ailesine uyar; büyük yazı ve kısa ekranda parola/onay eylemleri erişilir, efektler özel oturum içeriğini açığa çıkarmaz.
- [ ] `asenactl lock` sonrası tüm monitörler kaplıdır; yalnız doğru parola oturumu açar.
- [ ] Kilitliyken monitör takma/çıkarma ve kilitleyiciyi zorla sonlandırma masaüstünü açığa çıkarmaz.
- [ ] Yanlış parola, PAM iptali, servis hatası ve gecikmeli sonuçlar güvenli kalır; parola günlüklerde görünmez.
- [ ] TR-Q/TR-F/US ve Caps Lock durumu parola girişiyle uyumludur.
- [ ] Elle askıya alma ve logind kaynaklı uyku akışında kilit hazır onayı/inhibitör sırası kaydedilmiştir; uyanınca oturum kilitlidir.
- [ ] Karartma geri alınır; süre değişimlerinde yinelenen swayidle süreçleri oluşmaz.
- [ ] Kapak deneyleri donanım, AC/pil ve harici ekran koşullarıyla raporlanmıştır veya açıkça doğrulanmadı olarak işaretlidir.
- [ ] greetd ile iki normal kullanıcı giriş yapabilir; hatalı giriş, iptal ve oturum seçimi çalışır.
- [ ] Polkit gerektiren gerçek bir işlemde onay, yanlış parola ve iptal denenmiştir; yalnız bir ajan kayıtlıdır.
- [ ] Shell çökmesi kilidi etkilemez; çıkıştan sonra Asena yardımcı süreçleri kalmaz.
- [ ] Durum makineleri için testler ve genel kalite kontrolleri geçer; güvenlik açısından açık engel varken faz tamamlanmaz.

## Doğrulama
- VM anlık görüntüsü ve TTY kurtarma yolu hazırken yanlış parola, süreç çökmesi, monitör değişimi, uyku/uyanma ve ajan iptali senaryolarını uygula.
- İç içe oturum sonuçlarını gerçek giriş/TTY sonuçlarından ayrı raporla. Canlı kullanıcının display manager veya PAM dosyalarını deneme amacıyla değiştirme.
- Genel build, clippy, fmt ve test kontrollerini çalıştır; kullanılan PAM/greetd/polkit sürümlerini ve servis günlüklerini hassas veri içermeden raporla.

## Teslim
Uygulama onayından sonra önce 4A–4D planını sun. Bitince her kriterin kanıtı, güven sınırları, kurtarma adımları, kilit API kararı ve doğrulanamayan donanım koşulları raporlanır. `docs/ARCHITECTURE.md` ile `docs/ROADMAP.md` güncellenir.
