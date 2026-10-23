---
title: Yazılım, Debian ve Flathub tek katalogda
description: Asena’nın yazılım uygulaması ve arkasındaki paket servisi hazır. Sistem güncellemeleri yeniden başlatmada kuruluyor.
permalink: yazilim
key: software
date: 2026-10-23T14:00:00+03:00
---

Yazılım, 21 Ekim’deki denetimde gizlenen beş uygulamadan biriydi. Artık menüde ve gerçekten çalışıyor.

## Katalog

Debian’ın uygulama kataloğu (DEP-11) ve her Flatpak deposu ayrı okunuyor; aynı uygulama tek girdide birleşiyor ve nereden kuracağınızı seçiyorsunuz. Arama Türkçe harfleri doğru eşliyor, listeler Türkçe sıralanıyor. Ekran görüntüleri yalnız ayrıntı sayfasını açınca iniyor.

## Debian paketleri

Kurma ve kaldırma işini yeni paket servisi `org.asenalinux.Packages` yapıyor. Önce neyin değişeceğini görüyorsunuz, sonra polkit parolanızı soruyor. Servis geçersiz adları, `sudo` gibi korumalı paketlerin kaldırılmasını ve bayatlamış planları reddediyor; aynı anda yalnız bir APT işi çalışıyor.

## Flatpak

Flatpak uygulamaları kullanıcınıza kuruluyor. İstedikleri izinler kurulumdan önce gösteriliyor. Flathub’ı siz açıyorsunuz; ağ hatasında yeniden denenebiliyor. Kurulum, kaldırma ve güncelleme sürerken İptal düğmesi var.

## Güncellemeler

Asena günde bir kez denetliyor, güncellemeleri arka planda indiriyor ve tek bir bildirim gösteriyor. Sistem güncellemeleri yeniden başlatmada, masaüstü açılmadan kuruluyor; sonuç bir sonraki oturumda görünüyor. Flatpak güncellemeleri oturum içinde yapılıyor. `unattended-upgrades` yok, bilgisayar kendiliğinden yeniden başlamıyor.

## Denenenler

Sanal makinede `hello` paketi kuruldu ve kaldırıldı. Bekleyen yedi Debian güncellemesi (`tzdata` 2026c’den 2026e’ye dahil) yeniden başlatmada kuruldu. Kurulum betiği bilerek başarısız olan bir test paketinden sonra sistem yine açıldı; paket servisi yarım kalan işi algıladı ve onardı.
