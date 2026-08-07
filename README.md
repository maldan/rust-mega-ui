# mega-ui

Immediate/retained UI для игровых движков. **Сама ничего не рисует на GPU** — выдаёт список `DrawCommand`, а ты уже рисуешь их как хочешь (wgpu, OpenGL, …).

Зависимости core: `glam`, `fontdue`, `resvg` (SVG → иконки в атласе).  
Опционально `features = ["wgpu"]` — pipeline helpers (`mega_ui::wgpu::UiRenderer`, шейдер kinds 0–3, wgpu 30).  
winit остаётся только в `examples/` (dev-dependencies).

Тема сейчас одна (тёмная) и зашита внутри crate (`theme` — `pub(crate)`). Публичного API кастомизации цветов пока нет.

---

## Идея за 30 секунд

```text
каждый кадр:
  события окна  →  UiInput
  ui.begin_frame(input)
  ui.button / window / dock / …   // описание UI
  output = ui.end_frame()
  output.draw_list  →  твой рендерер
  output.cursor / clipboard / needs_repaint / want_capture_*  →  окно / игра
```

Состояние виджетов (скролл, фокус, open у select) хранит `Ui`.  
Данные приложения (`&mut String`, `&mut f32`, …) хранишь ты.

---

## Подключение

```toml
[dependencies]
mega-ui = { path = "../path/to/mega-ui" }
glam = "0.33"

# опционально: готовый wgpu-рендерер UI
# mega-ui = { path = "../path/to/mega-ui", features = ["wgpu"] }
```

```rust
use mega_ui::{Ui, UiInput, DrawCommand, CursorIcon};
```

Локальные демки (winit + feature `wgpu`):

```bash
cargo run --example demo --features wgpu
cargo run --example demo_dock --features wgpu
```

- `mega_ui::wgpu::UiRenderer` — pipeline, атласы, batch draw (`src/wgpu/`)
- `examples/framework.rs` — тонкий winit host вокруг `UiRenderer` (`Scene`, `Host::run`)

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
    mouse_right_down,
    mouse_right_pressed,
    mouse_right_released,
    viewport: Vec2::new(width, height),
    scroll_delta,    // пиксели; +y = колесо вверх
    dt,              // секунды
    text: typed_chars,
    key_backspace: …,
    // остальные key_* — по необходимости (текст, clipboard)
    clipboard: paste_text, // если key_paste
    ..Default::default()
};

ui.set_scale(1.0); // 2.0 = UI в 2 раза крупнее (виджеты + окна)
ui.begin_frame(input);

if ui.button("Save").clicked() { /* … */ }
ui.text_input("name", &mut name);

let out = ui.end_frame();

// 2) курсор
window.set_cursor(map_cursor(out.cursor));

// 3) clipboard out (copy/cut)
if let Some(text) = out.clipboard { clipboard.set_text(text); }

// 4) если out.needs_repaint — запроси ещё один кадр (плавный скролл, анимации)

// 5) want_capture_* — UI «съел» ввод; игру можно не крутить
// 6) нарисуй out.draw_list
```

`mouse_pressed` / `mouse_released` / `mouse_right_*` и `key_*` должны жить **один кадр** — после передачи в UI сбрасывай флаги.

---

## Что такое DrawCommand

Каждая команда = один textured/colored quad:

| поле | смысл |
|------|--------|
| `rect` | экранные пиксели (origin = top-left) |
| `uv_min` / `uv_max` | UV в атласе; для solid `uv_min == uv_max` (белый тексель). Для SDF round — локальные UV контента |
| `colors` | RGBA 0..1 на углах: TL, TR, BR, BL (solid = один цвет во всех) |
| `kind` | `0` = font atlas, `1` = host texture, `2` = SDF round rect, `3` = SDF line |
| `tex` | слот хост-текстуры при `kind == 1`. Хост батчит по слоту и ребиндит один `tex0` |
| `params` | `kind 2`: `[w, h, radius, corners]` (0=all, 1=top, 2=bottom); `kind 3`: `[ax, ay, bx, by]` px, thickness в `uv_min.x` |

Как виджеты мапятся на kinds:

- заливка / скругления → `kind = 2` (SDF) или solid через font atlas
- текст / иконки → `kind = 0`, UV в font atlas
- `ui.image(size)` → `kind = 1`, `tex = 0`
- `ui.texture(slot, size)` → `kind = 1`, `tex = slot` (сцена, превью, …)
- color picker SV → `kind = 1`, `tex = TEX_SLOT_COLOR_SV` (атлас из `ui.color_sv_atlas()`)
- линии (plot, separators) → `kind = 3`

UI **не знает**, что лежит в слоте. Ты биндишь свои `TextureView` сам. В демо — один шейдерный `tex0`, смена слота = новый draw batch (см. `examples/framework.rs`).

---

## wgpu feature (`UiRenderer`)

Включи `features = ["wgpu"]` — либа отдаёт готовый рендер draw-list’а (wgpu **30**):

```rust
use mega_ui::wgpu::UiRenderer;

let mut renderer = UiRenderer::new(&device, &queue, surface_format, &ui);
renderer.set_viewport(&queue, width as f32, height as f32);

// каждый кадр после end_frame:
renderer.sync_atlases(&device, &queue, &mut ui);
// … begin_render_pass …
let stats = renderer.draw(&queue, &mut pass, &out.draw_list);
```

Ещё:

- `set_texture_rgba(slot, pixels, w, h, …)` — залить свой RGBA в слот
- `bind_texture_view(slot, view)` — привязать внешний view (сцена / offscreen)
- `prepare` / `render` — если upload и draw в разных местах

Окно, surface, `UiInput`, cursor/clipboard — по-прежнему на твоей стороне (или смотри `examples/framework.rs`).

---

## wgpu: что нужно сделать вручную (без feature)

Если feature не используешь — пишешь pipeline сам. Ниже — контракт, который реализует `UiRenderer`.

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

Если используешь `color_edit`, так же синхронизируй SV-атлас:

```rust
if ui.color_sv_atlas_take_dirty() {
    let (pixels, w, h) = ui.color_sv_atlas(); // RGBA
    // залей в слот TEX_SLOT_COLOR_SV
}
```

Шрифт по умолчанию: Segoe UI (Windows). Свой:

```rust
ui.set_font_bytes(include_bytes!("MyFont.ttf"), 14.0)?;
// или ui.set_font_path("fonts/MyFont.ttf", 14.0)?;
```

### 2. Вершины

На каждый `DrawCommand` — 2 треугольника (6 вершин). Атрибуты как в демо:

```text
pos:    vec2   // пиксели
uv:     vec2
color:  vec4
kind:   f32
tex:    f32    // номер слота (для host; в FS не обязан читаться)
params: vec4   // SDF round / line
```

Viewport uniform: размер окна → в VS перевод в clip space (Y вниз как в UI).

### 3. Шейдер (логика)

См. `src/wgpu/ui.wgsl` (тот же шейдер, что внутри `UiRenderer`):

```text
kind 0 → sample font atlas (.r = alpha), color * alpha
kind 1 → sample tex0 (RGBA) * color; хост ребиндит tex0 между батчами
kind 2 → SDF rounded rect из params + uv
kind 3 → SDF line из params + thickness (uv.x)
```

Сэмплер: linear, clamp. Blending: обычный alpha (`src_alpha`, `one_minus_src_alpha`).

### 4. Bind group (как в демо)

Один layout на все батчи:

```text
0  uniform  viewport
1  texture  font atlas (R8)
2  sampler
3  texture  tex0  (текущий host-слот; placeholder, если batch без kind=1)
```

Хост режет `draw_list` на батчи при смене `DrawCommand.tex` у `kind ≈ 1` и ставит соответствующий bind group. Слоты (`ui.image` → 0, сцена → 1, `TEX_SLOT_COLOR_SV`, …) — просто разные `TextureView` под одним binding.

Альтернатива — несколько текстурных биндингов сразу; главное, чтобы `DrawCommand.tex` совпадал с тем, что ты биндишь.

### 5. Pass

1. (опционально) нарисуй 3D/игру в offscreen, если показываешь через `ui.texture`
2. sync font atlas (+ color SV, если нужен)
3. залей vertex buffer из `draw_list`
4. UI render pass поверх swapchain (или в свой target), батчами по `tex`

Порядок в `draw_list` уже правильный (окна, оверлеи). Клиппинг заложен в сами rect’ы команд.

---

## Ввод (winit → UiInput)

Минимальный набор:

| событие | поле |
|---------|------|
| `CursorMoved` | `mouse_pos` |
| ЛКМ press/release | `mouse_down` + `mouse_pressed` / `mouse_released` |
| ПКМ press/release | `mouse_right_down` + `mouse_right_pressed` / `mouse_right_released` (context menu) |
| `MouseWheel` | `scroll_delta` (line → умножь на ~40) |
| resize | `viewport` |
| текст / IME | `text` (символы за кадр) |
| Backspace / стрелки / Home / End / Enter | `key_*` |
| Ctrl+C/V/X/A | `key_copy` / `paste` / `cut` / `select_all` |
| Shift / Ctrl | `key_shift` / `key_ctrl` (на macOS Cmd обычно мапится в `key_ctrl`) |

Курсор из `UiOutput.cursor` → `winit::window::CursorIcon`  
(не ставь `Default` каждый кадр без проверки — собьёшь OS-курсор ресайза окна).

Clipboard: в `UiInput.clipboard` кладёшь текст при paste; из `UiOutput.clipboard` пишешь в систему при copy/cut.

`UiOutput.want_capture_mouse` / `want_capture_keyboard` — UI обработал ввод (клик по виджету, фокус в тексте и т.п.); игру можно не крутить.

Рабочий маппинг событий → `UiInput` есть в `examples/framework.rs` (`FrameInput`).

---

## Виджеты и layout

```rust
ui.load_builtin_icons(); // once at startup
// or: ui.load_icons([("my_icon", include_bytes!("my.svg"))]);

ui.icon("folder", 18.0);
ui.menu_item_icon("file", "Open…");

ui.modal(Window::new("Confirm").size(s).open(&mut show), |ui| {
    ui.label("Are you sure?");
    if ui.button("OK").clicked() { ui.close_modal(); }
});

ui.menu_bar(|ui| {
    ui.menu("File", |ui| {
        if ui.menu_item("New").clicked() {}
        if ui.menu_item("Open…").clicked() {}
        ui.menu("Open Recent", |ui| {
            if ui.menu_item("a.mega").clicked() {}
        });
        ui.menu_separator();
        if ui.menu_item("Exit").clicked() {}
    });
});

ui.window(Window::new("Settings").pos(p).size(s).resizable(true), |ui| {
    // pos/size — UI points (экран = points × scale)
    ui.label("Hello");
    if ui.button("OK").clicked() {}
    ui.checkbox("Enabled", &mut on);
    ui.toggle("mode", &mut mode, &["A", "B"]);
    ui.slider("vol", &mut vol, 0.0..=1.0);
    ui.drag_float("x", &mut x, 0.1);
    ui.vec2("pos", &mut pos, 0.1, Vec2::ZERO);
    ui.color_edit("tint", &mut color);
    ui.text_input("name", &mut name);
    ui.text_area("notes", &mut notes, Vec2::new(0.0, 80.0));
    ui.select("mode", &mut mode, &["A", "B"]);
    ui.progress_bar(0.4);
    ui.separator();
    ui.scroll_area("list", size, ScrollAxes::Vertical, |ui| { /* … */ });
    ui.row(|ui| {
        ui.label("Name");
        ui.flex(1.0, |ui| { ui.text_input("n", &mut name); });
        ui.button("OK");
    });
    ui.property("Volume", 0.35, |ui| { ui.slider("v", &mut vol, 0.0..=1.0); });
    ui.grid(3, |ui| {
        ui.grid_cell(|ui| { ui.knob("k", &mut v, 0.0..=1.0); });
    });
    ui.tabs("main", &["Basics", "Plot"], |ui, tab| match tab {
        0 => { /* … */ }
        _ => {
            ui.plot(Vec2::new(0.0, 100.0), &values);
            // или: ui.plot_with_view("wave", size, &values, &view);
        }
    });
    ui.curve_editor("ease", &mut curve, Vec2::new(0.0, 140.0));
    ui.table("files", &cols, |ui| {
        ui.table_row(|ui| {
            ui.table_cell(|ui| { ui.label("a.txt"); });
        });
    });
    ui.tree_node("src", "src", |ui| {
        ui.tree_leaf_icon("main", "file", "main.rs");
    });
    ui.browser("assets", size, &items);
    ui.notify_success("Saved");
    ui.add_enabled(false, |ui| { ui.button("Locked"); });
});

ui.status_bar(|ui| {
    ui.label("Ready");
});

// docking
ui.dock_space("main", viewport, &mut dock, |ui, tab| match tab {
    "Viewport" => ui.texture(1, ui.available_size()),
    "Inspector" => { /* … */ }
    _ => {}
});

// ПКМ-меню над зоной
ui.context_menu("ctx", hovered, |ui| {
    if ui.menu_item("Delete").clicked() {}
});
```

ID иерархические: одинаковые локальные имена в разных окнах не конфликтуют (`id_scope`).

### Обзор API

| Группа | Методы |
|--------|--------|
| Кадр | `begin_frame`, `end_frame`, `set_scale`, `request_repaint`, `input_debug` |
| Layout | `row` / `column` / `*_with`, `flex`, `property`, `grid` / `grid_cell`, `space`, `spacer`, `same_line` |
| Chrome | `window`, `modal`, `menu_bar` / `menu` / `menu_item*`, `context_menu`, `status_bar`, `dock_space` |
| Controls | `button` / `button_with`, `checkbox`, `toggle`, `slider`, `knob`, `drag_float`, `vec2` / `vec3`, `select`, `text_input`, `text_area`, `color_edit`, `progress_bar` |
| Data | `table` / `table_row` / `table_cell`, `tree_*`, `browser`, `tabs`, `collapsing_header`, `group`, `scroll_area` |
| Viz | `plot` / `plot_with_view`, `curve_editor`, `image` / `texture`, `color_box`, `line` |
| Feedback | `notify` / `notify_success` / `notify_warn` / `notify_error`, `label` / `label_styled` |
| Icons | `load_builtin_icons`, `load_icons`, `icon` / `icon_colored` |

Builtin icons (`load_builtin_icons`):  
`folder`, `folder_open`, `file`, `close`, `delete`, `plus`, `chevron_{left,right,up,down}`, `check`, `lock`, `unlock`, `reset`, `refresh`, `more_vert`, `save`, `search`, `settings`, `undo`, `redo`, `edit`, `copy`, `warning`, `info`, `grid`.

---

## Чеклист нового проекта на wgpu

1. Добавить `mega-ui` с `features = ["wgpu"]`
2. Создать `UiRenderer::new` после device/format; на resize — `set_viewport`
3. Пробросить мышь (включая ПКМ) / клаву / clipboard в `UiInput`
4. Каждый кадр: `begin_frame` → UI → `end_frame` → `sync_atlases` → `draw` в pass
5. Свои картинки/сцену — `set_texture_rgba` / `bind_texture_view` + `ui.image` / `ui.texture`
6. Учитывать `want_capture_*`, `needs_repaint`, `cursor`, `clipboard` из `UiOutput`

Без feature: тот же контракт DrawCommand, но pipeline/шейдер/атласы пишешь сам.
