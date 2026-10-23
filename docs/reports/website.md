# Web sitesi raporu

23 Ekim 2026. Karar K29 ([PLANNING](../PLANNING.md)), bağımlılıklar [DEPENDENCIES](../DEPENDENCIES.md),
geliştirme ve ekran görüntüsü akışı [website/README.md](../../website/README.md).

## İstek

Kullanıcı; Asena masaüstünü, uygulamalarını, adın Türk mitolojisindeki yerini ve son kullanıcıya
getirdiklerini anlatan, gerçek ekran görüntüleri ve artwork kullanan, doğal Türkçe metinli, etkileşimli ve
animasyonlu, belge ve haber de barındıran, Türkçe ve İngilizce bir site istedi. Ekran görüntülerinin VM'den
yeni çekilmesini ve Inter'in siteyle sunulmasını seçti.

## Yapılanlar

- **Yığın:** `website/` altında Astro 7.4 (statik), Pagefind araması, Fontsource yazı tipleri, sitemap ve
  dil başına RSS. Netlify derlemesi depo kökündeki `netlify.toml` ile; güvenlik başlıkları ve karma adlı
  varlıklara uzun önbellek. Çerez, analiz ya da üçüncü taraf isteği yok.
- **Sayfalar (iki dilde):** ana sayfa, Masaüstü, Uygulamalar, Hikâye, Neden Asena, İndir, Belgeler (8),
  Haberler (6), 404. Toplam 45 sayfa.
- **Etkileşimler:** logonun çizilerek belirdiği giriş; kaydırdıkça menü, denetim merkezi ve bildirim
  panelinin açıldığı masaüstü sahnesi; panelden tıklanan ve yüzeyleri yalnız kendi alanında VISION
  süreleriyle (180/120 ms) açan masaüstü turu; menü tipi, açık/koyu ve panel konumu değiştiricileri;
  cam/opak karşılaştırma kaydırıcısı; siteyi de boyayan dokuz vurgulu seçici (klasör simgeleri
  `data/icons/Asena-<vurgu>`'dan); `fold` karşılığıyla Türkçe arama denemesi; duvar kâğıtlarının
  kaydırmayla geçtiği hikâye bölümleri ve büyütülebilir galeri; GRUB, Plymouth ve satır satır yazılan
  gerçek fastfetch çıktısı. `prefers-reduced-motion` açıkken zorunlu olmayan hareket kalkar.
- **İçerik:** bütün iddialar depodaki belgelerden ve koddan alındı; çalışan şey "çalışıyor", karar ve
  hedefler öyle yazıldı. Belge metinleri yazılırken koddan doğrulandı (Ayarlar › Masaüstü ve panel
  seçenekleri, menünün klavye davranışı, telemetri olmadığı). Hikâye sayfasındaki tarih bilgileri web
  kaynaklarıyla doğrulandı ve sayfada kaynak verildi: Zhou shu'daki iki kurt anlatısı ve kurdun adının
  geçmemesi, Ergenekon'un Reşîdüddin ve Ebülgazi'deki yeri, bozkurt ayrıntısının yirminci yüzyıl
  anlatımlarından geldiği, ebrunun 2014 UNESCO kaydı, Konya Alaeddin minberi (1155) ve kündekârî, İznik
  paleti. Söylence kültür mirası olarak anlatıldı; siyasi sembol kullanılmadı.
- **Tek kaynak:** logo, duvar kâğıtları, uygulama ve klasör simgeleri, çizgi simgeler ve açılış kanıtları
  `data/` ile `docs/reports/evidence/` altından okunur. Sitenin kendi varlığı yalnız ekran görüntüleridir
  (70 WebP, 9 MB).

## Ekran görüntüleri

`website/scripts/capture-screens.sh` VM'de Asena oturumunu başsız Wayfire ile, ayrı bir gösterim ev
dizini ve 3840×2160 / ölçek 2 çıktıyla açar; `take-shots.sh` 35 çekimlik listeyi alır (masaüstü
yüzeyleri, üç menü tipi, cam ve opak, açık ve koyu, dokuz vurgu, dört panel kenarı, on iki uygulama).
`convert-screens.mjs` uygulama ve vurgu çekimlerini masaüstüyle farkından pencereye göre kırpar.
Türkçe ve İngilizce setler ayrı çekildi. Gösterim içeriği gerçektir: Resimler'de duvar kâğıtları, Belgeler'de
proje belgeleri, iki not, iki takvim girdisi (29 Ekim Cumhuriyet Bayramı dahil).

Yol üstünde:

- `tr_TR` yerel ayarında bash'in `[a-z]` aralığı `i` harfini içermiyor; betikler denetimi `C.UTF-8`'de yapar.
- Daemon `wayfire.ini`'yi yeniden yazınca Wayfire başsız çıkışı ölçek 1'e döndürüyor; `shot` her çekimden
  önce ölçeği denetler.
- `dev-session.sh` her açılışta `cargo build` çalıştırıyor ve VM'deki başka bir derlemenin kilidini
  bekliyordu; betik derlenmiş `asena-session`'ı doğrudan başlatır.
- VM'deki depo kopyasında ve `/usr/share/backgrounds/asena` içinde 22 Ekim'de değiştirilen eski duvar
  kâğıtları duruyordu. Depoda olmayan dört eski dosya (crimson-crescent, gokturk-tamga, seljuk-star,
  turquoise-flow) her iki yerden silindi ve yeni on duvar kâğıdı sistem dizinine kuruldu; Ayarlar'ın
  duvar kâğıdı listesi o dizini okuyor.
- VM diski (32 GB) çekim sırasında %100 doldu (`target/debug` 21 GB, aynı anda çalışan önyükleme testi
  `target/boot-test` altında). Ortak derleme dizinine dokunulmadı; çekim çalışma dizini tmpfs'teki `/tmp`'ye
  taşındı. Disk doluyken dconf yazamadığı için `gsettings` değişiklikleri sessizce uygulanmıyor.
- Çekimlerden sonra VM kapalı bulundu. VM diskinde üç sembolik bağ kaldı: `/home/kullanici` ve
  `/home/user` (gösterim ev dizinlerine kısa yol, bildirimlerde geliştirme yolu görünmesin diye) ve
  `~/asena/target/site-capture` (`/tmp`'ye). Hedefleri tmpfs'te olduğu için artık boşa bakıyorlar;
  `capture-screens.sh` bir sonraki çekimde ilk ikisini yeniden kurar, üçüncüsü elle silinebilir.

## Doğrulama

| Denetim | Sonuç |
|---|---|
| `npm run build` | 45 sayfa, hata yok; Pagefind 2 dil, 44 sayfa |
| `npm run check` | 0 hata, 0 uyarı |
| `npm test` | `fold` için 4 test geçti; örnekler `crates/asena-core/src/text.rs` testleriyle aynı |
| İç bağlantı denetimi | 45 sayfada 2345 iç bağlantı ve varlık çözülüyor; her sayfanın diğer dilde karşılığı var; sayfalarda em dash yok |
| Yatay taşma | 375 ve 768 piksel genişlikte 19 sayfada ekran dışına taşan metin ya da kaydırma yok |
| Etkileşim | Türkçe arama denemesi ("indir", "OYNATICI", "ARSIV" yalnız Asena aramasında bulunuyor), tema düğmesi (sistem, açık, koyu), vurgu seçici (site vurgusu ve görüntüler değişiyor), masaüstü turu, uygulama seçici ve `#adres` ile açılış, site araması ("guncelleme" sorgusu Türkçe sonuç veriyor); konsolda hata yok |
| Ağırlık | Ana sayfa ilk yüklemesi 1x ekranda yaklaşık 390 KB, 2x ekranda 560 KB (HTML, CSS, Inter latin ve latin-ext, ilk görünen görseller) |

Tarayıcı denemeleri uygulamanın tarayıcı panelinde (Chromium) `npm run preview` ile yapıldı. Panel gizliyken
animasyonlar duraklatıldığı için animasyonlu durumlar ekran görüntüsüyle değil DOM ölçümüyle denetlendi.

## Doğrulanmadı

- Netlify'a yayın yapılmadı; `netlify.toml` ile gerçek derleme ve başlıklar (CSP dahil) Netlify'da denenmedi.
- Safari, Firefox ve gerçek telefonlar denenmedi. Kaydırmaya bağlı CSS animasyonları ve sayfalar arası
  View Transitions desteklemeyen tarayıcıda devre dışı kalır; içerik etkilenmez, bu da denenmedi.
- Ekran görüntüleri VM'de yazılımla çizimle (llvmpipe) alındı; arayüzün görsel kabulü ve hacimli
  uygulama simgelerinin onayı hâlâ açık. Görüntüler arayüz değişince yeniden çekilmeli.
- Lighthouse ya da ekran okuyucuyla erişilebilirlik denetimi yapılmadı; klavye odağı ve ARIA rolleri kod
  düzeyinde yazıldı.

## Sonraki adaylar

- **Panel kenarı değişince menü yerinde kalıyor:** oturum açıkken panel üste, sola ya da sağa alındığında
  başlat menüsünün arama alanı doğru kenara geçiyor ama menü sol altta açılıyor (VM, 23 Ekim). DESIGN
  menünün panelin ekrana bakan kenarına bağlanmasını istiyor. Site bu yüzden panel konumlarını menüsüz
  gösteriyor.
- **Yazılım oturum açılışında kendiliğinden açıldı:** çekim oturumunda daemon'un güncelleme bildirimi
  tıklanmadan `asena-store --updates` başladı. Bildirim açılır penceresinin varsayılan eylemi bir
  tıklama olmadan tetikleniyor olabilir; neden ayrıca araştırılmalı.
- **Foot Client ve Foot Server menüde:** `foot` paketi üç masaüstü girdisi kurar; ISO'da da görünecekler.
- **Daemon `wayfire.ini` yazınca çıkış ölçeği:** başsız çıkışta ölçek 1'e döndü. Daemon'un kayıtlı ekran
  düzenini bu yeniden yüklemeden sonra yeniden uygulayıp uygulamadığı gerçek bir ekranda denenmeli.
- **VM disk alanı:** 32 GB'lık VM'de `target/debug` 21 GB. Önyükleme ve paket testleriyle birlikte disk
  doluyor.
- Sitenin Netlify'a bağlanması ve ilk ISO çıktığında İndir sayfasının gerçek indirme bağlantısı ve
  SHA-256 özetiyle güncellenmesi.
