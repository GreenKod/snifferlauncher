# SnifferLauncher Görsel & Grafik Motoru Master Planı (30 Aşama)
## Rendering, Visuals, Batching & Physics Architecture

Bu plan, SnifferLauncher grafik ve görsel işleme altyapısında (`sniffer_render`, `sniffer_core`, `.plugins/framework`) tespit edilen 6 ana grafiksel iyileştirme ve modernleştirme alanını, her biri bağımsız olarak uygulanabilir, derlenebilir ve test edilebilir **5'er alt aşamaya** bölerek toplam **30 aşamada** tamamlanacak şekilde yapılandırılmıştır.

---

## Katı Uygulama ve Doğrulama Kuralları

1. **Tek Seferde Tek Aşama:** Asla aynı anda birden fazla aşamaya geçilmeyecek. Her adımda sadece üzerinde çalışılan tek bir aşama uygulanacak.
2. **Zorunlu Doğrulama Kapısı:** Her aşamanın kodu yazıldıktan hemen sonra yerel terminalde:
   - `cargo check --workspace --all-targets`
   - `cargo test --workspace`
     komutları çalıştırılıp yeşil yandığı kanıtlanacak. Tek bir derleme hatası veya kırık test varken bir sonraki adıma geçilmeyecek.
3. **Linter & Format Temizliği:** Her adımdan sonra `cargo clippy --locked --workspace --all-targets -- -D warnings` ve `cargo fmt --all --check` kontrolleri sıfır uyarı/diff ile temiz tutulacak.
4. **Atomik İlerleme ve Onay:** Testler geçtikten sonra o aşamanın özeti ve yapılan doğrulama kullanıcıya raporlanıp onay alındıktan sonra plan üzerinde `[x]` olarak işaretlenecek.

---

## 🔤 Bölüm 1: Metin Çizimlerinin Toplu İşlenmesi (Glyph & Text Batching — `sniffer_render`)

_Hedef: Ekranda yer alan onlarca metin/etiket için ayrı ayrı GPU buffer tahsisini ve `glDrawArrays` çağrılarını kaldırarak tek geçişli toplu metin çizimine (`TextBatch`) geçmek._

- [x] **Aşama 1.1:** `TextBatch` tampon yönetiminin tasarlanması: Kalıcı dinamik Vertex/Instance VBO (`text_instance_vbo`) ve VAO (`text_instance_vao`) kurulumu.
- [x] **Aşama 1.2:** `draw_text_impl` fonksiyonunun her çağrıda `Vec<f32>` heap tahsisi yapması yerine glif köşe verilerini `TextBatch` içine biriktirecek şekilde dönüştürülmesi.
- [x] **Aşama 1.3:** `flush_text()` fonksiyonunun yazılması; `draw_ui` bitişinde veya state değişiminde biriken tüm gliflerin tek bir `glDrawArrays` çağrısıyla GPU'ya aktarılması.
- [x] **Aşama 1.4:** Renkli emojiler (`is_color == true`) ile standart font gliflerinin batch içinde shader uniform'unu değiştirmeden köşe özniteliği (vertex attribute) ile ayrıştırılması.
- [x] **Aşama 1.5:** 100+ etiket içeren büyük arayüzlerde (App Drawer / listeler) draw call sayısının ve CPU çizim süresinin düşüşünün test ve benchmark ile doğrulanması.

---

## ✂️ Bölüm 2: Donanım Destekli Yumuşak Kırpma ve Yuvarlak Köşe Maskeleme (Rounded Scissor & SDF Clipping — `sniffer_render`)

_Hedef: `overflow_hidden` ve `ScrollView` bileşenlerindeki içerik taşmalarını sert eksen hizalı kesim yerine yuvarlak köşeli (antialiased rounded clipping) olarak pürüzsüzce maskelemek._

- [x] **Aşama 2.1:** `Renderer` clip stack yapısına yuvarlak köşe yarıçapı (`radius`) ve transform matrisini tam olarak entegre eden `ClipRegion` veri modelinin eklenmesi.
- [x] **Aşama 2.2:** Eksen hizalı dik açılı kırpmalar için hızlı donanımsal `glScissor` yolunun korunması; yuvarlak köşeli bölgeler için fragman shader tabanlı SDF clip hesaplama yolunun açılması.
- [x] **Aşama 2.3:** `QuadBatch` ve `TextBatch` shader'larına aktif kırpma sınırları (`u_clip_rect`, `u_clip_radius`) desteğinin eklenmesi.
- [x] **Aşama 2.4:** `ScrollView` içinde kaydırılan çocuk elemanların yuvarlak köşe sınırlarında pürüzsüz antialiased olarak kesildiğinin doğrulanması.
- [x] **Aşama 2.5:** İç içe (nested) kırpma alanları ve dönme/ölçekleme transformasyonları altında kırpma hassasiyetinin test edilmesi.

---

## 🌫️ Bölüm 3: Çift Geçişli Arka Plan Bulanıklığı ve Cam Efekti (Dual-Pass Kawase Blur / Glassmorphism — `sniffer_render`)

_Hedef: Dock bar, uygulama çekmecesi veya popup kartların arkasındaki duvar kağıdı ve içeriğe 60/120 FPS akıcılıkta gerçek zamanlı donanım bulanıklığı (frosted glass) kazandırmak._

- [x] **Aşama 3.1:** Arka plan yakalama ve bulanıklaştırma için FBO (Framebuffer Object) ve çift doku (ping-pong FBOs) yaşam döngüsünün kurulması.
- [x] **Aşama 3.2:** Mobil GPU'lar için optimize edilmiş aşağı ölçekleme (half-res downsampling) ve hızlı Dual-Pass Kawase / Gaussian Blur fragment shader'larının geliştirilmesi.
- [x] **Aşama 3.3:** `Style` struct'ına `backdrop_blur: f32` ve `backdrop_tint: Option<u32>` özelliklerinin eklenmesi ve JS API'ye aktarılması.
- [x] **Aşama 3.4:** `draw_element_contents` içinde `backdrop_blur > 0.0` olan kartların arka plan dokusunu FBO üzerinden işleyip karta bind edecek render pipeline adımının bağlanması.
- [x] **Aşama 3.5:** Android ve Desktop ortamlarında 120Hz ekranlarda kare süresi (frame budget) etkisi ve VRAM tüketiminin test edilip optimize edilmesi.

---

## 🖼️ Bölüm 4: Çoklu Resim ve İkon Çizimlerinin Toplu İşlenmesi (Image & Icon Batching — `sniffer_render`)

_Hedef: Izgara (grid) içerisindeki onlarca uygulama ikonu ve resim için her kare yapılan ayrı ayrı shader ve doku değişimlerini (texture bind / draw call) en aza indirmek._

- [x] **Aşama 4.1:** Çoklu ikon ve küçük resimler için dinamik GPU Texture Atlas / Texture Array tahsis mekanizmasının tasarlanması.
- [x] **Aşama 4.2:** `ImageInstanceData` ve instanced image shader (`image_desktop.vs`, `image_android.vs`) altyapısının kurulması.
- [x] **Aşama 4.3:** `draw_image_impl` çağrılarının atlaslanan resimlerde anında çizim yerine `ImageBatch` tamponuna veri ekleyecek şekilde refactor edilmesi.
- [x] **Aşama 4.4:** `flush_images()` çağrısının render döngüsüne eklenmesi; doku atlasındaki tüm ikonların tek bir instanced çağrıyla GPU'ya gönderilmesi.
- [x] **Aşama 4.5:** 60+ ikon içeren uygulama çekmecesinde (App Drawer) draw call sayısının ve GPU state değişimlerinin dramatik düşüşünün doğrulanması.

---

## 🌓 Bölüm 5: Gelişmiş Dinamik Gölgeler ve Şekil Efektleri (Elevation, Multi-Layer Shadows & Border Styles — `sniffer_render`)

_Hedef: Basit tek katmanlı gölge yerine gerçekçi yükseklik (elevation) hissi veren çok katmanlı ortam gölgeleri ve zengin kenarlık stilleri sunmak._

- [x] **Aşama 5.1:** `Style` içine `elevation: f32`, `shadow_spread: f32`, `shadow_blur: f32` ve `border_gradient: Option<(u32, u32)>` tanımlarının tam olarak işlenmesi.
- [x] **Aşama 5.2:** `QuadInstanceData` struct'ına ve `shape` fragment shader'ına analitik SDF çift gölge (ambient occlusion + key light) hesaplamasının eklenmesi.
- [x] **Aşama 5.3:** Kenarlık (border) için doğrusal degrade (gradient border) ve kesintisiz kenar antialiasing desteğinin SDF shader'a eklenmesi.
- [x] **Aşama 5.4:** Kartlara basıldığında (pressed / active state) gölgenin ve boyutun dinamik olarak yumuşak bir biçimde değiştiği (elevation transition) animasyon entegrasyonu.
- [x] **Aşama 5.5:** Gölgelerin GPU fill-rate maliyetinin optimize edilmesi; aşırı geniş gölgelerde aşırı çizimin (overdraw) sınırlandırılması.

---

## 🌀 Bölüm 6: 120 FPS Yerel Fizik ve Yay Animasyon Motoru (Native Spring Physics & Gesture Motion — `sniffer_core` / `sniffer_render`)

_Hedef: Kullanıcı parmağını kaydırıp bıraktığında (fling) veya elastik kenara çarptığında (rubber-band) JS thread'ini uyandırmadan 120 FPS akıcı fizik hareketleri üretmek._

- [x] **Aşama 6.1:** `sniffer_core::anim` içine analitik ikinci derece yay fiziği çözücüsü (`SpringSimulation`: kütle, sertlik, sönümleme katsayıları) eklenmesi.
- [x] **Aşama 6.2:** `ScrollView` bileşenindeki rubber-band ve snap mekanizmasının zaman bazlı interpolasyondan doğrudan `SpringSimulation` çözücüsüne bağlanması.
- [x] **Aşama 6.3:** Dokunma bırakma anındaki fırlatma hızı (fling velocity) hesaplayıcısının entegrasyonu ve momentum kaydırmasının pürüzsüzleştirilmesi.
- [x] **Aşama 6.4:** Sayfa geçişleri ve uygulama çekmecesi açılışında yay dalgalanması (staggered cascade animation) desteğinin eklenmesi.
- [x] **Aşama 6.5:** 120 Hz ekranlarda girişten çizime (touch-to-render) gecikmenin ve frame drop (jank) durumlarının profiler ile test edilip doğrulanması.
