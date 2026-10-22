# K05: Etkin klavye kaynağı

19 Ekim 2026. GTK **4.18** sınırı içinde GdkDevice
[`get_active_layout_index`](https://docs.gtk.org/gdk4/method.Device.get_active_layout_index.html)
ve [`get_layout_names`](https://docs.gtk.org/gdk4/method.Device.get_layout_names.html)
API'leri var. Yerel gdk4 0.9.6 binding'inde ikisi de `v4_18` ile korunuyor.
Caps Lock, `get_caps_lock_state` ve notify sinyaliyle okunabilir.

Faz 4A kilidi, klavye odağı kendisindeyken bu device değerlerini kullanır.
Geçersiz indeks veya olmayan klavyede düzen gizlenir. GSettings tercih listesi
etkin düzenin kanıtı sayılmaz. Düzen adları backend verisidir; Asena bunları
tahmin ederek TR-Q/TR-F/US etiketlerine dönüştürmez.

Kaynak/API incelemesi tamamlandı; TR-Q/TR-F/US ve Caps Lock çalışma zamanı kabulü
henüz yapılmadı. Odağı olmayan panelin küresel grup güncellemesini aynı kaynaktan
sürekli alabildiği ayrıca kanıtlanmalı. Kilit widget'ının desteklenmesi bu genel
panel sözleşmesini tamamlandı yapmaz; K05 açık kalır.
