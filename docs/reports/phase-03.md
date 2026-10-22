# Faz 3 uygulama raporu

19 Ekim 2026. **Sürüyor; M1 tamamlanmadı.** Kullanıcı elle denemeyi sonraya
bıraktı. Kaynak uygulaması ve yerel otomatik doğrulama devam ediyor.

## Uygulanan parçalar

- GTK içermeyen ortak PropertyModel ve ObjectModel GObject'leri. Başlangıç
  okuması asenkron; özellik ve ObjectManager değişiklikleri olayla izlenir.
  Servis sahibi değişince eski nesneler temizlenir. İşlem kapısı eski cevapları
  eler ve yinelenen işlemleri reddeder. Invalidated özellikler yeniden okunur.
- libpulse-binding + libpulse-glib-binding ile ayrı GLib iş parçacığında ses
  istemcisi. Çıkış/giriş, seviye, sessizlik, mikrofon ve çıkış seçimi; %100 sınırı.
  Dosya izleyicileriyle socket yeniden oluşumuna bağlanma, normalde yoklama yok.
- Başarı, yalnız libpulse isteğinin onayıyla verilmez. İstenen değer servisin
  sonraki durumunda gözlenmeden CLI/OSD başarı bildirmez; sonlu süre aşımı vardır.
- Worker'da sysfs arka ışık keşfi/okuma; yazma yalnız logind SetBrightness.
  Alt sınır %5 veya bir ham adım. `/sys` izinleri değiştirilmez.
- UPower pil ve power-profiles-daemon profilleri; VM'de pil yoksa alan gizlenir,
  sunulmayan performans profili gösterilmez.
- Gerçek ses/mikrofon ve parlaklık CLI sözleşmeleri; OSD onaylanmış değer alır.
  Uzun servis istekleri menü IPC kuyruğunu bekletmez.
- Hızlı Ayarlar: ses, çıkış seçimi, mikrofon, desteklenirse parlaklık, pil/profil,
  Rahatsız etme; ortak güç menüsü. Ayarlar ve ekran görüntüsü henüz devre dışı.
- NetworkManager nesne modeli, kararlı Wi-Fi kimlikleri, açık/kişisel ağ isteği,
  yalnız ayrıntı açma/yenilemede tarama. BlueZ adaptör ve eşleşmiş cihaz modeli,
  aç/kapa ve Connect/Disconnect istekleri. Donanım yoksa ilgili alan gizlenir.
- MPRIS oynatıcı keşfi, oynayan/son kullanılan seçimi ve yetenek denetimli medya
  eylemleri. StatusNotifierWatcher/host, ARGB pixmap, native eylemler ve canlı
  dbusmenu. Mevcut watcher devralınmaz.
- AccountsService kullanıcı adı/avatar yolu ve güvenli kullanıcı adı geri dönüşü.
  Ayrı daemon görünüm aktarımı, yalnız Asena dizininde labwc config üretimi ve
  tek wlsunset çocuğunun durdurulup beklenerek yönetilmesi. İç içe oturumda sistem
  görünümüne ve gamma'ya yazılmaz. Gece ışığı yalnız backend çıktısında uygulama
  gözlenince etkin bildirilir.

## Doğrulanmış sonuçlar

- Debian 13: libpulse 17.0, PipeWire/Pulse 1.4.2, WirePlumber 0.5.8, UPower
  1.90.9, power-profiles-daemon 0.30, NetworkManager 1.52.1, BlueZ 5.82.
- PPD ve UPower gerçek D-Bus introspection'ı incelendi. PPD VM'de balanced ve
  power-saver sunuyor; UPower IsPresent=false bildiriyor.
- PropertyModel özel D-Bus deneyi geçti: eksik servis, geliş, özellik değişimi,
  kayıp, eski durumun temizlenmesi ve yeniden bağlanma.
- ObjectManager özel D-Bus deneyi geçti: başlangıç verisi, property delta,
  nesne ekleme/silme ve sahibin kaybolmasında temizleme.
- Sanal null çıkış + remap giriş ile gerçek libpulse deneyi geçti: %30, +5 yüzde
  puanı, sessize al/aç, mikrofon ve geçersiz sayı reddi.
- Son Faz 3 revizyonunda genel Linux kalite kapısı **34 Rust + 8 Python** testiyle
  geçti: build, Clippy `-D warnings`, fmt, veri, ShellCheck ve çeviri kontrolleri.
  [Kalite günlüğü](evidence/phase-03/quality.log).
- Gerçek Pulse sunucusunu durdur/başlat ve socket olayından yeniden bağlanma
  deneyi geçti. MPRIS iki fixture seçimi/eylemi/kaybı; SNI iki fixture istemcisi,
  dbusmenu/eylem/kayıp ve mevcut watcher çakışması deneyleri geçti. Bunlar iki
  bağımsız gerçek uygulama ile kabulün yerine geçmez.
- İç içe labwc'de CLI ses/mikrofon sonuçları `pactl` üzerinden gözlendi;
  olmayan arka ışık hata verdi, daemon gamma'yı kullanılamaz bildirdi, koyu/açık
  geçişleri ve süreç/manager ortamı temizliği geçti.
  [Deney günlüğü](evidence/phase-03/gui.log). Kaydırma alanının genişlememesi
  düzeltildi; yeni VM deneyi ve koyu görünüm görüntüsü incelendi. Ses, mikrofon,
  görünüm, bildirim ve güç satırları erişiliyor. Bütün panel konumları ve büyük
  yazı kabulü ayrıca açık.

Sanal PipeWire null aygıt SUSPENDED iken `pactl` dahil kontrol isteklerini kabul
edip değerini değiştirmedi. Test, yalnız kendi null aygıtlarına sessiz playback ve
record akışı ekleyerek çalışır; gerçek mikrofon/hoparlör kullanılmaz.

## VM sınırları ve açık işler

VM servis kurulumu `scripts/setup-vm-services.sh` ile açık test bayrağı gerektirir.
Cloud VM'nin SSH bağlantısını korumak için bütün VM arayüzleri NetworkManager'da
unmanaged bırakıldı; mevcut ağ yöneticisi değişmedi. Bu ayar ürün paketine girmez.

- Wi-Fi/BlueZ gerçek donanım ve yanlış parola/yeniden deneme kabulü yapılmadı.
  İlk Wi-Fi isteği NetworkManager'a **volatile**, autoconnect=false gider; parola
  log/GSettings'e yazılmaz. Kalıcı profil/secret-agent akışı henüz kapanmış değil.
  Mevcut bir SecretAgent adı devralınmadı, yeni ajan kaydedilmedi.
- Gerçek arka ışık, pil, profil yetkileri ve harici donanım denenmedi.
- Gerçek etkin klavye grubunun kaynağı açık; tercih değeri etkin düzen diye
  gösterilmiyor. AccountsService fotoğrafı, portalların görünüm aktarımı,
  donanım gamma/gece ışığı ve canlı labwc tema/reconfigure kabulü açık.
- Tepsi özel simge yolu/overlay, gerçek uygulamalar ve well-known adın bağlantı
  kopmadan bırakılması gibi uç durumlar açık. Ağ cihazı işlem sırasında
  kaybolduğunda iptal ve Bluetooth/profil satırlarında odak kararlılığı izlenecek.
- Hızlı Ayarlar'ın dar/kısa alan, büyük yazı, odak ve tüm panel kenarları kabulü
  açık. Bluetooth satırları ve profil satırlarının kararlı odak davranışı incelenecek.
- Faz 4 için yalnız native C 1.3.0 backport onaylandı. Bu onay güvenli kilit/uyku
  kabulü değildir; kilit/uyku düğmeleri güvenlik deneyi geçene kadar kapalıdır.

## Sonraki adaylar

Gerçek ağ/BT donanımı, etkin klavye kaynağı, portal/tema aktarımı ve görsel kabul.
Bütün kabul kapıları geçmeden Faz 3 veya M1 tamamlandı işaretlenmez.
