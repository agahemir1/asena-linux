# Asena Pencere Hiyerarşisi ve Yerleşim Standartları (App Layouts)

Asena Desktop ekosisteminde yer alan uygulamaların pencere geometrisi, içerik akışı ve adaptif ekran düzeni kurallarını belirler.

---

## 1. Dört Temel Yerleşim Deseni

Tüm 16 Asena uygulaması amaçlarına göre aşağıdaki 4 ana mimari desenden birini uygular:

### Desen A: Gezintili İki Sütunlu Yerleşim (Navigation Split Layout)
- **Uygulamalar:** Asena Files, Asena Settings, Asena Store, Asena Notes, Asena Calendar.
- **Yapı:**
  - **Kenar Çubuğu (Sidebar):** 220px–280px sabit/yeniden boyutlandırılabilir genişlik. Klasör ağacı, kategoriler, etiketler veya takvimler.
  - **İçerik Alanı (Content):** Esnek genişlikte liste, ızgara veya detay görünümü.
- **Kırılma Noktası (Breakpoint):** 720px altında kenar çubuğu otomatik olarak gizlenir veya tek sütunlu `AdwNavigationView` yapısına dönüşür.

### Desen B: Belge ve Çalışma Alanı Yerleşimi (Document & Workspace Layout)
- **Uygulamalar:** Asena Text, Asena Documents, Asena Terminal.
- **Yapı:**
  - **Üst Çubuk:** Sekme barı (`AdwTabBar` / `GtkNotebook`) + belge araçları.
  - **Çalışma Alanı:** Metin tamponu, PDF canvas'ı veya VTE terminal yüzeyi.
  - **İsteğe Bağlı Yan Panel:** İçindekiler, arama/değiştirme çubuğu, sayfa önizlemeleri.

### Desen C: Medya ve Galeri Yerleşimi (Media Canvas Layout)
- **Uygulamalar:** Asena Photos, Asena Player, Asena Camera.
- **Yapı:**
  - **Canvas Odaklı:** İçerik tam genişlikte görsel/video akışıdır.
  - **Yüzen veya Otomatik Gizlenen Araçlar:** Player'da fare hareketsizliğinde kaybolan alt oynatma kontrolleri; Photos'ta üst araç çubuğu ve alt küçük resim şeridi.
  - **Tam Ekran (F11):** Başlık çubuğu ve panel gizlenerek saf medya odağı sağlanır.

### Desen D: Kompakt Yardımcı Araç Yerleşimi (Compact Utility Layout)
- **Uygulamalar:** Asena Calculator, Asena Clock, Asena Weather, Asena Capture, Asena Archive.
- **Yapı:**
  - Sabit veya kontrollü pencere boyutu (Örn: Hesap makinesi 360×540px, Saat 440×580px).
  - Görünüm Seçici (`ViewSwitcher`): Modlar veya sekmeler arasında hızlı geçiş.
  - Dikey akışlı, sade kart veya tuş takımı dizilimi.

---

## 2. Kırılma Noktaları ve Adaptif Boyutlar

| Ekran Sınıfı | Genişlik Aralığı | Düzen Davranışı |
|---|---|---|
| **Kompakt (Mobile/Half-Screen)** | 360px – 600px | Tek sütun; kenar çubuğu çekmeceye (overlay/sub-page) geçer; arama çubuğu tam genişlik açılır. |
| **Orta (Standard Desktop)** | 601px – 1024px | Standart iki sütun; ızgaralarda 3–4 sütun; araç çubuğunda simge + kısa metin. |
| **Geniş (Large/HiDPI Desktop)** | 1025px+ | İki sütun + önizleme paneli (Files çift panel); zengin veri tabloları; 5–6 sütunlu ızgara. |

---

## 3. Pencere Başlık Çubuğu Standartları

1. Her pencerenin sol tarafında Asena tema uyumlu kapatma/büyütme/küçültme kontrolleri ya da sağda tekil pencere kontrol grubu Asena düğme düzenini (`appmenu:minimize,maximize,close`) izler.
2. Başlık etiketi pencere ortasında yer alır; dosya veya not açıkken: `<Dosya Adı> — <Uygulama Adı>` biçiminde başlıklandırılır.
3. Kaydedilmemiş değişiklik olduğunda başlığın yanında veya sekmede küçük bir değişiklik göstergesi (`*` veya nokta) yer alır.

---

## 4. 16 Uygulamanın Yerleşim Eşlemesi

| Uygulama | Temel Desen | Minimum Boyut | Varsayılan Boyut | Özgün Bileşenler |
|---|---|---|---|---|
| **Files** | Desen A | 640×480 | 1000×650 | Çift panel (`AdwNavigationSplitView`), Liste/Izgara görünümü, Önizleme çubuğu |
| **Settings** | Desen A | 500×400 | 980×680 | 17 MVP paneli, Arama alanı, Tercih grupları |
| **Store** | Desen A | 600×500 | 950×700 | Kategori kenar çubuğu, Öne çıkanlar banner'ı, İndirme ilerleme çubuğu |
| **Terminal** | Desen B | 480×320 | 800×500 | Sekme çubuğu, Yatay/Dikey bölme, Arama çubuğu |
| **Documents** | Desen B | 500×400 | 900×700 | Sayfa önizleme kenar çubuğu, Yakınlaştırma kaydırıcısı, Vurgulama araçları |
| **Photos** | Desen C | 500×400 | 960×650 | Adaptif fotoğraf ızgarası, Tam ekran slayt, EXIF bilgi paneli |
| **Player** | Desen C | 400×300 | 850×550 | Video oynatma yüzeyi, Oynatma listesi çekmecesi, MPRIS entegrasyonu |
| **Text** | Desen B | 450×350 | 850×600 | Sekmeler, Satır numarası sütunu, Markdown canlı önizleme |
| **Archive** | Desen D | 400×350 | 680×480 | Arşiv içeriği liste görünümü, Çıkarma/Ekleme araç çubuğu |
| **Calculator** | Desen D | 320×440 | 360×540 | Tuş takımı ızgarası, Mod seçici, Geçmiş çekmecesi |
| **Calendar** | Desen A | 600×450 | 900×650 | Gün/Hafta/Ay görünüm seçici, Etkinlik oluşturma popover'ı |
| **Notes** | Desen A | 550×400 | 850×600 | Not listesi, Markdown düzenleyici, Önizleme bölmesi |
| **Clock** | Desen D | 380×450 | 440×580 | Dünya saatleri, Alarm kartları, Kronometre göstergesi |
| **Capture** | Desen D | 350×250 | 450×320 | Mod seçici (Ekran/Pencere/Alan), Süre sayacı, Çizim araçları |
| **Camera** | Desen C | 400×350 | 750×550 | Canlı vizör, Çekim butonu, Son çekimler şeridi |
| **Weather** | Desen D | 400×500 | 500×680 | Güncel hava kartı, Saatlik kaydırma şeridi, 7 günlük tahmin tablosu |
