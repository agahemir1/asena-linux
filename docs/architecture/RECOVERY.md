# Sıfırlama ve kurtarma (K30)

`asena-recovery --service`, sistem yolunda `org.asenalinux.Recovery` adını,
`/org/asenalinux/Recovery` nesnesini sunar. `Status() -> s` JSON durum döndürür;
`Schedule(s mode)`, `Cancel()` ve `Reset(s mode, s username, s password)`
her çağrıda gönderenin D-Bus bağlantısını polkit ile yetkilendirir. Kip yalnız
`keep` veya `remove` olabilir. İstemci bölüm, dosya yolu veya komut gönderemez.
Yazma işleri tek işçiyle, arayüzü durdurmadan yürütülür.

Ayarlar › Kurtarma yalnız işlemi hazırlar. Yeniden başlatma ayrıca seçilir.
GRUB tek seferlik `asena-recovery` girdisini açar; Secure Boot yedek çekirdek
seçicisi bu girdiyi ezmez. Kurtarma ortamında cage içinde ortak `asena-app`
çatısını kullanan pencere açılır. Son silme onayı burada verilir. Her şeyi
kaldırırken yeni hesap ve parola alınır; parola diske, günlüğe veya komut
satırına yazılmaz. `chpasswd` parolayı standart girdiden alır.

Yetkili işçi `/usr/lib/asena/recovery.py` dosyasıdır. D-Bus servisi bu sabit
dosyayı yalıtılmış Python kipinde çalıştırır. Python Debian'ın sistem
kitaplığıdır; ek Python veya Rust bağımlılığı yoktur.

## Kurulum sözleşmesi

Calamares kökü ve ayrı ext4 kurtarma bölümünü bağladıktan, sistemi açtıktan ve
GRUB'u kurduktan sonra şu komutu çağırır:

```sh
/usr/lib/asena/install-recovery TARGET_ROOT RECOVERY_MOUNT LIVE_MEDIUM
```

Araç bölüm oluşturmaz veya biçimlendirmez. İki hedefin bağlı, birbirinden ve
çalışan kökten farklı ext4 bölümleri olduğunu doğrular. Canlı ortamdaki
`live/filesystem.squashfs`, Debian imzalı `live/vmlinuz` ve `live/initrd.img`
kopyalanır; üç dosyanın SHA-256 özeti alınır. Boş alan imajın toplamından en az
512 MiB fazla olmalıdır. Var olan kurtarma imajının üstüne yazılmaz.

Kök/kurtarma/ESP UUID'leri kökte `/etc/asena/recovery.json`, kurtarma bölümünde
`/asena/recovery.json` olarak root sahipliğiyle tutulur. Özetler
`/asena/image.json` içindedir. GRUB girdisi bu UUID ile canlı ortamı bulur.
Kurtarma bölümünün boyutunu ISO üretiminde ölçülen imaj belirler; sabit 4 GB
varsayılmaz. Calamares'in bölüm düzeni ve bu çağrısı Faz 6D kabulünün parçasıdır.

## Veri sınırı ve kesinti

Silmeden önce UUID, ext4, bağlı hedef olmaması, iç içe bağlama olmaması,
imaj özeti, hesap çakışmaları ve kullanıcı dizinleri doğrulanır. Şifreli/LVM
ve ayrı `/home` düzenleri desteklenmez. Canlı ortam dışında `Reset` reddedilir.

`keep`, `/home` ağacını korur. UID 1000–65533 hesapları, parola özetleri,
gruplar/sudo üyeliği, makine adı, dil, klavye, saat dilimi ve kayıtlı
NetworkManager bağlantıları taşınır. Sistem yeniden açıldığı için sonradan
kurulan paketler ve sistem Flatpak'ları gider. Kullanıcının
`~/.local/share/flatpak` kurulumu kaldırılır, `~/.var/app` verisi kalır.
Kaldırılan paket/uygulama kimlikleri ev dizininde
`Asena-removed-applications.json` dosyasına yazılır. Yazılım üzerinden otomatik
yeniden kurma önerisi henüz uygulanmadı.

`remove`, ev dizinlerini ve eski hesapları kaldırır, girilen yeni hesabı
`sudo` grubuyla oluşturur. Calamares'i yeniden çalıştırmaz: var olan kök UUID
ve ESP korunur, bölüm tablosu değişmez. Bu, K30'daki Calamares önerisinin
yerine kullanılan uygulama ayrıntısıdır.

Hesap ve makine bilgileri silme öncesi kurtarma bölümünün root erişimli
`/asena/state` dizinine atomik yazılır. `/boot/grub` silinmez; işlem sırasında
GRUB bir sonraki açılış için kurtarmayı yeniden işaretler. Kesinti sonrası aynı
kipten yeniden denenir. Başarıdan sonra geçici hesap kayıtları silinir.
Sıfırlama güvenli disk silme değildir; disk bozulmasına karşı da yedek sayılmaz.

## Doğrulama

`tests/test_recovery.py` veri koruma, uygulama kaldırma, hesap birleştirme,
kimlik/parola doğrulama ve bozuk imaj reddini denetler.
`sudo python3 scripts/test-recovery-vm.py` yalnız geçici loop aygıtlarında gerçek
ext4, squashfs, rsync, useradd ve chpasswd ile iki sıfırlama kipini dener.
Bu düzenekte initramfs ve GRUB üretimi atlanır; gerçek ISO/Secure Boot açılışı
yerine geçmez. Kabul sonuçları ilgili [raporda](../reports/apps-recovery.md) tutulur.
