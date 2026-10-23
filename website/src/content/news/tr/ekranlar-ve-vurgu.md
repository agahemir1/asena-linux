---
title: Ekran düzeni, vurgu renkleri ve TR-F
description: Ayarlar › Ekranlar düzeni değiştiriyor ve onaylanmazsa 15 saniyede geri alıyor. Vurgu rengi Flatpak uygulamalarına da geçiyor.
permalink: ekranlar-ve-vurgu
key: displays-and-accents
date: 2026-10-22
---

## Ekranlar

Ayarlar › Ekranlar artık birincil ekranı, çözünürlüğü, yenileme hızını, ölçeği, döndürmeyi ve ekranların konumunu değiştiriyor. Değişiklik önce deneniyor. 15 saniye içinde onaylamazsanız, Ayarlar kapanırsa ya da ekranlar değişirse eski düzen geri geliyor. Onaylanan düzen o ekran takımı için saklanıyor ve aynı ekranlar takıldığında yeniden uygulanıyor. Bu işin tek sahibi asena-daemon; Ayarlar yalnız istiyor.

Yolda iki sorun bulduk. GTK 4.18, ölçeği 1’e dönen bir ekranda eski geometriyi bildiriyor; kabuk bu yüzden ekran geometrisini compositor’den okuyor. wlroots 0.18 de modu olmayan sanal çıkışlarda çöküyordu; yukarı akıştaki düzeltmeyi yamalı bir paketle aldık. Gerçek monitörlerle ve ekran takıp çıkararak yapılacak kabul testi sürüyor.

## Vurgu renkleri

asena-daemon artık Flatpak uygulamaları için Settings portalının arka ucunu sunuyor. Koyu stil ve vurgu rengi bu uygulamalara anında geçiyor. Klasör simgeleri ile Dosyalar ve Ayarlar simgeleri de seçtiğiniz vurguyu izliyor; her vurgu için ayrı çizilmiş bir simge teması var.

## TR-F

Wayfire kısayolları fiziksel tuşa bakar. Bu yüzden TR-F klavyede <kbd>Super</kbd> + <kbd>A</kbd> başka bir tuşa düşüyordu. Artık harfli kısayollar, ilk klavye düzeninizde o harfi yazan tuşa çevriliyor.
