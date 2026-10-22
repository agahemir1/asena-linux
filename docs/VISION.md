# Asena — Vizyon ve Tasarım İlkeleri

## Tek cümlede
Asena, Debian üzerinde çalışan, tanıdık **alt panel + başlat menüsü** düzenini modern GTK4/libadwaita estetiğiyle birleştiren, **Türkçe öncelikli** bir Wayland masaüstüdür.

## Kimin için
- Windows'tan gelen ve alışık olduğu düzeni arayan kullanıcılar
- Türkçe konuşan kullanıcılar, öğrenciler ve okullar
- Sade, hızlı ve tutarlı bir masaüstü isteyen geliştiriciler

## İlkeler (çatışırsa üstteki kazanır)
1. **Kararlılık özellikten önce gelir.** Çöken veya donan bir panel, eksik bir özellikten kötüdür. Bir servis yoksa ilgili öğe gizlenir, shell çalışmaya devam eder.
2. **Tanıdık ama modern.** Düzen bildik olsun (alt panel, başlat menüsü, sağ altta saat), görünüm 2026'ya ait olsun.
3. **Tek tasarım dili.** Shell ve uygulamalar libadwaita'nın renk ve tipografi sistemini paylaşır. Shell'e özgü her şey bu sistemin üzerine kurulur.
4. **Türkçe birinci sınıf.** `i/İ/ı/I` doğru işlenir; tarih ve sayı biçimleri, TR-Q/TR-F klavyeler ve çeviriler eksiksizdir. Türkçe, sonradan eklenen bir çeviri değildir.
5. **Standartlara bağlılık.** Shell compositor'le yalnızca standart Wayland protokolleri ve freedesktop D-Bus arayüzleri üzerinden konuşur. Compositor değiştirilebilir kalmalıdır; 21 Ekim 2026'daki labwc'den Wayfire'a geçişte shell kodu değişmedi.
6. **Önce çalışsın, sonra yerlileştir.** Gerekirse mevcut bir CLI aracı (swayidle, wlsunset, grim) sarmalanabilir. Ama arayüz sabit kalır ki sonra yerli koda geçmek kolay olsun.
7. **Hız bir özelliktir.** Aşağıdaki performans bütçeleri birer kabul kriteridir.

## Hedef dışı (şimdilik)
- Kendi compositor'ümüz (Wayfire yetmezse yeniden değerlendirilir)
- X11 oturumu (XWayland üzerinden X11 uygulamaları çalışır)
- Tema motoru ve sınırsız özelleştirme: birkaç iyi seçenek, yüzlerce ayardan iyidir
- GNOME Shell eklenti uyumluluğu
- Qt ile uygulama yazmak (Calamares bilinçli bir istisna)

---

## Tasarım dili

**Seçilen yön: Odak.** Kullanıcı kararıyla zarif, temiz, kompakt ve içerik odaklı yerleşim; derinlikli uygulama simgeleri, ince çizgili kontroller ve buzlu cam yüzeyler hedeflenir. Ayrıntılı sözleşme [DESIGN.md](DESIGN.md) içindedir. Tasarım hazırlığı [T0-tasarim.md](../prompts/T0-tasarim.md) ile uygulamadan önce yapılır.

### Renk
- Tüm renkler libadwaita'nın adlandırılmış renklerinden gelir (`@window_bg_color`, `@accent_bg_color`, `@card_bg_color` …). Böylece açık/koyu mod ve vurgu rengi değişiklikleri otomatik yansır.
- **Varsayılan vurgu rengi: `teal` (turkuaz).** libadwaita'nın sistem vurgu paletinden seçildiği için tüm libadwaita uygulamaları da bu rengi izler.
- Varsayılan şema: sistem tercihi; ilk açılışta koyu önerilir.

### Biçim

| Token | Değer |
|---|---|
| Panel yüksekliği | 48 px |
| Açılır pencere köşe yarıçapı | 16 px |
| Kart / döşeme köşe yarıçapı | 12 px |
| Kontrol köşe yarıçapı | 8 px |
| Boşluk ölçeği | 4 · 8 · 12 · 16 · 24 · 32 px |
| Yazı tipi | Inter, 11 pt (arayüz); JetBrains Mono (sabit genişlik) |
| Yüzey kenarlığı | 1 px, `alpha(currentColor, 0.08)` |
| Gölge | Yüzeylerde yumuşak ayrışma; uygulama simgelerinde kontrollü hacim/temas gölgesi |

### Hareket
- Açılma: 180 ms, `ease-out-cubic`, opaklık + 8 px kayma.
- Kapanma: 120 ms, `ease-in-cubic`.
- Durum geçişleri (döşeme açma/kapama, hover): 150 ms.
- `gtk-enable-animations=false` ise tüm animasyonlar kapanır (hareket azaltma).

### Malzeme ve bilinen teknik sınırlar
- **Buzlu cam/gerçek blur hedefi onaylandı.** Panel ve geçici shell yüzeylerinde kullanılır; içerik okunaklılığı, opak erişim/uyumluluk görünümü ve performans bütçeleri korunur. Bu tercih önceki “blur yok” tasarım varsayımının yerini alır.
- **Teknik yol:** blur'u Wayfire 0.9'un Dual Kawase eklentisi çizer (K13). VM ölçümünde boşta ek maliyet görülmedi; gerçek GPU ölçümü açıktır.
- **Pencere animasyonu:** Wayfire `animate` eklentisi pencereleri 180 ms soluklaşmayla açar ve kapatır. Wayfire açılış ve kapanış için ayrı süre sunmaz; 120 ms kapanış bütçesi pencerelerde karşılanmaz. Animasyonlar kapatılınca eklenti yüklenmez.

### Uyarlanabilirlik ve simgeler
- Kendi uygulamalarımız dar/geniş pencere, kısa yükseklik, uzun Türkçe metin ve büyük yazıya uyum sağlar. Temel yerleşim/erişilebilirlik son cila fazına ertelenmez.
- Uygulama simgeleri kontrollü ışık, malzeme ve derinlik taşır; eylem/durum simgeleri tutarlı ve okunaklı çizgi ailesinden gelir. Kesin paketler ve GTK görünüm eşlemesi K14'te seçilir.
- Uygulama simgesinde hacim, kalınlık, yüzey ilişkisi ve malzeme işçiliği gerekir; renkli kutuya çizgi sembolü koymak hedef kaliteyi karşılamaz. İlk dört özgün simge kullanıcı incelemesinden geçer; üretim sözleşmesi [ICONOGRAPHY.md](ICONOGRAPHY.md) içindedir.
- Odak varsayılan düzendir. Ayarlar Merkezi'nden menü tipi ve panelin dört kenarı, hizası ve boyutu değiştirilebilir; farklı düzenler aynı tasarım dilini korur.

### Ses ve dil
- Arayüz metinleri kısa, samimi ve düz Türkçedir ("Wi-Fi'ye bağlan", "Ağ bulunamadı").
- Hata mesajları kullanıcıya ne yapması gerektiğini söyler.
- Başlıkların altında dolgu açıklamalar, ayarın adını yeniden anlatan cümleler ve tanıtım sloganları kullanılmaz. Uygulamanın kendi arayüz metinlerinde em dash yoktur; dış kaynaklı/kullanıcı metni korunur.
- Dekoratif renkli rozetler, gerekçesiz üçlü kartlar ve her sayfaya tekrar eden başlık/alt başlık kalıpları kullanılmaz. Ayrıntılı kurallar DESIGN'in arayüz metni ve düzen bölümündedir.

---

## Performans bütçeleri
Referans makine: 4 çekirdek, 8 GB RAM, entegre GPU, SSD.

| Ölçüt | Bütçe |
|---|---|
| Oturum açılışından panelin görünmesine | < 1,5 sn |
| Başlat menüsünün açılması (tuştan ilk kareye) | < 100 ms |
| Aramada tuşa basıştan sonuca | < 50 ms |
| `asena-shell` boştayken CPU | ~%0 (zamanlayıcıyla yoklama yok, olay güdümlü) |
| `asena-shell` bellek (RSS) | < 150 MB |
| Tüm Asena süreçleri toplam bellek | < 300 MB |

---

## Açık kararlar
Bunlar netleşince bu bölümden çıkarılıp ilgili belgeye işlenir.

- [x] **Kaynak lisansı:** açıkça onaylanan Faz 0 kapsamıyla GPL-3.0-or-later manifest ve LICENSE dosyasına uygulandı. Geçişli bağımlılık ve dağıtım bildirimleri ayrıca denetlenecek.
- [ ] **Alan adı:** D-Bus ve uygulama kimlikleri için `org.asenalinux.*` kullanılıyor. Alan adı alınırsa ona göre güncellenir.
- [ ] **Logo ve marka kimliği**
- [ ] **Depo ve APT barındırma:** GitHub + GitHub Pages veya kendi sunucumuz
- [x] **Tasarım yönü ve hizalama:** Odak; uygulamalar ve başlat solda, durum/saat sağda.
- [ ] **Gerçek blur teknik yolu:** K13; mevcut compositor seçimiyle uyum kararı bekliyor.
- [ ] **Varsayılan GTK/ikon/imleç paketleri:** K14; hedef görünüm Asena Odak.
- [ ] **Zamanlanmış otomatik koyu mod** (gün batımında geçiş)
