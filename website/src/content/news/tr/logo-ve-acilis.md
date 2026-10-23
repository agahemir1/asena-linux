---
title: Logo ve açılış, Selçuk yıldızından bir çini
description: Asena’nın logosu seçildi ve açılıştan masaüstüne her yere yerleşti. Varsayılan çekirdek XanMod LTS oldu.
permalink: logo-ve-acilis
key: logo-and-boot
date: 2026-10-23T10:00:00+03:00
---

## Logo

Logo, başlat menüsündeki sekiz köşeli yıldızdan çizilen bir çini plaka: turkuaz sır, aralarında krem bir bant olan iç içe iki kare. Şu yerlerde görünüyor:

- GRUB menüsü (“Asena GNU/Linux”) ve canlı ISO’nun önyükleme menüsü
- Plymouth açılış ekranı
- fastfetch ve konsol girişindeki `/etc/issue`
- Giriş ve kilit ekranı, Ayarlar › Hakkında

Logo üç dosyadan oluşuyor: renkli SVG, tek renk sürüm ve uçbirim için ASCII sürüm.

Bir ayrıntı: Debian’ın imzalı GRUB’u yapılandırmasını yalnız `EFI/debian` altında arıyor. Dizin adını Asena yapsaydık sistem ilk GRUB güncellemesinden sonra açılmazdı; bunu sanal makinede gördük. Bu yüzden EFI dizini `debian` kalıyor, menüde ise Asena yazıyor.

## Duvar kâğıtları

22 Ekim’de on yeni duvar kâğıdı geldi; varsayılanı Asena Gece. Hepsi CC BY-SA 4.0. Her birinin motifini [Hikâye](/hikaye/) sayfasında anlattık.

## Çekirdek

Varsayılan çekirdek artık XanMod LTS; Debian’ın imzalı çekirdeği her zaman yedekte kurulu duruyor. Test diskinde Secure Boot kapalıyken XanMod 6.18, açıkken Debian’ın 6.12 çekirdeği açıldı. İki durumda da takas için zram etkindi. Deneme sanal makinede yapıldı; gerçek donanım ve gerçek donanım yazılımı henüz denenmedi.
