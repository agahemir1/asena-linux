---
title: Sistem gereksinimleri
description: İlk sürümün destekleyeceği donanım ve kurulum biçimi.
permalink: sistem-gereksinimleri
key: system-requirements
order: 2
---

Bu sayfa ilk sürüm için alınmış kararları anlatır. İlk ISO henüz derlenmediği için bellek ve disk alt sınırları ölçülmedi; ölçülünce buraya yazılacak.

## İşlemci ve önyükleme

- 64 bit x86 işlemci (amd64).
- UEFI. Eski BIOS kipi desteklenmeyecek.
- Varsayılan çekirdek XanMod LTS’dir ve x86-64-v2 destekleyen bir işlemci ister; kabaca 2009’dan sonra çıkan işlemciler. Daha eski bir işlemcide XanMod açılmaz ve bu bir yükleme hatası sayılmadığı için menü kendiliğinden yedeğe geçmez: önyükleme menüsünden Debian çekirdeğini seçersiniz.
- Debian’ın imzalı çekirdeği her zaman yedekte kurulu durur.

## Secure Boot

Asena, Debian’ın imzalı shim, GRUB ve çekirdeğini kullanır. Secure Boot açıkken Debian çekirdeği, kapalıyken XanMod açılır. Bu düzen sanal makinede iki kez denendi; gerçek donanımda henüz denenmedi. Microsoft’un 2011 sertifikalarını 2023 sertifikalarıyla değiştiren donanım yazılımlarındaki durum ayrıca doğrulanacak.

## Ekran kartı

Masaüstü OpenGL ES ile çizilir; bulanıklık ve pencere animasyonları ekran kartında hesaplanır. Eski ya da sürücüsü sorunlu bir kartta Ayarlar › Görünüm’den **Saydamlık efektleri**ni, Ayarlar › Erişilebilirlik’ten animasyonları kapatmak yükü azaltır.

## Bellek

Performans hedefleri 4 çekirdekli, 8 GB belleği, tümleşik ekran kartı ve SSD’si olan bir başvuru makinesine göre konuldu. Takas alanı olarak sıkıştırılmış bellek (zram) varsayılan olarak açıktır.

## Kurulum biçimi

- İlk sürümde Asena boş bir diske tek sistem olarak kurulur.
- Yanında Windows ya da başka bir sistem (dual boot) ve disk şifreleme ilk sürümde yok.
- Kurulum aracı Calamares olacak. Dil, klavye ve saat dilimini sorar; konumunuzu bulmak için ağa sormaz.
- Root hesabı kilitli olur, kurulumda oluşturduğunuz ilk kullanıcı yöneticidir.

## Donanım yazılımı

Debian’ın `non-free-firmware` bileşeni ve Intel ile AMD işlemci mikro kodları canlı sistemde hazır gelir ve kuruluma da geçer. Kapalı kaynak sürücüler (`non-free`) eklenmez.
