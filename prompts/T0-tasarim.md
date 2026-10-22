# T0 — Odak tasarımı ve uygulama öncesi görünüm kararları

## Bağlam
Önce oku: `docs/DESIGN.md`, `docs/VISION.md`, `docs/PLANNING.md`, `docs/ARCHITECTURE.md`, `CLAUDE.md`. Kullanıcı Odak yönünü; derinlikli uygulama simgelerini, zarif çizgi simgeleri ve buzlu cam blur hedefini seçti. Proje hâlâ planlama aşamasındadır.

## Hedef
Faz 0–7 uygulanmadan önce kendi ekranlarımızın görünümü, dar/geniş davranışı ve varsayılan tema ailesi incelenebilir taslaklarla kararlaştırılır. Geliştirme fazlarının numarası değişmez; T0 tasarım ön koşuludur.

## Yapılacaklar
1. Panel/başlat, hızlı ayarlar, bildirim merkezi, settings, giriş/kilit/polkit ve Hoş geldin için Odak taslaklarını hazırla. Tüm yüzeyleri bir kerede kesinleştirmek yerine küçük ekran gruplarını sırayla kullanıcıya sun.
2. Renk ve tipografi hiyerarşisi, malzeme, köşe/gölge, boşluk ve etkileşim durumlarını ortak bileşen tablosuna bağla.
3. `docs/ICONOGRAPHY.md` kalite kapısıyla Dosyalar, Ayarlar Merkezi, Terminal ve Görüntüler için gerçek hacimli nesne örnekleri üret; önce bu grubu kullanıcıya incelet. Düz çizgi sembolü + renkli kutu yaklaşımı nihai çözüm değildir. Düzenlenebilir kaynak, malzeme/ışık tutarlılığı ve optik boyutlar dahil olsun. Çizgi ailesi ayrı olarak ağ/ses/güç/arama/kapat örnekleriyle gösterilir.
4. Her ekran grubu için dar/geniş, kısa yükseklik, uzun Türkçe metin, büyük yazı ve yüksek kontrast örnekleri hazırla. Tasarım kırılma noktaları içerikten türetilsin.
5. Cam malzemesinde hafif/dengeli blur ve opak uyumluluk görünümünü karşılaştır. Kullanıcı normal görünüm hedefini zaten seçmiştir; opak örnek bu tercihi geçersiz kılan alternatif değildir.
6. GTK3, GTK4/libadwaita, üçüncü taraf GTK4, Flatpak, Wayfire (21 Ekim'e kadar labwc), ikon ve imleç için görünüm eşleme tablosu çıkar. Paket seçimini önizleme çizimiyle karıştırma; K14 kararını somut set ve lisanslarla kapat.
7. K13 için gerçek blur seçeneklerini araştır: Debian 13'te paketlenebilirlik, layer-shell/toplevel/session-lock/output-management, GTK entegrasyonu, GPU/VM/yazılım render ve bakım maliyeti. Seçenekleri kanıtla sun; kullanıcının mimari kararı olmadan compositor değiştirme.
8. Onaylanan her görsel kararı DESIGN'e, mimari kararları ARCHITECTURE'e ve fazlara ait kabul kriterlerini ilgili prompt'a aktar. Tasarım seçiminin kod uygulaması anlamına gelmediğini koru.
9. Ayarlar Merkezi'nde menü tipleri, panelin dört kenarı, hizalama, boyut ve simge boyutu tercihlerini tasarla. Odak liste/klasik iki bölüm/ızgara menülerini aynı görsel ailede sun. Yatay/dikey panel, açılır yüzeylerin yönü, çoklu monitör geri dönüşü ve canlı önizlemeyi birlikte incele.
10. Bütün taslaklarda DESIGN'in arayüz metni ve düzen kurallarını uygula: sade sayfa/ayar adları, yalnız gerekli yardımcı metin; dolgu alt başlıkları, em dash, dekoratif rozetler ve gerekçesiz üçlü kart düzeni yok. Önceki önizlemeler bu ayrıntılar için onaylanmış referans değildir.

## Kapsam dışı
- Rust/GTK uygulaması, paket kurulumu, compositor fork'u geliştirme ve ISO üretimi.
- macOS uygulama simgelerini Asena markası olarak kopyalama ve sınırsız tema motoru.

## Kabul kriterleri
- [ ] Ekranlarda gereksiz başlık/alt başlık, ayarı yeniden anlatan metin, tanıtım sloganı, em dash, dekoratif renkli rozet veya gerekçesiz üçlü kart şablonu bulunmaz.
- [ ] Temel ekran grupları için incelenebilir taslaklar ve kullanıcı seçimleri kaydedilmiştir.
- [ ] İlk dört uygulama simgesi ICONOGRAPHY kalite kapısıyla incelenip kullanıcı tarafından kabul edilmiştir; çizgi ailesi ayrıca tanımlıdır.
- [ ] Üç menü tipi ve dört panel kenarı, Ayarlar Merkezi önizlemesi ve responsive durumlarıyla tasarlanmıştır; varsayılan Odak düzeni belirgindir.
- [ ] Cam, opak, açık/koyu ve responsive davranışlar okunabilir örneklerle gösterilmiştir.
- [ ] Varsayılan görünümün GTK/Flatpak/pencere kenarı/ikon/imleç kapsamı ve eksikleri belgelidir.
- [ ] Blur mimari kararı, protokol uyumu ve bakım maliyetiyle açıkça karara bağlanmış veya engel olarak bırakılmıştır.
- [ ] Uygulama kabul kriterleri ilgili fazlara işlenmiştir; gerçek Linux deneyi yapılmadan başarılı test sonucu yazılmamıştır.

## Teslim
Seçilen ekran taslakları, simge/malzeme örnekleri, açık kararlar ve sonraki tasarım grubu sunulur. T0 tamamlanmadan ve kullanıcı uygulamaya geçiş istemeden Faz 0 başlatılmaz.
