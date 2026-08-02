# mega-ui

Immediate/retained UI для игровых движков. **Сама ничего не рисует на GPU** — выдаёт список `DrawCommand`, а ты уже рисуешь их как хочешь (wgpu, OpenGL, …).

Зависимости либы: только `glam` + `fontdue`. Никакого wgpu/winit внутри.

---

## Идея за 30 секунд

```text
каждый кадр:
  события окна  →  UiInput
  ui.begin_frame(input)
  ui.button / window / dock / …   // описание UI
  output = ui.end_frame()
  output.draw_list  →  твой рендерер
  output.cursor / clipboard / needs_repaint  →  окно
```

Состояние виджетов (скролл, фокус, open у select) хранит `Ui`.  
Данные приложения (`&mut String`, `&mut f32`, …) хранишь ты.

---

## Подключение

```toml
[dependencies]
mega-ui = { path = "../path/to/mega-ui" }
glam = "0.33"
```

```rust
use mega_ui::{Ui, UiInput, DrawCommand, CursorIcon};
```

Локальная демка (winit + wgpu, только dev-зависимости):

```bash
cargo run --example demo
```

Рабочий пример целиком также можно смотреть в проекте `engine` рядом с этой либой (`app_ui.rs` + `main.rs` + `shader.wgsl`).

---

## Кадр (минимум)

```rust
let mut ui = Ui::new();

// 1) собери input из своего windowing (winit и т.п.)
let input = UiInput {
    mouse_pos,
    mouse_down,
    mouse_pressed,   // true один кадр при press
    mouse_released,  // true один кадр при release
    viewport: Vec2::new(width, height),
    scroll_delta,    // пиксели; +y = колесо вверх
    dt,              // секунды
    text: typed_chars,
    key_backspace: …,
    // остальные key_* — по необходимости (текст, clipboard)
    clipboard: paste_text, // если key_paste
    ..Default::default()
};

ui.set_scale(1.0); // 2.0 = UI в 2 раза крупнее
ui.begin_frame(input);

if ui.button("Save").clicked() { /* … */ }
ui.text_input("name", &mut name);

let out = ui.end_frame();

// 2) курсор
window.set_cursor(map_cursor(out.cursor));

// 3) clipboard out (copy/cut)
if let Some(text) = out.clipboard { clipboard.set_text(text); }

// 4) если out.needs_repaint — запроси ещё один кадр (плавный скролл)

// 5) нарисуй out.draw_list
```

`mouse_pressed` / `mouse_released` и `key_*` должны жить **один кадр** — после передачи в UI сбрасывай флаги.

---

## Что такое DrawCommand

Каждая команда = один textured/colored quad:

| поле | смысл |
|------|--------|
| `rect` | экранные пиксели (origin = top-left) |
| `uv_min` / `uv_max` | UV в атласе; для solid `uv_min == uv_max` (белый тексель) |
| `color` | RGBA 0..1, умножается на сэмпл |
| `kind` | `0` = font atlas (альфа в `.r`), `1` = твоя RGBA-текстура |
| `tex` | слот хост-текстуры при `kind == 1` (`0` = image, `1` = …) |

Виджеты:

- обычная заливка / скругления → `kind = 0`, solid UV
- текст → `kind = 0`, UV глифа в атласе
- `ui.image(size)` → `kind = 1`, `tex = 0`
- `ui.texture(slot, size)` → `kind = 1`, `tex = slot` (сцена, превью, что угодно)

UI **не знает**, что лежит в слоте. Ты биндишь свои `TextureView` сам.

---

## wgpu: что нужно сделать в приложении

Либа не даёт готовый pipeline — его пишешь один раз и копируешь между проектами (см. `engine`).

### 1. Font atlas → текстура

```rust
let (pixels, w, h) = ui.font_atlas(); // R8, row-major
// TextureFormat::R8Unorm, TEXTURE_BINDING | COPY_DST
```

Каждый кадр после UI:

```rust
if ui.font_atlas_take_dirty() {
    // если размер изменился — пересоздай texture + bind group
    queue.write_texture(/* atlas pixels */);
}
```

Шрифт по умолчанию: Segoe UI (Windows). Свой:

```rust
ui.set_font_bytes(include_bytes!("MyFont.ttf"), 14.0)?;
// или ui.set_font_path("fonts/MyFont.ttf", 14.0)?;
```

### 2. Вершины

На каждый `DrawCommand` — 2 треугольника (6 вершин). Пример атрибутов:

```text
pos:   vec2   // пиксели
uv:    vec2
color: vec4
kind:  f32
tex:   f32    // номер слота
```

Viewport uniform: размер окна → в вершинном шейдере перевод в clip space (Y вниз как в UI).

### 3. Шейдер (логика)

```text
if kind < 0.5:
    // font / solid: sample font atlas (.r = alpha), color * alpha
else:
    // host texture: sample tex0 / tex1 / … по полю tex
```

Сэмплер: linear, clamp. Blending: обычный alpha (`src_alpha`, `one_minus_src_alpha`).

### 4. Bind group (типично)

```text
0  uniform  viewport
1  texture  font atlas (R8)
2  sampler
3  texture  slot 0  (например ui.image)
4  texture  slot 1  (например сцена / viewport)
```

Слоты можешь расширять как нужно — главное, чтобы `DrawCommand.tex` совпадал с биндингом.

### 5. Pass

1. (опционально) нарисуй 3D/игру в offscreen, если показываешь через `ui.texture`
2. sync font atlas
3. залей vertex buffer из `draw_list`
4. UI render pass поверх swapchain (или в свой target)

Порядок в `draw_list` уже правильный (окна, оверлеи). Клиппинг заложен в сами rect’ы команд.

---

## Ввод (winit → UiInput)

Минимальный набор:

| событие | поле |
|---------|------|
| `CursorMoved` | `mouse_pos` |
| ЛКМ press/release | `mouse_down` + `mouse_pressed` / `mouse_released` |
| `MouseWheel` | `scroll_delta` (line → умножь на ~40) |
| resize | `viewport` |
| текст | `text` (символы за кадр) |
| Backspace / стрелки / Home / End / Enter | `key_*` |
| Ctrl+C/V/X/A | `key_copy` / `paste` / `cut` / `select_all` |
| Shift / Ctrl | `key_shift` / `key_ctrl` |

Курсор из `UiOutput.cursor` → `winit::window::CursorIcon`  
(не ставь `Default` каждый кадр без проверки — собьёшь OS-курсор ресайза окна).

Clipboard: в `UiInput.clipboard` кладёшь текст при paste; из `UiOutput.clipboard` пишешь в систему при copy/cut.

---

## Виджеты (кратко)

```rust
ui.window(Window::new("Settings").pos(p).size(s).resizable(true), |ui| {
    ui.label("Hello");
    if ui.button("OK").clicked() {}
    ui.checkbox("Enabled", &mut on);
    ui.slider("vol", &mut vol, 0.0..=1.0);
    ui.text_input("name", &mut name);
    ui.text_area("notes", &mut notes, Vec2::new(0.0, 80.0));
    ui.select("mode", &mut mode, &["A", "B"]);
    ui.separator();
    ui.scroll_area("list", size, ScrollAxes::Vertical, |ui| { /* … */ });
    ui.add_enabled(false, |ui| { ui.button("Locked"); });
});

// docking
ui.dock_space("main", viewport, &mut dock, |ui, tab| match tab {
    "Viewport" => ui.texture(1, ui.available_size()),
    "Inspector" => { /* … */ }
    _ => {}
});
```

ID иерархические: одинаковые локальные имена в разных окнах не конфликтуют.

---

## Чеклист нового проекта на wgpu

1. Добавить `mega-ui` в `Cargo.toml`
2. Скопировать из `engine` (или написать заново): UI shader, pipeline, font texture, `draw_ui`
3. Пробросить мышь/клаву в `UiInput`
4. Каждый кадр: `begin_frame` → описание UI → `end_frame` → sync atlas → draw
5. Свои картинки/сцену — через текстурные слоты + `ui.image` / `ui.texture`

Кастомизация: шейдер, число слотов, свой windowing, свой порядок pass’ов — всё на твоей стороне. Либа только считает layout и выдаёт quads.
