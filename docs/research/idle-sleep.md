# Boşta kalma ve uyku sözleşmesi

20 Ekim 2026. Debian VM swayidle 1.8.0, logind 257 ailesi ve
`InhibitDelayMaxUSec=30000000` değerini bildirdi. Gerçek uyku henüz denenmedi.

Swayidle yalnız idle zamanlarını üretir; `before-sleep` kaydı verilmez. Böylece
uyku geciktirmesinin sahibi daemon olur. `-C /dev/null -w` ile kullanıcıya ait
başka swayidle yapılandırması yüklenmez ve eylem komutu bitmeden yenisi üst üste
başlatılmaz. Program yolu POSIX alıntılanır; kullanıcı metninden komut üretilmez.
Başvurulan API: [swayidle 1.8.0 man sayfası](https://github.com/swaywm/swayidle/blob/1.8.0/swayidle.1.scd).

logind inhibitörü bir FD ile yaşar; son FD kapanınca bırakılır. `delay` sonludur.
Daemon hazır kilit yanıtı bekler; süre aşımı dışarıdan başlatılmış uykuyu sonsuza
kadar engellemez. `Suspend(false)` yetkilendirme diyaloğu açmaz; `CanSuspend=yes`
olmadan kullanılmaz. [logind D-Bus sözleşmesi](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html),
[inhibitör](https://www.freedesktop.org/software/systemd/man/latest/systemd-inhibit.html).
Bu kaynaklar API açıklamasıdır; VM uyku kabulünün yerine geçmez.

Kapak/dock/AC politikasını logind yönetir. Asena `handle-lid-switch` inhibitörü
almaz veya ikinci bir kapak eylemi üretmez. Donanım matrisi açık kalır.

Uyku bekletmesi, ekran kapatma boyunca da parola kabulünü durdurur. Ayrı bağlantı
sahipliği sahibin çökmesini fark eder. Uyanma yalnız sistem bekletmesini temizler;
yaşayan ekran gücü sahibini temizlemez. Ekran geri gelmeden yanlışlıkla kilit
açılmasına izin veren yarış bu ayrımla önlenir. Native PAM/GTK kopyaları ve gerçek
DRM/TTY koşulları ayrıca güvenlik incelemesine tabidir.
