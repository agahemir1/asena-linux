---
title: Wayfire’a geçtik, bulanıklık gerçek oldu
description: Asena artık tek compositor olarak Wayfire 0.9 kullanıyor. Panelin ve menülerin arkası gerçekten bulanıklaşıyor.
permalink: wayfire-gecisi
key: wayfire
date: 2026-10-21T18:00:00+03:00
---

Odak tasarımı panelde ve geçici yüzeylerde buzlu cam istiyordu: arkada kalan pencerenin gerçekten bulanıklaşması. Bunu GTK yapamaz; pencerelerin arkasını compositor çizer.

## Deneme

Debian 13’teki Wayfire 0.9.0’ın blur eklentisini Dual Kawase yöntemiyle denedik. Panelin, başlat menüsünün, denetim merkezinin ve bildirim panelinin arkası bulanıklaştı. Kabuk kodunda tek satır değişmedi, çünkü kabuk compositor’le yalnızca standart Wayland protokolleriyle konuşuyor. Aynı sonuç labwc’de daha yeni labwc, wlroots ve SceneFX sürümlerini kendimiz paketlemeyi gerektirecekti.

## Karar

21 Ekim’de Wayfire tek compositor oldu, labwc kaldırıldı. Ayarlar › Görünüm’e **Saydamlık efektleri** anahtarı geldi. Kapalıyken ya da yüksek kontrastta yüzeyler opak olur ve blur eklentisi hiç yüklenmez. Kısayollar, giriş aygıtı ayarları ve animasyon tercihi asena-daemon tarafından Wayfire yapılandırmasına yazılıyor ve hemen uygulanıyor.

## Denenenler

Sanal makinede oturum, kilit ekranı, giriş ekranı ve Flathub’dan bir uygulama (dosya seçici ve belge portalı dahil) çalıştı. Yazılımla çizimde (llvmpipe) hareket sırasında işlemci kullanımı yaklaşık bir buçuk katına çıktı. Gerçek ekran kartında ölçüm henüz yapılmadı.

## Bilinen fark

Wayfire yapılandırmamızda `wp-security-context-v1` protokolü yok. Flatpak uygulamaları çalışıyor, ama compositor sandbox içindeki bir uygulamayı ekran yakalama gibi ayrıcalıklı protokollerden ayıramıyor. Bunu açıkça not ediyoruz.
