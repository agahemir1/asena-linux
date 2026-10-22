# Faz 7 — Ekran görüntüsü, erişilebilirlik ve 1.0 hazırlığı

## Bağlam
Önce oku: `CLAUDE.md`, `docs/PLANNING.md`, `docs/VISION.md` (tasarım ve performans bütçeleri), `docs/COMPONENTS.md` §2.14–2.15, `docs/ARCHITECTURE.md` ve Faz 0–6 kabul raporları.

Faz 6 kurulabilir sürüm adayı üretir. Bu faz eksik ekran görüntüsü akışını tamamlar; klavye erişimi, Türkçe, kaynak tüketimi ve hata toparlanmasını ölçerek 1.0 kapısını değerlendirir.

## Hedef
Kurulu Asena'da ekran görüntüsü alınır; temel masaüstü klavye ve yardımcı teknolojilerle kullanılabilir. Tasarım ve Türkçe tutarlıdır, performans bütçeleri ölçülmüştür. Kabul edilmeyen açık engeller varsa 1.0 ilan edilmez.

## Ön koşullar
- M3 VM kurulum ve güncelleme kanıtları hazır olmalı.
- Wayfire'ın gerçek yakalama protokolleri, pencere geometrisi ve portal yetenekleri Faz 0/6 envanterinden okunur. Pencere yakalamayı sağlayan veri kaynağı yoksa özellik uygulanmış varsayılmaz.
- Pano geçmişi `docs/COMPONENTS.md`'de **Sonra** kapsamındadır. Bu fazda pano kapsamı ekran görüntüsünü panoya kopyalamaktır; geçmiş özelliği ayrı kullanıcı kararı olmadan 1.0 koşulu yapılmaz.

## Yapılacaklar

### A. Ekran görüntüsü (7A)
1. `asena-screenshot` libadwaita uygulaması ve `asenactl screenshot area|window|screen` entegrasyonu yap. Hızlı Ayarlar kısayolunu, Print ve uygun alan kısayolunu bağla; Faz 3'teki geçici devre dışı durumları kaldır.
2. MVP'de `grim` ve `slurp` asenkron çocuk süreç olarak yönetilir. Alan seçimini iptal etmek hata bildirimi veya boş dosya üretmez. Argümanlar ayrı verilir; dosya yolu/metaveriden komut oluşturulmaz. Süreç bitişleri ve iptaller kaynak bırakmaz.
3. Tüm ekran/alan için çoklu monitör, negatif mantıksal koordinat, HiDPI ve döndürmeyi işle. Fiziksel piksel ile mantıksal alan dönüşümünü tek yerde tut.
4. **Pencere modu:** desteklenen protokolle gerçek pencere geometrisi/kimliği elde edilmesi için önce kanıt çıkar. Foreign-toplevel listesinin tek başına geometri sağlaması varsayılmaz. Gereken kaynak yoksa pencere modu açıklayıcı biçimde kullanılamaz olur; serbest alan seçimi pencere yakalama diye etiketlenmez. Bu açık madde giderilmeden üç modun tamamı kabul edilmiş sayılmaz.
5. İsteğe bağlı gecikmeli çekimde sayaç/iptal sağla; çekim arayüzü kareye girmesin. Kilitli oturumdan çekim yapılmaz; greeter/başka kullanıcı oturumuna erişim eklenmez.
6. Görüntüyü XDG Pictures altında yerelleştirilmiş ekran görüntüsü alt dizinine, çakışmayan adla kaydet. Yazma izni veya disk dolu hatasında kullanıcıya anlaşılır çözüm sun. Görüntüyü panoya kopyala; Wayland'da uygulama kapandıktan sonraki pano sahipliği davranışını ölç ve gerekirse kontrollü bir sahip süreç tasarla.
7. Başarı bildiriminde Aç ve Klasörü göster eylemleri standart URI açma yolunu kullanır. Başka uygulamaların bildirim içeriklerinden komut çalıştırılmaz. Ekran görüntüsü paketini Faz 6 metapaketine/ISO'ya ekle.

### B. Erişilebilirlik ve etkileşim (7B)
8. Panel, menü, bildirim merkezi, Hızlı Ayarlar, settings, greeter ve lock için klavye gezinme haritası çıkar. Odak görünürlüğü, doğru rol/ad, Enter/Space/Esc ve açılır yüzey kapanınca odak iadesini düzelt.
9. AT-SPI/Orca ile gerçek Wayland oturumunda temel akışları dene. Katman yüzeyleri, cage greeter ve kilit bağlamındaki erişim sınırlarını ayrı raporla; yalnız erişilebilir ad verilmesini tam ekran okuyucu desteği diye kabul etme. Yardımcı teknolojilerin kilidi aşmasına izin verilmez.
10. %100/%200 ölçek, büyük metin, açık/koyu ve mevcut yüksek kontrast tercihlerinde kesilme/taşma denetimi yap. Hareket azaltma tercihinde zorunlu olmayan animasyonları kapat; işlem/bildirim durumunu yalnız renkle anlatma.
11. Türkçe arama, Türkçe alfabetik sıralama, tarih/sayı biçimi, TR-Q/TR-F ve İngilizce geri dönüşü dene. `po/tr.po`, çoğullar, mnemonic ve çevrilemeyen sabitler için denetim yap. Kullanıcıya görünen hata metinleri eylem önerir.
12. Yüzeylerin köşe, boşluk, tipografi, hareket ve simge işçiliğini VISION ile `docs/DESIGN.md` Odak sözleşmesine göre karşılaştır. Onaylı gerçek blur hedefini K13'te seçilen teknik yolla doğrula; opak erişim modu ayrıca denensin. Compositor'ın sağlamadığı pencere animasyonunu varmış gibi sunma.
   - Bütün kendi arayüz metinlerinde dolgu açıklama, tekrarlanan başlık/alt başlık, tanıtım sloganı ve em dash denetimi yap. Dekoratif renkli rozetleri ve içerikle gerekçelendirilemeyen üçlü kart şablonlarını kaldır; gerçek hata/sonuç açıklamaları ve kullanıcı/dış uygulama içerikleri korunur.

### C. Performans ve dayanıklılık (7C)
13. Referans koşullarını yaz: 4 çekirdek, 8 GB RAM, entegre GPU, SSD; dağıtım/çekirdek/derleme türü ve ekran düzeni de raporda olsun. Ana ölçümler kurulu gerçek oturumda release derlemesiyle yapılır; VM/iç içe sonuçları ayrı tutulur.
14. Panel için oturum başlatma → ilk görünür kare; menü için kısayol → ilk kare; arama için tuş → güncel sonuç ölçüm noktalarını tanımla. 180 ms açılış animasyonunun tamamlanması ile <100 ms ilk kare hedefini ayır. Aynı koşulda en az 20 tekrar için medyan, p95 ve en kötü değeri ver; soğuk ve sıcak açılışı karıştırma.
15. VISION hedeflerini kabul sınırı olarak uygula: panel <1,5 sn, menü <100 ms, arama <50 ms. Tekrarlı etkileşimlerde p95 sınırı geçmeli; soğuk başlangıç ayrıca raporlanır. Başarısızlıkta hedefi düşürmeden darboğazı ölçerek düzelt.
16. Boşta CPU'yu 60 sn oturma sonrası en az 5 dk izle; sürekli yoklama veya yoğun uyanma varsa kaynağını kaldır. `~%0` için araç, zaman aralığı ve gözlenen değer raporlanır; saat güncellemesi gibi beklenen olaylar ayrıca yazılır.
17. `asena-shell` RSS <150 MB, Asena süreçlerinin toplam RSS'i <300 MB koşulunu tanımlı boşta senaryoda ölç. Toplama dahil süreçleri listele; compositor ve harici sistem servislerini ayrı ver. 100 menü aç/kapat, bildirim ekle/temizle ve monitör döngüsünden sonra sürekli büyüme olup olmadığını incele.
18. Shell/daemon yeniden başlatma, servis kesilmesi, eksik dosya, bozuk bildirim verisi, ağ kaybı ve monitör değişimi regresyonlarını uygula. Açık güvenlik/veri kaybı hataları sürüm engelidir.

### D. Sürüm adayı ve kapanış (7D)
19. Yeni paket/ISO sürümünü Faz 6 süreciyle üret ve kurulum/yükseltme smoke kontrollerini tekrar yap. Tek başına geliştirme ağacındaki başarı paketlenmiş sürümün kanıtı sayılmaz.
20. 1.0 sürüm notlarını, desteklenen/denenmeyen donanım matrisini, bilinen sınırları ve hata bildirme yönergelerini yaz. Ekran görüntüleri ve günlüklerde kişisel verileri temizle; varsayılan telemetri ekleme.
21. Kapanışta her fazın kabul raporunu bağla. Kritik/güvenlik/veri kaybı engeli varken veya temel performans/erişilebilirlik koşulları kanıtlanmamışken M4 tamamlanmaz. Dış yayın ve sürüm etiketi ayrı yayımlama kararıdır.

## Uygulama sırası
`7A → 7B → 7C → 7D`. Ölçümler öncesi ilgili özellikler tamamlanır; paketlenmiş son çıktı tekrar kontrol edilir.

## Kapsam dışı
- Pano geçmişi, ekran kaydı, görüntü düzenleme/işaretleme, GNOME Shell eklentileri.
- Yerli screencopy istemcisi, kendi compositor'ümüz ve yeni varsayılan uygulamalar.
- Bu faza eklenmesi istenirse pano geçmişi ayrı prompt ve kapsam onayıyla planlanır; parola kaynağı işaretlerinin güvenilirliği ayrıca değerlendirilir.

## Kabul kriterleri
- [ ] Odak'ın derinlikli uygulama/zarif çizgi simgeleri, açık/koyu malzemesi, gerçek blur ve opak erişim modu paketlenmiş çıktıda incelenmiştir; blur açık/kapalı kaynak ölçümleri rapordadır.
- [ ] Alan ve tüm ekran çekimi, gecikme/iptal, dosyaya kaydetme, pano ve bildirim eylemleri çalışır.
- [ ] Pencere modu gerçek veri kaynağıyla doğrulanmıştır; destek yoksa açık sürüm engeli/karar kaydı vardır ve özellik varmış gibi sunulmaz.
- [ ] Monitör ölçeği, negatif koordinat, döndürme, yazma hatası ve disk dolu senaryoları raporlanmıştır.
- [ ] Temel oturum/menü/ayar/bildirim akışları klavyeyle tamamlanır; ekran okuyucu deneyleri bağlam bazında kanıtlıdır.
- [ ] Türkçe, İngilizce, büyük metin, koyu/açık, yüksek kontrast ve hareket azaltma denetimleri geçer.
- [ ] Panel, menü ve arama süreleri tanımlı ölçüm koşullarında bütçeyi karşılar; ham sonuçların özeti rapordadır.
- [ ] CPU ve RSS bütçeleri ölçülmüş; tekrarlı kullanımda açıklanamayan sürekli kaynak artışı yoktur.
- [ ] Servis/shell çökmesi ve monitör regresyonları geçer; kilit güvenliği bozulmamıştır.
- [ ] Yeni .deb/ISO ile temiz kurulum ve önceki sürümden yükseltme doğrulanmıştır.
- [ ] Build, clippy, fmt, test, paket kontrolleri ve çeviri kontrolleri geçer; M4 için açık engel kalmamıştır.

## Doğrulama
- Performans için release sürümü ve ölçüm notları; arayüz için ekran görüntüsü/video ve erişilebilirlik raporu; güvenlik için Faz 4 regresyon matrisi kullan.
- Pano kopyasını kaynak uygulama açık/kapalıyken dene; uygulama ömrü ve kaynak sahipliğini raporla.
- `docs/reports/` altında donanım, senaryo, beklenen/gerçek sonuç, ölçüm ve açık sorun referanslarıyla kapanış raporu tut.

## Teslim
Uygulama onayından sonra alt bölüm planını sun. Bitince ekran görüntüsü örnekleri, erişilebilirlik ve performans raporu, paket/ISO referansı, sürüm engelleri ve sonraki adaylar verilir. ROADMAP'te M4/1.0 yalnız tüm geçiş koşulları sağlanınca tamamlanır.
