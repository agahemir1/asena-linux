---
title: Sık sorulanlar
description: Ücret, indirme, GNOME ve KDE, Debian, X11, Flatpak, gizlilik ve kaynak kodu hakkında.
permalink: sss
key: faq
order: 8
---

## Asena ücretli mi?

Hayır. Asena özgür yazılımdır; kaynak kodu GPL-3.0-or-later, duvar kâğıtları CC BY-SA 4.0 lisanslıdır.

## Ne zaman indirebilirim?

İlk kurulabilir ISO henüz derlenmedi ve bir tarih vermiyoruz. Hazır olduğunda [İndir](/indir/) sayfasında ve [haber akışında](/haberler/rss.xml) duyuracağız.

## GNOME ya da KDE tabanlı mı?

Hayır. Panel, menü, bildirimler, kilit ve giriş ekranı Asena’nın kendi kabuğudur ve Wayfire üzerinde çalışır. Uygulamalar GTK4 ve libadwaita kullandığı için GNOME uygulamalarıyla aynı görünür. GNOME Shell uzantıları çalışmaz.

## Debian’ın bir sürümü mü?

Asena, Debian 13 üzerine kurulur ama Debian projesinin bir parçası değildir. Sistem kimliği `ID=asena` ve `ID_LIKE=debian`’dır; sürüm kod adı `trixie` olarak kaldığı için Debian’a göre yazılmış üçüncü taraf depolar çalışır. Dağıtımı yalnız `ID` alanına bakarak tanıyan kurulum betikleri Asena’yı tanımayabilir.

## X11 uygulamaları çalışır mı?

Evet, XWayland üzerinden. Ayrı bir X11 oturumu yoktur.

## Flatpak uygulamaları çalışır mı?

Evet. Bilinen bir fark var: Wayfire yapılandırmamızda `wp-security-context-v1` protokolü yok. Uygulamalar çalışır, ama compositor sandbox içindeki bir uygulamayı ekran yakalama gibi ayrıcalıklı protokollerden ayıramaz.

## Neden Wayfire?

Panel ve menülerin arkasını gerçekten bulanıklaştırmak için. Debian 13’teki Wayfire 0.9 bunu yapabiliyor. Kabuk compositor’le yalnızca standart Wayland protokolleriyle konuştuğu için geçişte kabuk kodu değişmedi; gerekirse compositor yine değiştirilebilir.

## Uçbirim neden menüde yok?

Asena’nın kendi uçbirimi henüz hazır değil. Yerine Foot kurulu gelir; menüden ya da <kbd>Super</kbd> + <kbd>Enter</kbd> ile açılır.

## Kullanım verisi toplanıyor mu?

Asena’nın kendi bileşenleri kullanım verisi toplamaz. Bu bileşenler ağa yalnız iş yaparken çıkar: güncellemeleri denetlerken Debian depolarına, Yazılım’da katalog ve ekran görüntüleri için Debian ve (açtıysanız) Flathub sunucularına. Firefox gibi uygulamaların kendi ayarları vardır.

## Kaynak kodu nerede?

Herkese açık kaynak deposu henüz açılmadı. Açıldığında buradan duyuracağız.
