# Asena Odak — Görsel tasarım ve etkileşim

## Seçilen yön
18 Ekim 2026 kullanıcı kararı: **Odak**. Ana nitelikler zarafet, temizlik, odaklanma ve AAA düzeyinde işçilik hedefidir. Varsayılan düzen kompakt alt panel, solda liste ağırlıklı başlat menüsü ve içerik öncelikli uygulamalardır. Kullanıcı Ayarlar Merkezi'nden menü tipini ve panel konumunu değiştirebilir; bütün düzenler aynı Odak görsel dilini korur.

Onaylanan görsel hedefler:
- Uygulama simgelerinde macOS benzeri hacim, kontrollü ışık ve derinlik.
- Arayüz eylemlerinde zarif, tutarlı çizgi simgeler.
- Panel ve geçici shell yüzeylerinde buzlu cam hissi ve gerçek arka plan blur'u.
- Sakin yüzeyler, açık hiyerarşi, ölçülü vurgu, eksiksiz açık/koyu görünüm.
- Asena'nın kendi uygulamalarında pencere boyutuna uyarlanma.
- Ayarlar Merkezi'nden menü tipi ve panelin alt/üst/sol/sağ konumunun değiştirilebilmesi.

Bu belge tasarım kararıdır; çalışan GTK teması veya compositor uygulaması değildir. Kesin simge paketi, son marka simgesi, tüm ekran taslakları ve blur'un teknik yolu henüz seçilmedi.

## Yerleşim ve hiyerarşi
- Varsayılan alt panel tam genişlikte ve başlangıçta 48 mantıksal piksel olur. Başlat ve uygulamalar solda; sistem durumu ve saat sağdadır. Konum tercihiyle üst/sol/sağ yerleşime geçilir. İçerik sığmadığında görev çubuğu taşma menüsüne geçer; durum eylemleri kaybolmaz.
- Varsayılan başlat menüsü solda, panelin üstünde açılır. Panel taşındığında menü panelin ekran içine bakan kenarına bağlanır; arama ve güç eylemleri ekran dışında kalmaz. Satırlarda simge ve kısa ad; yalnız anlamlı olduğunda açıklama gösterilir.
- Settings geniş pencerede kategori listesi + içerik, dar pencerede kategori → içerik → geri gezinmesi kullanır. Büyük ekranlarda içerik satırları gereksiz uzamaz.
- Bir görünümde ana eylem kolayca ayırt edilir. Birbirine eşit her kutuya vurgu rengi veya gölge verilmez. İkincil ayarlar içerik hiyerarşisi içinde kalır.
- Panel ve menüde gereksiz bilgi, dekoratif rozetler, sürekli animasyon veya büyük marka alanı kullanılmaz.

## Uygulanan shell yüzeyleri (21 Ekim 2026)
Kullanıcının "Windows 11 kopyası" geri bildirimiyle shell yüzeyleri Odak diline göre yeniden kuruldu.
Bunlar çalışan koddur; görsel kabul açıktır.

- **Panel:** solda sekiz köşeli yıldız başlat simgesi (vurgu rengi), ortada/başta görev çubuğu, sağda
  tepsi, durum kümesi ve iki satırlı saat (saat + kısa tarih, panel 44 px ve üstündeyse). Görev
  çubuğu simgeleri her durumda aynı merkezde durur; çalışan uygulama ekran kenarında kısa bir çizgi,
  etkin uygulama vurgu renginde geniş çizgi ve hafif zemin taşır.
- **Durum kümesi:** ağ (Wi-Fi gücü, kablolu, bağlanıyor, kapalı), Bluetooth (yalnız açıkken), ses
  düzeyi/sessiz, pil (düzey ve şarj; %20 ve altında yüzde metni) ve rahatsız etme. Servis veya donanım
  yoksa ilgili simge gizlenir; hiçbiri yoksa tek bir sistem simgesi düğmeyi erişilebilir tutar.
- **Başlat menüsü (Odak listesi):** tek sütun; sabitlenenler, ardından harf başlıklı bütün uygulamalar.
  Harf başlığına tıklamak menünün içinde Türk alfabesi ızgarasını açar; liste daralmaz. Arama alanı
  her zaman panele bakan kenardadır (alt panelde menünün altında). Hesap, Ayarlar, kilit ve güç üst
  satırdadır. Sağ tık menüye/panele sabitleme sunar. Klasik (kategoriler) ve ızgara düzenleri aynı
  dizin, arama ve sabitleri paylaşır. Ayar araması gerçek panellere götürür.
- **Denetim merkezi:** üstte ses ve parlaklık sürgüleri (birden fazla çıkış aygıtında seçici), ortada
  bağlantı satırları (Wi-Fi, kablolu, Bluetooth; anahtar ve yerinde açılan ağ/aygıt listesi), altında
  hap biçimli modlar (rahatsız etme, koyu stil, gece ışığı, güç profili), en altta pil durumu ve
  ekran görüntüsü/Ayarlar/kilit/güç düğmeleri. Eşit kutu ızgarası kullanılmaz.
- **Bildirim kenar paneli:** saate tıklamak ekranın sağından boydan boya kayan bir panel açar
  (Budgie Raven'a benzer yaklaşım): gün ve tarih, takvim, MPRIS medya kartı ve tüm yüksekliği kullanan
  bildirim listesi. Medya denetimleri yalnız burada bulunur.
- **Masaüstü:** Ev ve Masaüstü klasörü içeriği simge olarak gösterilir; çift tık veya Enter açar.
  Boş alana sağ tık: yeni klasör, uçbirimde aç, Dosyalar'da aç, masaüstü simgelerini göster, arka planı
  değiştir, ekran ayarları, masaüstü ve panel ayarları. Simgeye sağ tık: aç, yeniden adlandır, çöpe taşı.
- **Pencere düğmeleri:** Asena uygulamaları ve üçüncü taraf GTK uygulamaları küçült, büyüt ve kapat
  düğmelerini gösterir (`appmenu:minimize,maximize,close`).

## Arayüz metni ve düzen
Kullanıcı arayüzünde kısa adlar, doğrudan eylemler ve gerçek durum bilgisi kullanılır. Kullanıcının son yönlendirmesiyle aşağıdaki kurallar bütün Asena ekranlarına ve sonraki taslaklara uygulanır:

- Her sayfada büyük başlık + açıklama alt başlığı kalıbı kurulmaz. Sayfa adı ve gerekli bölüm adları yeterlidir; pencere başlığı aynı içerikte tekrar tekrar yazılmaz.
- Ayarın adını yeniden anlatan metin eklenmez. “Panel konumu” ve seçili “Alt” değeri yeterlidir; altına “Panelinizin konumunu buradan değiştirebilirsiniz” yazılmaz.
- “Masaüstünü kendine göre düzenle”, “Deneyimini geliştir”, “Güçlü ve sezgisel” gibi tanıtım cümleleri ürün ekranlarında bulunmaz. “Odak”, “AAA” ve tasarım hedefleri de ayar sayfasına slogan/rozet olarak taşınmaz.
- Uygulamanın kendi etiket ve açıklama metinlerinde em dash (U+2014) kullanılmaz. Başlıkları noktalama veya sloganla süsleme; gerekli açıklamayı kısa ayrı cümleyle ver. Kullanıcı içeriği, uygulama adları ve dış kaynaklı bildirim metinleri bu editoryal kural için değiştirilmez.
- Dekoratif renkli rozet, “Yeni”, “Önerilen”, “Premium” gibi dolgu etiketleri ve işlevi olmayan durum hapları eklenmez. Okunmamış bildirim sayısı gerçek veri olarak sade gösterilir; renkli rozet zorunluluğu yoktur.
- Sayfa varsayılan olarak üç eşit kart, özet kutuları veya kontrol paneli düzenine bölünmez. Ayarlar doğal satırlar ve anlamlı gruplar halinde düzenlenir; sütun sayısını gerçek içerik ve kullanılabilir alan belirler.
- İlişkisiz kontrollere aynı boyut renkli kartlar verilmez. Hızlı ayar düğmesi, gerçek bildirim veya medya içeriği gibi işlevsel yüzeyler kendi gereksinimleriyle tasarlanır; dekoratif kart sayısını tamamlamak için içerik üretilmez.
- Renk seçili kontrol, odak ve gerçek durum için kullanılır. Her kategori farklı renge boyanmaz. Hacimli uygulama simgelerinin malzeme renkleri kendi kimliğini korur; bu karar renksiz bir simge seti talebi değildir.
- Teknik terim, mimari ayrıntı, geliştirme notu ve üretim süreci normal ürün ekranlarına yazılmaz. Kullanıcının karar vermesi için gerekli uyumluluk veya hata bilgisi ilgili yerde verilir.
- Yardım metni yalnız belirsizliği, işlemin sonucunu, güvenlik/veri kaybı etkisini veya çözülebilir hatayı açıklıyorsa kullanılır. Ağ adı, parça sanatçısı, dosya yolu ve arama sonucunu ayırt eden gerçek ikincil bilgi korunur; her satıra alt başlık eklenmez.

Örnek normal kullanım: “Görünüm”, “Menü tipi”, “Panel konumu”, “Simge boyutu”, “Varsayılana dön”. Kalite; hizalama, tipografi, malzeme, simge işçiliği ve etkileşim tutarlılığıyla sağlanır.

## İki simge ailesi

### Uygulama simgeleri
- Gerçek nesne hacmi, kalınlık, ön/yan yüz ilişkisi, kenar pahları ve iç derinlik taşır. Ortak ışık, malzeme yansıması ve parça örtüşmesindeki temas/ortam gölgelemesi bu yapıyı görünür kılar. Üretim ve kabul sözleşmesi [ICONOGRAPHY.md](ICONOGRAPHY.md) içindedir.
- Önceki önizlemelerdeki düz çizgi sembolünün renkli kutuya yerleştirilmesi kalite hedefini karşılamaz. Dört özgün nesne örneği kullanıcı tarafından incelenmeden bütün sete geçilmez.
- Asena'nın simgeleri aynı perspektif, optik ağırlık, köşe ailesi ve ışık yönünü paylaşır. Her uygulamanın nesnesi ayırt edilir; yalnız renk değiştirilmiş aynı kutu olmaz.
- macOS referansı malzeme ve derinlik içindir. Asena'ya özgü çizimler hazırlanır; başka ürünlerin uygulama simgeleri Asena markası olarak kullanılmaz.
- Menü satırları için 32–36, panel için 24–28 mantıksal piksel başlangıç ölçüleridir; küçük boyutta ayrıntı sadeleştirilir. Daha büyük önizleme küçük simge çiziminin yerine geçmez.
- Üçüncü taraf uygulamalar kendi kimliğini korur. Özel simge teması seçilirse kapsam, miras alınan set, eksik simge geri dönüşü, lisans ve bakım durumu yazılır.

### Arayüz/çizgi simgeleri
- Ağ, ses, arama, geri, kapat, ayar ve benzeri eylemler tek bir sembolik aile kullanır. Aynı satırda farklı çizgi dilleri karıştırılmaz.
- 24 px referansta yaklaşık 1,5 px çizgi başlangıç önerisidir; 16/20 px için optik kalınlık ve piksel hizası ayrıca düzenlenir. İncelik okunabilirliği azaltmamalı.
- Etkin, seçili, devre dışı, hover ve odak durumları tanımlanır. Kritik durum yalnız renk veya çok küçük çizgi farkıyla anlatılmaz.
- Kendi uygulamalarında derinlikli uygulama simgesi ile sade sembolik kontrolün rolleri ayrıdır. Önizlemedeki temsilî çizimler nihai simge paketi sayılmaz.

## Buzlu cam ve yüzey malzemesi
- Gerçek blur hedefi panel, başlat menüsü, hızlı ayarlar, bildirim merkezi ve uygun geçici bildirim yüzeyleridir. Ana uygulama içerikleri okunaklı, sakin ve büyük ölçüde opak olur.
- Cam malzemesi: bulanık arka plan, açık/koyu moda uygun nötr renk örtüsü, hafif kenar aydınlığı ve yumuşak ayrışma gölgesi. Metin, simge ve kontroller bulanıklaştırılmaz.
- Ayrıntılı veya çok aydınlık duvar kâğıdı üzerinde metin kontrastı korunur. Opaklık/blur değerleri taslaklarda karşılaştırılır; üretim ölçüleri gerçek compositor ve GPU ölçümüyle seçilir.
- Yüksek kontrast, azaltılmış saydamlık tercihi, desteklenmeyen donanım ve yazılım render yolunda opak malzeme bulunur. Bu erişim/uyumluluk modu, normal görünümdeki gerçek blur hedefini tamamlanmış saydırmaz.
- Sürekli ekran görüntüsü alıp arka planı bulanıklaştıran taklit çözüm kullanılmaz. Masaüstünün başka pencerelerini bulanıklaştırma işi compositor katmanında ele alınır; GTK içeriğine CSS blur vermekle eşdeğer değildir.
- Blur'u Wayfire 0.9'un `blur` eklentisi Dual Kawase ile çizer; shell yüzeyleri layer-shell ad alanlarıyla (`asena-panel`, `asena-menu`, `asena-quick-settings`, `asena-notification-center`) eşleşir. K13 deneyi ve geçiş kararı: [blur-options.md](research/blur-options.md), [wayfire-migration.md](reports/wayfire-migration.md).
- Görünüm ayarlarındaki **Saydamlık efektleri** açıkken panel %78, yüzeyler %80 opaklıkta cam malzemesi kullanır. Kapalıyken veya yüksek kontrastta opak malzemeye dönülür ve blur eklentisi yüklenmez. Bu değerler geçicidir; gerçek GPU ve görsel kabulle kesinleşir.
- Blur fiziksel piksel üzerinden hesaplandığı için %200 ölçekte mantıksal yarıçap yarıya iner. Ölçeğe göre ayar gerçek donanım ölçümünden sonra değerlendirilir.

## Varsayılan görünüm / GTK teması
- Çalışma adı **Asena Odak** olan varsayılan görünüm; shell, uygulamalar, pencere kenarları, ikonlar, imleç ve duvar kâğıdını birlikte kapsar. Kullanıcı ilk açılışta bunları tek tutarlı masaüstü olarak görür.
- GTK4/libadwaita uygulamalarında desteklenen açık/koyu, vurgu, tipografi ve uygulamaya ait stil yolu kullanılır. Ortak bileşenler `asena-widgets` üzerinden tutarlı davranır.
- GTK3 için uyumlu bir tema; diğer GTK4 uygulamaları için desteklenen tema/ayar yolu ve Flatpak görünümü ayrı doğrulanır. Tüm uygulamalara tek CSS dosyasının uygulanacağı varsayılmaz.
- Varsayılan GTK3 teması, simge/imleç setleri, lisans ve paketleme kararı **K14** kapsamında netleşir. Mevcut mimarideki adw-gtk3 bir adaydır; kullanıcı bu adla bir paketi henüz seçmedi.
- Turkuaz mevcut vurgu varsayımıdır. Tam palet, duvar kâğıdı ve logo tasarım örnekleriyle kapanır. Kontrol/yüzey renkleri semantik tema token'larından gelir; ikon varlıklarının kendi malzeme paleti UI kontrol renkleriyle karıştırılmaz.

## Ayarlar Merkezi — masaüstü kişiselleştirme
Kullanıcı kararıyla menü tipi ve panel konumu 1.0 kapsamına alınmıştır. Odak varsayılanı, kullanıcı tercihini engelleyen sabit bir düzen değildir.

| Kontrol | Planlanan seçenek / davranış |
|---|---|
| Menü tipi | Odak kompakt liste; klasik iki bölümlü menü; uygulama ızgarası |
| Panel konumu | Alt, üst, sol, sağ |
| Uygulama hizası | Panelin başlangıcı, ortası veya sonu; dikey panelde yönü doğru adlandırılır |
| Panel kalınlığı ve simge boyutu | Birbirine uyumlu, tasarımda doğrulanan boyutlar; kontroller kırpılmaz |
| İçerik sırası | Sabitlenmiş uygulamaları yeniden sıralama |
| Monitör kapsamı | Tüm monitörler veya seçilen tek monitör; ekran çıkarıldığında erişilebilir geri dönüş |
| Görünüm ve malzeme | Açık/koyu/sistem, vurgu, cam/azaltılmış saydamlık ve desteklenen blur düzeyi |
| Geri dönüş | Menü/panel düzenini Odak varsayılanına döndür; diğer kişisel tercihleri sıfırlama |

- Seçenek adları ve son menü çizimleri T0'da incelenir; üç menü tipi aynı uygulama indeksini, aramayı, sabitlemeleri ve klavye eylemlerini paylaşır.
- Değişiklikler küçük canlı önizleme ve gerçek shell'de yeniden giriş gerektirmeden uygulanır. Menü tipi değişince uygulama listesi, sabitler ve arama davranışı kaybolmaz.
- Panel konumuyla exclusive zone eski kenardan kaldırılır, yeni kenarda uygulanır. Menü, hızlı ayarlar, takvim/bildirim, popup ve OSD konumları aynı yerleşim modelini izler.
- Dikey panelde içerik ve simgeler dik durur; saat/durum ve tepsi için özel dar düzen kullanılır. Yalnız bütün paneli döndürmek yeterli değildir.
- Çoklu monitör, farklı ölçek ve dar alanda bütün seçenekler kullanılabilir kalır. Topoloji değişirse en az bir erişilebilir panel ve klavye yolu korunur.
- Boyut ve konum birlikte doğrulanır; arızalı yerleşim kaydedilmez. Kişiselleştirme değişikliği başarısızsa son çalışan düzen korunur; varsayılanlara dönüş klavyeyle de erişilir.
- Otomatik gizleme ve serbest eklenti/tema motoru ayrı sonraki adaydır; menü tipi ve panel kenarı tercihi bu özelliklere bağlı değildir.

## Responsive / uyarlanabilir davranış
- Yerleşim fiziksel ekran çözünürlüğüne göre değil, kullanılabilir mantıksal pencere alanına göre değişir. Kesin kırılma noktaları içerik ve Türkçe metinlerle seçilir.
- Tasarım örnekleri en az 360, 600, 900 ve 1280 mantıksal piksel genişlikte incelenir. Bunlar her masaüstü ekranının aynı yüksekliği sağladığı anlamına gelmez; kısa yükseklik ve kullanılabilir çalışma alanı da ele alınır.
- Dar görünümde kenar listesi ayrı sayfaya dönüşür; iki sütun tek sütun olur; uygulama ızgarası/satırları alanı izler. Menü/dialog ekran sınırına göre konumlanır; eylemler ekran dışında kalmaz.
- Uzun Türkçe/İngilizce metin, %200 metin ölçeği, %100/%125/%150/%200 ekran ölçeği, dikey ekran ve iki farklı ölçekli monitör senaryoları tasarım kabul listesine girer.
- Formlar ve metinler yatay kaydırma istemez. Listeler dikey kayabilir; sabit alt eylemler içeriği örtmez. Kapanan veya yeniden düzenlenen yüzey odağı kaybetmez.
- Kendi uygulamalarımız bu sözleşmeye uyar. Dağıtıma seçilen üçüncü taraf uygulamaların uyumu ayrıca değerlendirilir; değiştirmediğimiz uygulamalara destek garantisi verilmez.

## İşçilik ve kabul
- Her ekran arayüz metni ve düzen kurallarıyla incelenir. Gereksiz açıklama alt başlıkları, em dash kullanılan uygulama metinleri, dekoratif rozetler ve gerekçesiz üçlü kart şablonları kabul edilmez.
- Her temel yüzeyin normal, hover, basılı, klavye odağı, devre dışı, yükleniyor, boş, hata ve başarı durumları tasarlanır; uygulanmayan durumlar gerekçesiyle işaretlenir.
- Görsel kabul, açık/koyu ve opak/cam örneklerini, gerçek küçük simge ölçülerini, uzun metni ve dar görünümü birlikte içerir.
- Yazı ve semboller her duvar kâğıdı örneğinde okunur; camın arkasındaki hareket etkileşimi zorlaştırmaz. Klavye odağı ve yüksek kontrast baştan tasarlanır.
- Animasyonlar kısa, kesilebilir ve işlevi açıklayıcıdır; hareket azaltma tercihinde zorunlu olmayan hareket kalkar. Hareket/geçişler ilgili VISION bütçelerini izler.
- Piksel hizası, simge optik boyutu, köşe eşleşmesi, metin taşması, titreşim ve gölge tutarlılığı son görsel incelemeye dahildir. “AAA” hedefi bu somut incelemeler ve ölçümlerle takip edilir; yalnız stil adı olarak bırakılmaz.
- Blur açık/kapalı karşılaştırmasında kare süreleri, GPU/CPU ve RSS ölçülür. VISION performans bütçeleri korunur; başarısızlık görsel kalite hedefini sessizce düşürerek kapatılmaz.

## Fazlara aktarım
- **Tasarım hazırlığı (T0):** onaylı Odak taslakları, gerçek hacimli simge örnekleri, üç menü tipi/dört panel kenarı taslakları, malzeme, GTK tema eşlemesi ve responsive ekran matrisi.
- **Faz 0:** blur/protokol/GTK uyumluluk araştırması ve tema altyapısının doğrulaması. K13 üretim shell'i başlamadan çözülür.
- **Faz 1–3:** panel, menü, bildirim, hızlı ayarlar ve simgeler tasarım sözleşmesiyle birlikte uygulanır. Ortak yerleşim modeli ve menü sunum katmanı Faz 5 kişiselleştirmesine baştan zemin hazırlar.
- **Faz 4–5:** giriş/kilit/polkit ve Ayarlar Merkezi aynı malzeme/etkileşim ailesine, güvenlik ve okunabilirlik sınırlarıyla uyar. Menü tipleri, panel konumları ve canlı kişiselleştirme Faz 5'te tamamlanır.
- **Faz 6:** varsayılan tema/simge/imleç paketleri, lisanslar ve ISO ilk görünümü tamamlanır.
- **Faz 7:** son işçilik, erişilebilirlik ve performans regresyonları kontrol edilir. Responsive ve temel görünüm bu faza ertelenmez.
