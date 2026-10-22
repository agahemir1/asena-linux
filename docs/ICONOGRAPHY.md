# Asena — Uygulama simgelerinin üretimi ve kalite kapısı

## Kullanıcı kararı
Uygulama simgelerinde gerçek hacim ve yüksek işçilik isteniyor. Önizlemelerdeki renkli yuvarlatılmış kutu + çizgi sembolü yaklaşımı kalite hedefini karşılamadı. Önceki örnekler yerleşim taslağıdır; nihai simge yönünün onayı veya üretim varlığı sayılmaz.

## Hacmin tanımı
- Simgenin nesnesi hacimlidir: kalınlığı, ön/yan yüz ilişkisi, kenar pahları, iç boşlukları ve parçalar arası derinliği okunur.
- Üst soldan ortak ana ışık, kontrollü dolgu ışığı, malzemeye uygun yansıma ve parçaların birbirini örttüğü yerde temas/ortam gölgelemesi bulunur.
- Cam, metal, kâğıt, seramik veya benzeri malzemeler nesnenin anlamına göre ayrılır. Bütün simgelere aynı parlak kaplama uygulanmaz.
- Perspektif, ışık sıcaklığı, optik ağırlık, kadraj ve gölge dili set boyunca tutarlıdır. Aşırı perspektif ya da yansıma uygulamanın tanınmasını zorlaştırmaz.
- Derinlik nesnenin yapısından gelir. Tek düz sembole gradient, dış gölge veya bevel filtresi eklemek tek başına yeterli değildir.
- Uygulama simgeleri bitmiş görsel varlık olarak sunulur; masaüstünde sürekli çalışan bir 3D motor gerektirmez. 3D model + render veya eşdeğer özenle hazırlanmış katmanlı illüstrasyon üretim yolu T0'da örneklerle seçilir.

## İlk üretim grubu
Önce dört simge hazırlanıp kullanıcıya ayrı bir simge incelemesi olarak sunulur:

| Simge | Aranacak nesne/derinlik özelliği |
|---|---|
| Dosyalar | Ayrı ön/arka kapak, kâğıt katmanları ve gerçek kalınlık hissi |
| Ayarlar Merkezi | Hacimli mekanik/işlevsel nesne, oyuk ve parça örtüşmeleri |
| Terminal | Ekran/çerçeve kalınlığı, derinde içerik yüzeyi, kontrollü cam hissi |
| Görüntüler | Çerçeve ve görüntü düzleminin ayrılması; setle uyumlu malzeme |

Bunlar başlangıç brief'leridir; son nesne çizimleri kullanıcı incelemesiyle seçilir. macOS benzeri malzeme kalitesi hedeflenir; çizimler Asena'nın özgün kimliğini taşır.

## Teslim varlıkları
- Düzenlenebilir ana kaynak, kullanılan üretim yöntemi, malzeme/ışık ayarları ve lisans kaydı.
- En az 1024 px şeffaf ana çıktı; 16, 24, 32, 48, 64, 128 ve 256 px kullanım örnekleri. Her boyut yalnız otomatik küçültülmüş ana görselden ibaret olmaz; gerektiğinde optik sadeleştirme yapılır.
- Açık, koyu, cam ve opak yüzeylerde aynı nesnenin okunurluk incelemesi. Küçük simgelerde ince detay kırılması, ışıklı kenar saçılması ve aşırı yumuşama giderilir.
- İkon tema dizini, kimlik/alias ve miras zinciri; eksik üçüncü taraf simgesinde kimliği koruyan geri dönüş. Ayarlar Merkezi'ndeki simge boyutu değişimlerinde doğru varlık seçimi.

## Üretim durumu (21 Ekim 2026)
- **Çizgi (sembolik) simgeler:** `data/icons/Asena/symbolic/` altında 264 dosya; düzenlenebilir kaynak
  `scripts/icons/symbolic.py`. 16 birimlik ızgara, 1,2 birim çizgi (24 px'te yaklaşık 1,8 px), yuvarlak
  uç ve birleşim. GTK sembolik simgeleri yalnız dolguyu yeniden renklendirdiği için her çizgi dolgulu
  dış hat olarak üretilir; kesikler (slash) clip path ile açılır. Standart freedesktop/Adwaita adlarını
  kullanır; `Asena` simge teması `Inherits=Adwaita,hicolor` ile eksik adları Adwaita'dan alır.
- **Hacimli uygulama simgeleri:** ilk inceleme grubu Dosyalar, Ayarlar, Terminal ve Fotoğraflar
  (Görüntüler) `scripts/icons/apps.py` ile katmanlı SVG olarak üretildi; ortak sol üst ışık, nesne
  kalınlığı ve tek zemin gölgesi taşır.
- **Kullanıcı kararı (22 Ekim 2026):** vurgu rengine göre klasör simgeleri **onaylandı**. Kullanıcı, yedek
  simgede kalan bütün Asena uygulamalarının simgesinin çizilmesini istedi; böylece set ilk dört simgenin
  onayını beklemeden genişletildi. Aynı dille çizilen 12 simge: Metin Düzenleyici (kâğıt ve kalem),
  Notlar (spiralli defter), Hesap Makinesi (kabarık tuşlu cihaz), Arşiv (fermuarlı kutu), Takvim
  (halkalı masa takvimi), Saat (metal çerçeveli kadran), Oynatıcı (plak), Ekran Görüntüsü (seçim
  çerçeveli ekran), Kamera (gövde ve cam objektif), Belgeler (sayfalar ve büyüteç), Yazılım (Asena
  yıldızlı çanta), Hava Durumu (bulut ve güneş). 16 uygulamanın hepsinin kendi simgesi var; uygulama
  simgelerinin görsel onayı **bekliyor**.
- **Bağlantı:** masaüstü dosyası `Icon=` adı uygulama kimliğidir; `AsenaWindowBuilder` pencere simgesini
  varsayılan olarak uygulama kimliğinden alır ve Hakkında penceresi aynı adı kullanır.
  `tests/test_icons.py` her masaüstü dosyasının simgesinin çizildiğini ve üreticiyle aynı olduğunu
  denetler.
- **Yedek simge:** `org.asenalinux.App` nötr seramik blok üzerinde sekiz köşeli yıldız. Artık hiçbir
  Asena uygulaması kullanmaz; yalnız simgesi olmayan üçüncü taraf uygulamalar için menü ve görev
  çubuğunda yedektir.
- **Kurulum:** uygulama simgeleri `hicolor/scalable/apps` ve `Asena/scalable/apps` altına kurulur
  (ikinci kopya, Adwaita'nın genel adlarının hacimli simgelerin önüne geçmesini engeller). Geliştirmede
  `scripts/compile-data.sh` bunları `target/share/icons` altına kopyalar. Asena süreçleri simge temasını
  vurgu rengine göre `asena_core::theme::icon_theme` ile seçer; gerçek oturumda daemon aynı adı
  `org.gnome.desktop.interface icon-theme` anahtarına yazar ve üçüncü taraf GTK uygulamaları da izler.
- **Vurgu rengine göre klasörler (22 Ekim 2026):** tam renkli klasörler (`folder`, `folder-documents`,
  `folder-download`, `folder-music`, `folder-pictures`, `folder-videos`, `folder-templates`,
  `folder-publicshare`, `folder-remote`, `folder-open`, `folder-drag-accept`, `user-home`, `user-desktop`)
  Dosyalar simgesinin malzemesiyle çizilir; özel klasörlerin ön kapağına ilgili çizgi simge 40 birimlik
  kabartma olarak basılır (32 px listede seçilir). Dosyalar ve Ayarlar simgeleri de vurguyu izler.
  Çizimler teal yapılır ve `Asena` temasında durur; diğer sekiz vurgu için `Asena-<vurgu>` temaları
  `Inherits=Asena` ile yalnız bu 15 simgeyi taşır (Yaru'nun vurgu varyantlarıyla aynı düzen). Renkli
  katmanların tonu vurguya kaydırılır, açıklık korunur; teal dosyalar inceleme bekleyen hâliyle bayt
  bayt aynıdır. Kaynak `scripts/icons/apps.py`, eşitlik denetimi `tests/test_icons.py`. Klasörler de
  aşağıdaki kullanıcı inceleme kapısına tabidir.
- **İnceleme sayfaları:** her boyutta açık/koyu zeminde işleme GdkPixbuf ile VM'de yapıldı; GTK'nin
  sembolik yeniden renklendirmesi oturumda ayrıca görüldü (panel ve menü).

## Kabul kapısı
- [ ] Dört örnek birlikte ve tek tek gerçek kullanım boyutlarında incelenmiştir.
- [ ] Hacim, kalınlık, malzeme ve parça örtüşmeleri görünür; düz çizgi sembolü + renkli taban yaklaşımına düşülmemiştir.
- [ ] Ortak ışık/perspektif/optik ağırlık vardır; uygulamalar birbirinden ayırt edilir.
- [ ] Kenarlar temiz, şeffaflıkta hale yok, küçük boyutta detaylar dağılmıyor; açık/koyu üzerinde okunur.
- [ ] Kaynak dosyası, dışa aktarım boyutları, kimlikler ve lisanslar teslim edilmiştir.
- [ ] Kullanıcı uygulama simgelerini görsel olarak kabul etmiştir. (Klasörler 22 Ekim'de onaylandı; set kullanıcının isteğiyle onaydan önce genişletildi.)

AAA hedefi bu incelemelerin yerine geçen bir etiket değildir. Kalite kapısını geçmeyen simge, daha çok uygulamaya çoğaltılarak tamamlanmış sayılmaz. Arayüzdeki zarif çizgi simgeleri ayrı aile olarak DESIGN sözleşmesini izler.
