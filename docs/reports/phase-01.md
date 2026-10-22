# Faz 1 uygulama ve kabul raporu

19 Ekim 2026. Durum: **🟨 sürüyor**. Kullanıcı tüm fazların sıralı uygulamasını
onayladı; dış CI yerine yerel Linux kontrollerini seçti. Görsel/blur ve normal
oturum kabulü tamamlanmış sayılmaz.

## Kaynak uygulaması

- Ayrı thread/bağlantıda foreign-toplevel istemcisi; kanalla çift yönlü istekler,
  atomik `done`, kapatmada handle temizliği, eksik protokolde boş model.
- GTK içermeyen Toplevel GObject ve gerçek `gio::ListModel`; güncellemede nesne
  kimliği korunur, bağlantı kaybında eski pencereler temizlenir.
- GIO uygulama indeksi; tam desktop ID, StartupWMClass, Türkçe `fold` ve ters-DNS
  son parça eşleştirmesi. AppInfoMonitor ile worker'da yeniden indeksleme.
  Oturum Flatpak export dizinlerini XDG arama yoluna ekler.
- Sabitlenmişler + açılma sırasındaki uygulama grupları; aktif durum, bir/iki nokta,
  tek tık odak/küçült, pencere listesi, orta tık başlatma, sağ tık sabitle/yeni/kapat.
  Çoklu kapatmada onay. Dar alanda ayrı taşma düğmesi; durum/saat alanı korunur.
- GDK launch context token'ı kullanılır; dosya yükleme ve gerçek başlatma worker'da.
- Dakika sınırına hizalanan saat, saniye tercihi ve yerel kısa tarih tooltip'i.
- Shell zbus sunucusu ve ortak proxy sözleşmesi; `asenactl` clap grameri. Menü,
  hızlı ayarlar ve bildirim merkezi bu fazda yalnız komut/log yer tutucusudur.
  OSD girdisi doğrulanır; görsel OSD Faz 2'dir. Kilit/ses/screenshot uygulanmış
  gibi davranmaz; henüz desteklenmeyen komutlar hata kodu verir.
- Monitör başına arka plan, zoom/fit/center/solid, worker ölçekleme, 300 ms
  crossfade, eski isteği geçersiz kılma, dosya/piksel sınırları ve eksik dosyada
  tema rengi. CSS geliştirme oturumunda asenkron yeniden yüklenir.
- Super+Space, Super+A/N, XF86 ve Print komut eşlemeleri. Varsayılan compositor
  ve üretim CSS malzemesi değişmedi. Nihai hacimli simgeler üretilmedi.

## Gerçek Linux sonuçları

Debian 13.7 VM, Rust 1.85.1, GTK 4.18.6, libadwaita 1.7.6,
gtk4-layer-shell 1.0.4, labwc 0.8.3. Windows üzerinde Rust derlenmedi.

| Kontrol | Sonuç |
|---|---|
| build / clippy all-targets `-D warnings` / fmt | Geçti |
| Birim testleri | Geçti: 17 Rust, 8 Python; model/eşleştirme/sıralama/saat/arka plan/CLI/oturum/gettext |
| ShellCheck, XML, desktop, gettext ve Türkçe katalog | Geçti |
| Gerçek toplevel küçült/geri odakla/kapat | Geçti; ayrı `asena-probe` penceresinde protokol olayları kaydedildi |
| Gerçek görev çubuğu sol tıklaması | Geçti; wlrctl sanal pointer tıklaması ve compositor durum sorgusuyla doğrulandı |
| Nautilus/Ptyxis/Firefox eşleştirmesi | Üç uygulama doğru simgelerle listelendi. Ptyxis user-scope bus hatası ayrıca araştırılıyor; çalışan terminal kabulü verilmedi |
| D-Bus toggle ve CSS reload | Geçti; journal'da komutlar görüldü, geçersiz OSD reddedildi |
| Eksik arka plan dosyası | Shell çökmedi; tema rengine döndü. Üç ölçekleme modu gerçek SVG çözümüyle birim testinde geçti |
| İki sanal çıkış ve %200 ölçek | Panel ve arka planlar çalıştı; fiziksel hotplug kanıtı değil |
| Süreç/ortam temizliği | İlk bağımsız smoke geçti. Önizleme ile eşzamanlı son test yanlış shell PID'sini seçti; runtime'a göre PID seçimi düzeltildi, tekrar koşusu bekliyor |
| systemd kill-9 yeniden başlatma | Henüz denenmedi; kaynak unit `Restart=on-failure`, `RestartSec=1` içeriyor |
| Görsel kabul, klavye/popover, büyük yazı, dört kenar | Tam matris denenmedi; canlı kenar tercih arayüzü Faz 5 |
| Gerçek blur | Tamamlanmadı; kullanıcı yalnız yalıtılmış VM Wayfire deneyini seçti |

Smoke betiği: `scripts/smoke-session.sh`. Kanıtlar konuğun
`target/evidence/phase1-quality.log` ve `target/evidence/session-smoke-*`
dizinlerindedir. Seçilen nihai kayıtlar bu raporun yanına eklenecek.

## Ölçümler ve bulunan sorunlar

30 saniyelik debug VM gözlemleri: varsayılan GTK renderer ile %0,533 CPU /
257872 kB RSS; cairo ile üç uygulamalı denemede %0,333 CPU / 90324 kB RSS.
Koşullar aynı değildir; bu değerler karşılaştırmalı benchmark veya gerçek GPU
performans kabulü değildir. Varsayılan renderer RSS bütçesini aşmıştır. Release,
oturma süresi ve gerçek GPU deneyi açıktır; hedef sessizce düşürülmedi.

İlk görüntülerde çizgi simgeleri eksikti: no-recommends kurulumda
`librsvg2-common` yoktu. Paket kurulum listesine eklendi; sonraki görüntüde
başlat/durum simgeleri düzeldi. Ptyxis ayrı private runtime'da systemd user-bus
yolunu bulamadı. Headless test/önizleme runtime'ına gerçek VM user-bus socket'i
bağlandı; session bus yine özel, user-manager ortamı aktarılmıyor. Tekrar deney
sonucu ayrıca yazılacak.

## Açık kabul ve sonraki faz

Kalıcı sabitlemenin yeniden başlatma testi; orta/sağ tık, çoklu pencere ve taşma
menüsü; gerçek klavye erişimi; systemd yeniden başlatma; normal oturum; tema
renklerinin Asena/Asena-Dark pencere kenarlarına uygulanması; K13 blur ve K14
nihai simgeler/görsel matris açıktır. Faz 1 ✅ değildir. Menü/bildirim kaynaklarına
geçişte aynı modeller ve IPC kullanılacak; görsel veya güvenlik kapıları yazılmış
kodla tamamlandı sayılmayacak.
