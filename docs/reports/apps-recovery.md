# Uygulamalar ve sıfırlama devamı (23–24 Ekim 2026)

Kaynak oturum: Claude `059b1f01-e91b-4ea5-84d5-e042b896d9cc`, başlık
“Asena Linux uygulamaları ve sıfırlama özelliği”. Oturum sınırında Yazıcılar
panelinden sonra kalmıştı. Mevcut değişiklikler korunarak devam edildi.

## Kaynak kapsamı

- Sistem İzleyici, Yedekleme, Diskler; gerçek Uçbirim/Kamera/Belgeler/Hava Durumu.
- Ayarlar › Uygulamalar, Yazıcılar, Kurtarma; toplam 20 panel.
- Yazılım `.deb`/`.flatpakref` dosya açma. Yerel Debian paketi kök servise dosya
  tanıtıcısıyla geçer, normal dosya/boyut sınırı ve polkit kontrolünden sonra
  salt kullanıcı erişimine kapalı önbelleğe kopyalanır; kurulum planı onaylatılır.
- Kurtarma D-Bus servisi, çevrimdışı pencere/işçi, GRUB girdisi, kurulum sonu
  imaj kopyalayıcı ve dağıtım dosyaları. [Sözleşme](../architecture/RECOVERY.md).
- Dört uygulamanın menü gizlemesi kaldırıldı, yeni üçünün girdisi/hacimli simgesi
  eklendi. Uçbirim varsayılan seçim oldu, başlatıldığı dizini yeni pencereye taşır.
- Yeni bağımlılıklar geliştirme ve ISO listesine işlendi; yeni metinler Türkçeleştirildi.

İncelemede ayrıca PDF render'ının stdout hedefi, Flatpak izin dosyasının diğer
bölümlerini koruma, salt okunur `home:ro` iznini kaldırma, yedekleme stderr
borusunun dolması, yedek hedefinin kaynak içinde olması, sistem diski koruması
ve GRUB tek seferlik kurtarma girdisinin Secure Boot seçicisi tarafından
ezilmesi düzeltildi.

## Deney koşulları ve sonuçlar

Debian 13 geliştirme VM'si, rustc/cargo 1.85.1, GTK 4.18, libadwaita 1.7,
onaylı gtk4-layer-shell 1.3.0 yerel prefix'i kullanılır. Windows'ta derleme yok.

| Kontrol | Durum | Kanıt/sınır |
|---|---|---|
| Kaynak/şema/çeviri listesi | Geçti | 34 crate, 199 Rust dosyası; XML/POTFILES/LF |
| Python testleri | Geçti | 32 test |
| Kurtarma geçici disk deneyi | Geçti | Gerçek ext4 loop aygıtları, squashfs ve rsync; iki kip |
| Hesap koruma | Geçti | Dosya, uygulama verisi, parola özeti, sudo üyeliği, makine adı korundu |
| Uygulama kaldırma | Geçti | Sistem ve kullanıcı Flatpak kurulumları kaldırıldı |
| Her şeyi kaldır | Geçti | Eski hesap/ev silindi; gerçek useradd/chpasswd ile yeni hesap ve SHA-512 parola özeti oluşturuldu |
| Shell betikleri | Geçti | ShellCheck; GRUB ve canlı kurtarma bileşeni dahil |
| Rust build/clippy/fmt/test | Geçti | Workspace build, Clippy `-D warnings`, fmt kontrolü ve 215 Rust testi |
| Çalışan masaüstünde açılış | Geçti | `scripts/dev-session.sh` yoluyla Wayfire/VNC; yedi yeni/yenilenen uygulama ve üç Ayarlar paneli |

Disk deneyi `sudo python3 scripts/test-recovery-vm.py` ile çalışır. Yalnız
geçici loop aygıtlarını değiştirir, VM'nin sistem diskine dokunmaz. Bu düzende
initramfs/GRUB üretimi atlanır; sıfırlama sonrası gerçek önyükleme kanıtı değildir.

## 24 Ekim son doğrulaması

- `scripts/test-apps-session.sh`: Sistem İzleyici, Diskler, Yedekleme, Kamera, Uçbirim, Belgeler ve Hava Durumu gerçek Wayfire oturumunda açıldı. Her pencerede görüntü alındı. Uçbirimde kabuk komutu çalıştırılıp çıktı dosyası okundu; PDF sayfasındaki metin görüldü; İstanbul için gerçek Open-Meteo yanıtı alınıp önbelleğe yazıldı. Kamera aygıt olmadığı için doğru boş durumu gösterdi. Uygulamalar/Yazıcılar/Kurtarma yolları açıldı.
- Canlı denemede Sistem İzleyici'nin dar pencere başlık setter'ında geçersiz null GValue bulundu; aynı kalıp Saat'te de düzeltildi. Kurtarma'daki bekleyen işlem satırı yalnız gerçekten bekleyen işlem varken görünür.
- `scripts/test-recovery-service-vm.sh`: sistem D-Bus aktivasyonu ve `Status` yanıtı; yetkisiz `Schedule`/`Reset` çağrıları `AccessDenied` ile reddedildi.
- `scripts/test-packages-vm.sh`: yerel `.deb` dosya tanıtıcısıyla planlandı, kuruldu, gerçek dosyası okundu ve servis üzerinden kaldırıldı. Önceki paket yetki/koruma/yabancı plan/bozuk postinst onarım deneyleri de geçti.
- `scripts/test-recovery-vm.py`: iki kip gerçek geçici ext4 loop aygıtlarında tekrar geçti. Ayrı `/home` düzeninin reddi ve imajda bulunmayan eski çekirdeklerin temizlenmesi de birim kapsamındadır.
- CUPS yapılandırması sözdizimi ve betikler denetlendi. Fiziksel yazıcı işi gönderilmedi.

Yeniden çalıştırılabilen kanıtlar geliştirme VM'sinde `target/evidence/apps-recovery`, `target/evidence/apps-session` ve `target/evidence/packages-test` altındadır; Windows çalışma kopyasına da alınır. VNC, kullanıcı denemesi için açık bırakılır.

## Açık kabul kapıları

- Faz 6D Calamares bölüm düzeni ve `install-recovery` çağrısı henüz kurulum
  akışına bağlı değil. ISO üretilip açılmadığı için kurulu ürün üzerinde sıfırlama
  kabulü tamamlanmadı. Kurtarma bölümü bulunmayan sistemde düğme devre dışıdır.
- Gerçek kurtarma ISO açılışı, Secure Boot, elektrik kesintisinden yeniden açılış,
  gerçek kamera ve fiziksel yazıcı testi açık. ISO/donanım kapıları geçmeden
  faz veya sürüm tamamlandı denmez.
- Yeni üç simge ve uygulama ekranlarının kullanıcı görsel kabulü açık.

## Sonraki adaylar

- Kaldırılan uygulamaları ilk oturumda Yazılım'dan seçerek yeniden kurma;
  şimdilik ev dizinine JSON listesi yazılır.
- Kurtarma imajının sürüm yükseltmede yenilenmesi ve şifreli kök desteği.
- Disklerde bölüm tablosu düzenleme; uygulamalardaki ileri özellikler mevcut
  uygulama yol haritasında kalır.
