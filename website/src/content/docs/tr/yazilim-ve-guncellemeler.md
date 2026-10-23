---
title: Yazılım ve güncellemeler
description: Yazılım uygulamasıyla Debian paketleri ve Flatpak uygulamaları kurmak, güncellemeler ve onarım.
permalink: yazilim-ve-guncellemeler
key: software-and-updates
order: 6
---

## Yazılım uygulaması

Yazılım’da dört bölüm var: **Keşfet** (kategoriler), arama, **Kurulu** ve **Güncellemeler**. Bir uygulamaya tıklayınca açıklaması, lisansı, web sitesi ve ekran görüntüleri gelir.

Katalog iki kaynaktan oluşur: Debian’ın uygulama kataloğu (DEP-11) ve etkinse Flathub. Aynı uygulama iki kaynakta da varsa tek girdide birleşir ve hangisinden kuracağınızı seçersiniz. Uygulama kaydı olmayan paketler, örneğin komut satırı araçları, katalogda görünmez; onları her Debian sisteminde olduğu gibi `apt` ile kurarsınız.

## Debian paketi kurmak ve kaldırmak

Yazılım önce sistemde neyin değişeceğini gösterir: kurulacak, yükseltilecek ve kaldırılacak paketler. Onaylarsanız parolanız sorulur. İşi Asena’nın paket servisi yapar; yetkiyi polkit denetler.

Servis bazı işleri reddeder: geçersiz paket adları, sistemi çalışmaz bırakacak korumalı paketlerin kaldırılması (örneğin `sudo`) ve siz onay verdikten sonra değişmiş, bayatlamış planlar. Aynı anda yalnız bir paket işi çalışır.

## Flatpak ve Flathub

Flathub’ı Yazılım’dan siz açarsınız; kendiliğinden eklenmez. Flatpak uygulamaları kullanıcınıza kurulur. Bir uygulamanın istediği izinler (dosyalarınız, ağ, aygıtlar gibi) kurulumdan önce listelenir. Kurulum, kaldırma ve güncelleme sürerken İptal düğmesi her zaman vardır.

## Güncellemeler

- Asena günde bir kez güncellemeleri denetler, varsa arka planda indirir ve tek bir bildirim gösterir.
- Sistem güncellemeleri siz yeniden başlattığınızda, masaüstü açılmadan önce kurulur. Güncellemeler sayfasındaki **Yeniden başlat ve kur** düğmesi bunu hemen başlatır.
- Sonuç bir sonraki oturumda Yazılım’da görünür.
- Flatpak uygulamaları ve çalışma zamanları oturum içinde güncellenir.
- Bilgisayar hiçbir zaman kendiliğinden yeniden başlamaz ve `unattended-upgrades` kurulu değildir.

## Onarım

Bir paket işi yarıda kalırsa, örneğin elektrik kesilirse, Yazılım bunu algılar ve bir onarım şeridi gösterir. Onarım da parolanızı ister.

## Komut satırı

Asena bir Debian sistemidir; `apt` her zamanki gibi çalışır. Kurulumda oluşturulan ilk kullanıcı `sudo` grubundadır.
