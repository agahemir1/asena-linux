# Faz 5 — Ayarlar Merkezi

## Bağlam
Önce oku: `CLAUDE.md`, `docs/PLANNING.md`, `docs/COMPONENTS.md` §2.12, `docs/ARCHITECTURE.md` (GSettings, servis modelleri, output-management), `docs/VISION.md` ve Faz 0–4 raporları.

Shell hızlı işlemleri sağlar; bu faz ayrıntılı tercihleri tek uygulamada toplar. Faz 3'ün servis modelleri ve Faz 4'ün polkit ajanı yeniden kullanılır. Ayarlar ikinci bir donanım yönetim katmanı oluşturmaz.

## Hedef
`docs/DESIGN.md` ortak görünüm sözleşmesidir: Odak, derinlikli uygulama simgeleri, zarif çizgi kontrolleri, opak içerik ve dar alanda tek sayfalı gezinme.

`asena-settings`, tanımlı MVP panellerini arama, doğrudan panel açma, klavye erişimi ve güvenli hata/geri alma davranışlarıyla sunar. Shell ve uygulama aynı tercihleri anında görür.

## Ön koşullar
- Faz 4'te kimlik doğrulama ve oturum yaşam döngüsü kabul edilmiş olmalı.
- Her panelin servis/yetenek bağımlılığı tanımlanmalı. Servis eksikliği uygulamanın tamamını kullanılamaz yapmaz.
- Ekran yapılandırmasında çıktı kararlı kimlikleri ve geri alma sahipliği uygulamadan önce karara bağlanır; daemon işlemin tek sahibidir.

## Yapılacaklar

### A. Uygulama ve panel sözleşmesi (5A)
1. Tek örnek `adw::Application` (`org.asenalinux.Settings`), uyarlanabilir gezinme ve Blueprint/CompositeTemplate tabanlı paneller oluştur. Debian'ın libadwaita 1.7 sınırındaki bileşenleri kullan.
   - DESIGN'in arayüz metni ve düzen kurallarını uygula. Sayfa başlığı ve gerekli ayar grupları yeterlidir; başlık altına tanıtım cümlesi veya her satıra yeniden açıklama ekleme. Satır/seçim öncelikli düzen kullan; dekoratif rozet ve üçlü özet kartları oluşturma. Yardım yalnız gerçek karar veya hata için gerekli olduğunda gösterilir.
2. Her panelin kararlı kimliği, çevrilebilir adı, anahtar kelimeleri ve uygunluk durumu olur. `asena-settings <panel-id>` ve D-Bus `OpenPanel` aynı yönlendirmeyi kullanır; bilinmeyen kimlik anlaşılır hata verir.
3. Faz 2'nin boş ayar arama sağlayıcısına aynı panel kayıtlarını bağla. Arama için `text::fold`, görüntülenen alfabetik sıralama için Türkçe yerel ayarına duyarlı karşılaştırma kullan.
4. GSettings değişikliklerini anında uygula; ayrıcalıklı/donanımsal işlemde bekleniyor, başarılı, reddedildi ve servis yok durumlarını göster. İptalde kullanıcı tercihini uygulanmış gibi kaydetme.

### B. Masaüstü tercihleri (5B)
5. **Görünüm:** açık/koyu/sistem, libadwaita paletinden vurgu, duvar kâğıdı ve zoom/fit/center/solid, metin ölçeği. Dosya seçimini portal uyumlu yap; okunamayan dosyada önceki ayarı koru. Daemon'ın Faz 3 yansıtma yolunu kullan.
6. **Masaüstü kişiselleştirme:** menü tipi (Odak kompakt liste, klasik iki bölümlü, ızgara), panel konumu (alt/üst/sol/sağ), başlangıç/orta/son hizası, panel kalınlığı ve uyumlu simge boyutu, sabitlenmiş uygulama sırası, saat biçimi/saniyeler ve tüm/seçili monitör kapsamı. DESIGN'deki yerleşim sözleşmesini uygula. Küçük canlı önizleme sun; shell'e yeniden giriş gerekmeden uygula. Aynı uygulama arama/başlatma modeli tüm menü tiplerinde paylaşılır. Otomatik gizleme bu grubun ön koşulu veya otomatik kapsamı değildir.
   - Panel kenarı değişince exclusive zone ve bütün açılır yüzeylerin bağlandığı kenar güncellenir; eski kenarda boşluk kalmaz. Dikey panel ayrı yerleşimdir, içerik döndürülmez.
   - Yanlış boyut/kenar kombinasyonunu reddet; seçilen ekran çıkarıldığında erişilebilir bir panele dön. İşlem hatasında son çalışan düzen korunsun. Yalnız menü/panel düzenini Odak varsayılanına döndüren eylem bulunsun.
   - Görünüm sayfasında cam/azaltılmış saydamlık ve desteklenen blur tercihlerini K13 yoluyla bağla; desteklenmeyen efekti çalışıyor gibi sunma.
7. **Bildirimler:** Rahatsız Etme ve uygulama başına izin; Faz 2 sunucusunu bu tercihlere bağla. Bilinen uygulamaları desktop kayıtları ve bu oturumda bildirim gönderenler üzerinden listele. Engellenen uygulamadan yeni bildirim popup/geçmişe eklenmez; mevcut geçmişi silme ayrı kullanıcı eylemidir. DND'nin kritik bildirim istisnası açık uygulama engelini aşmaz.
8. **Güç:** boşta kalma süreleri, pil/AC ve kapak tercihleri, mevcut güç profilleri. Süre sırasını doğrula; güvenli kilitlemenin mümkün olmadığı bir durumu gizleyerek kaydetme.
9. **Klavye:** TR-Q, TR-F, US dahil düzen listesi, sıra, geçiş kısayolu ve tekrar hızı. **Fare/dokunmatik yüzey:** hız, doğal kaydırma, dokunarak tıklama. Donanıma uygun alanları sun; daemon'ın `wayfire` modülü `wayfire.ini` dosyasına yazar, Wayfire kendisi yeniden okur. XKB seçenekleri ve Wayfire sürüm desteği doğrulanır; başarısız dosya son çalışanı bozmaz.

### C. Bağlantılar ve ses (5C)
10. **Ağ:** Wi-Fi listesi, mevcut bağlantı ayrıntıları, bağlan/ayır, kaydedilmiş ağı unutma ve Ethernet durumu. Faz 3'ün parola akışını kullan. VPN, hotspot, kurumsal Wi-Fi, sertifika yönetimi ve gelişmiş IP editörü kapsam dışıdır.
11. **Bluetooth:** tarama, yeni cihaz eşleştirme, bağlan/ayır ve kaldırma. BlueZ Agent1 gerektiren doğrulama/onay akışlarını destekle; cihazın gösterdiği kodla kullanıcı karşılaştırır. Zaman aşımı/iptalde tarama ve diyalog temizlenir. Güven kararı kullanıcı eylemi olmadan verilmez.
12. **Ses:** giriş/çıkış seçimi, ses/sessizlik, uygulama başına oynatma düzeyi. Kapanan akış satırını kaldır; mikrofon seviyesinin gösterimi için yalnız panel açıkken gerekli kaynakları kullan. Hoparlör/mikrofon testi eklenirse kayıt veya dosya bırakmadan açık kullanıcı eylemiyle başlatılır.

### D. Ekranlar ve geri alma (5D)
13. `wlr-output-management` üzerinden çözünürlük, yenileme, ölçek, döndürme, yerleşim ve birincil ekran tercihini sun. Mantıksal koordinatlarla fiziksel pikselleri ayır; geçersiz yerleşim ve bütün ekranları kapatma reddedilsin.
14. Daemon test/uygulama/onay/geri alma işleminin tek sahibidir. Desteklenen protokolde önce yapılandırmayı test et; uygulandıktan sonra kullanıcıya **15 saniye** onay ver. Kullanıcı onaylamazsa, uygulama çökerse veya bağlantı koparsa son çalışan düzen geri gelsin.
15. İşlem sürerken ikinci bir işlem başlatma. Monitör takma/çıkarma halinde topolojiyi yeniden değerlendir; artık var olmayan ekran kimliğiyle eski düzeni körlemesine uygulama. Başarılı ve onaylı düzeni atomik kaydet; sonraki oturumda yalnız eşleşen donanıma uygula.
16. Compositor protokolü yoksa ekran paneli açıklayıcı salt okunur/uygunsuz durum gösterir; compositor dosyalarına shell veya settings içinden doğrudan erişim eklenmez.

### E. Sistem, kullanıcı ve varsayılanlar (5E)
17. **Kullanıcılar:** AccountsService ad ve avatar işlemleri; parola değişimi için Debian'da desteklenen kimlik doğrulama yolunu doğrula. Parolayı loglama/saklama; yönetici işlemleri polkit yetkisine bağlıdır. Kullanıcı ekleme/silme bu MVP'ye dahil değildir.
18. **Tarih/saat:** timedated ile NTP ve saat dilimi, kullanıcı tercihi olarak 24 saat biçimi. **Bölge/dil:** sistem için localed, kullanıcı için desteklenen locale tercihleri; ikisinin etkisini ayır. Eksik dil paketi ve sonraki oturumda uygulanacak değişiklik açıkça belirtilir. Metin dosyalarını ayrıcalık yükselten betikle düzenleme.
19. **Varsayılan uygulamalar:** web, e-posta, müzik, video, fotoğraf, dosya ve terminal tercihleri. Standart MIME/URI eşlemelerini kullan; terminal için doğrulanan standart veya açıkça belgelenmiş Asena tercihi seç. Mevcut olmayan bir sistem standardı varsayma.
20. **Hakkında:** `/etc/os-release`, desteklenen donanım özeti ve hostnamed; bilgisayar adı değişimi izin kontrollüdür. Makine kimliği, seri numarası gibi gereksiz kişisel tanımlayıcıları gösterme.

## Uygulama sırası
`5A → 5B → 5C → 5D → 5E`. Her grup ayrı teslim edilebilir; M2 için tüm MVP panelleri ve Faz 4 tamamlanmalıdır.

## Kapsam dışı
- Yazıcılar, paylaşım, Flatpak izin editörü, genel kısayol editörü ve ayrı erişilebilirlik ayar paneli.
- Ağın gelişmiş kurumsal özellikleri, kullanıcı ekleme/silme, parola yöneticisi.
- Sonraki aday olarak işaretlenen ayarları yer tutucu, çalışıyor gibi görünen kontrollerle sunma.

## Kabul kriterleri
- [ ] Ayarlar Merkezi metin/düzen incelemesinden geçmiştir: sade adlar, gerekli açıklamalar ve içerikle belirlenen gruplar; dolgu alt başlıkları, em dash, dekoratif rozetler ve gerekçesiz üçlü kartlar yoktur.
- [ ] Üç menü tipi aynı arama/sabitleme/klavye davranışıyla çalışır; tip değişince kullanıcı verisi korunur ve oturum yeniden başlatılmaz.
- [ ] Panel alt/üst/sol/sağ kenarlara taşınır; exclusive zone, menü, hızlı ayarlar, bildirimler ve OSD yeni düzene doğru yerleşir.
- [ ] Panel/simge boyutu, hizalama ve tek/tüm monitör tercihleri canlı uygulanır ve yeniden girişte korunur; seçilen monitör çıkarılınca panel erişilebilir kalır.
- [ ] Geçersiz kişiselleştirme başarısızlığı son çalışan düzeni bozmaz; menü/panel varsayılanına dönüş diğer kişisel ayarları değiştirmez.
- [ ] DESIGN'deki dar/geniş, kısa yükseklik, büyük metin ve farklı ölçek matrisi denenmiştir; kategori listesi içerikle üst üste binmez ve formlar yatay kaydırma gerektirmez.
- [ ] Tüm MVP panel kimlikleri CLI/D-Bus ile açılır; aynı istek ikinci süreç oluşturmaz.
- [ ] Menüde Türkçe ayar araması doğru paneli açar; bilinmeyen panel güvenli hata verir.
- [ ] Görünüm, panel, bildirim, güç ve giriş ayarları yeniden girişten sonra korunur ve shell ile tutarlıdır.
- [ ] Uygulama engeli ile DND ayrı davranır; kritik bildirim istisnası yalnız DND için geçerlidir.
- [ ] Yanlış Wi-Fi parolası, Bluetooth kod reddi, cihaz kaybı ve servis yeniden başlatması uygulamayı çökertmez.
- [ ] Ses cihazı değişir ve uygulama başına düzey çalışır; kapanan akış geride satır bırakmaz.
- [ ] Ekran değişikliği onaylanınca korunur; 15 sn, settings çökmesi ve monitör kaybı deneylerinde geri alma çalışır.
- [ ] Yetki reddi/iptali uygulanmış ayar gibi görünmez; parola hassas veri denetiminden geçer.
- [ ] Bölge/dil değişikliğinin kullanıcı/sistem kapsamı ve yeniden giriş gereksinimi doğru açıklanır.
- [ ] Varsayılan uygulama seçimiyle ilgili dosya/bağlantı seçilen uygulamada açılır.
- [ ] Klavye gezinmesi, dar pencere, metin ölçeği ve açık/koyu görünüm bütün panellerde denenmiştir.
- [ ] Ayar doğrulama, yönlendirme ve ekran işleminin durum makinesi testleriyle genel kalite kontrolleri geçer.

## Doğrulama
- İki farklı kullanıcıyla tercih ayrımını ve yetki reddini dene; sistem değişikliği deneylerini geliştirme VM'sinde yap.
- Ekran geri alma için VM sanal çıkışı ve mümkünse iki fiziksel ekran kullan; koşulları ayrı raporla.
- Shell, hızlı ayarlar ve settings açıkken aynı değeri değiştir; üç yüzeyin tutarlı kaldığını doğrula.
- Üç menü tipi × dört panel kenarı için dar/geniş, kısa yükseklik ve farklı ölçek kontrollerini uygula. Açık bir popup varken konum değişimini ve tek monitör tercihiyle ekran çıkarılmasını dene; çalışma alanının eski kenardan boşaldığını doğrula.
- Genel build, clippy, fmt ve test kontrollerini çalıştır; panel başına en az bir başarı, iptal/hata ve eksik servis senaryosu raporla.

## Teslim
Uygulamaya geçildikten sonra alt grup planını sun. Bitince panel kapsam tablosu, ekran geri alma kanıtları, özel API kararları, doğrulanamayanlar ve M2 durumu raporlanır. Mimari, bileşen ve yol haritası belgeleri gerçek sonuçlarla güncellenir.
