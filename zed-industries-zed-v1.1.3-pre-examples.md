# Code Examples for zed-industries-zed (Version: v1.1.3-pre)

## `example_file:crates/gpui/examples/active_state_bug.rs`

```rust
/// Click the button — the `.active()` background gets stuck on every other click.
use gpui::*;
use gpui_platform::application;

struct Example;

impl Render for Example {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Colors from Zed's default dark theme
        let bg = hsla(215. / 360., 0.12, 0.15, 1.);
        let text = hsla(221. / 360., 0.11, 0.86, 1.);
        let hover = hsla(225. / 360., 0.118, 0.267, 1.);
        let active = hsla(220. / 360., 0.118, 0.20, 1.);

        div().bg(bg).size_full().p_1().child(
            div()
                .id("button")
                .px_2()
                .py_0p5()
                .rounded_md()
                .text_sm()
                .text_color(text)
                .hover(|s| s.bg(hover))
                .active(|s| s.bg(active))
                .on_click(|_, _, _| {})
                .child("Click me"),
        )
    }
}

fn main() {
    application().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(200.), px(60.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Example),
        )
        .unwrap();
        cx.activate(true);
    });
}

```
---
## `example_file:crates/gpui/examples/anchor.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    Anchor, AnchoredPositionMode, App, Axis, Bounds, Context, Half as _, InteractiveElement,
    ParentElement, Pixels, Point, Render, SharedString, Size, Window, WindowBounds, WindowOptions,
    anchored, deferred, div, point, prelude::*, px, rgb, size,
};
use gpui_platform::application;

struct AnchorDemo {
    hovered_button: Option<usize>,
}

struct ButtonDemo {
    label: SharedString,
    corner: Option<Anchor>,
}

fn resolved_position(corner: Anchor, button_size: Size<Pixels>) -> Point<Pixels> {
    let offset = Point {
        x: px(0.),
        y: -button_size.height,
    };

    offset
        + match corner.other_side_along(Axis::Vertical) {
            Anchor::TopLeft => point(px(0.0), px(0.0)),
            Anchor::TopCenter => point(button_size.width.half(), px(0.0)),
            Anchor::TopRight => point(button_size.width, px(0.0)),
            Anchor::LeftCenter => point(button_size.width, button_size.height.half()),
            Anchor::RightCenter => point(px(0.), button_size.height.half()),
            Anchor::BottomLeft => point(px(0.0), button_size.height),
            Anchor::BottomCenter => point(button_size.width / 2.0, button_size.height),
            Anchor::BottomRight => point(button_size.width, button_size.height),
        }
}

impl AnchorDemo {
    fn buttons() -> Vec<ButtonDemo> {
        vec![
            ButtonDemo {
                label: "TopLeft".into(),
                corner: Some(Anchor::TopLeft),
            },
            ButtonDemo {
                label: "TopCenter".into(),
                corner: Some(Anchor::TopCenter),
            },
            ButtonDemo {
                label: "TopRight".into(),
                corner: Some(Anchor::TopRight),
            },
            ButtonDemo {
                label: "LeftCenter".into(),
                corner: Some(Anchor::LeftCenter),
            },
            ButtonDemo {
                label: "Center".into(),
                corner: None,
            },
            ButtonDemo {
                label: "RightCenter".into(),
                corner: Some(Anchor::RightCenter),
            },
            ButtonDemo {
                label: "BottomLeft".into(),
                corner: Some(Anchor::BottomLeft),
            },
            ButtonDemo {
                label: "BottomCenter".into(),
                corner: Some(Anchor::BottomCenter),
            },
            ButtonDemo {
                label: "BottomRight".into(),
                corner: Some(Anchor::BottomRight),
            },
        ]
    }
}

impl Render for AnchorDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let buttons = Self::buttons();
        let button_size = size(px(120.0), px(65.0));

        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .bg(gpui::white())
            .gap_4()
            .p_10()
            .child("Popover with Anchor")
            .child(
                div()
                    .size_128()
                    .grid()
                    .grid_cols(3)
                    .gap_6()
                    .relative()
                    .children(buttons.iter().enumerate().map(|(index, button)| {
                        let is_hovered = self.hovered_button == Some(index);
                        let is_hoverable = button.corner.is_some();
                        div()
                            .relative()
                            .child(
                                div()
                                    .id(("button", index))
                                    .w(button_size.width)
                                    .h(button_size.height)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(gpui::white())
                                    .when(is_hoverable, |this| {
                                        this.border_1()
                                            .rounded_lg()
                                            .border_color(gpui::black())
                                            .hover(|style| {
                                                style.bg(gpui::black()).text_color(gpui::white())
                                            })
                                            .on_hover(cx.listener(
                                                move |this, hovered, _window, cx| {
                                                    if *hovered {
                                                        this.hovered_button = Some(index);
                                                    } else if this.hovered_button == Some(index) {
                                                        this.hovered_button = None;
                                                    }
                                                    cx.notify();
                                                },
                                            ))
                                            .child(button.label.clone())
                                    }),
                            )
                            .when_some(self.hovered_button.filter(|_| is_hovered), |this, index| {
                                let button = &buttons[index];
                                let Some(corner) = button.corner else {
                                    return this;
                                };

                                let position = resolved_position(corner, button_size);
                                this.child(deferred(
                                    anchored()
                                        .anchor(corner)
                                        .position(position)
                                        .position_mode(AnchoredPositionMode::Local)
                                        .snap_to_window()
                                        .child(
                                            div()
                                                .py_0p5()
                                                .px_2()
                                                .bg(gpui::black().opacity(0.75))
                                                .text_color(rgb(0xffffff))
                                                .rounded_sm()
                                                .shadow_sm()
                                                .min_w(px(100.0))
                                                .text_sm()
                                                .child(button.label.clone()),
                                        ),
                                ))
                            })
                    })),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(750.), px(600.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| AnchorDemo {
                    hovered_button: None,
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/animation.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use std::time::Duration;

use anyhow::Result;
use gpui::{
    Animation, AnimationExt as _, App, AssetSource, Bounds, Context, SharedString, Transformation,
    Window, WindowBounds, WindowOptions, bounce, div, ease_in_out, percentage, prelude::*, px,
    size, svg,
};
use gpui_platform::application;

struct Assets {}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        std::fs::read(path)
            .map(Into::into)
            .map_err(Into::into)
            .map(Some)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(std::fs::read_dir(path)?
            .filter_map(|entry| {
                Some(SharedString::from(
                    entry.ok()?.path().to_string_lossy().into_owned(),
                ))
            })
            .collect::<Vec<_>>())
    }
}

const ARROW_CIRCLE_SVG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/image/arrow_circle.svg"
);

struct AnimationExample {}

impl Render for AnimationExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(gpui::white())
            .text_color(gpui::black())
            .justify_around()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .justify_around()
                    .child(
                        div()
                            .id("content")
                            .flex()
                            .flex_col()
                            .h(px(150.))
                            .overflow_y_scroll()
                            .w_full()
                            .flex_1()
                            .justify_center()
                            .items_center()
                            .text_xl()
                            .gap_4()
                            .child("Hello Animation")
                            .child(
                                svg()
                                    .size_20()
                                    .overflow_hidden()
                                    .path(ARROW_CIRCLE_SVG)
                                    .text_color(gpui::black())
                                    .with_animation(
                                        "image_circle",
                                        Animation::new(Duration::from_secs(2))
                                            .repeat()
                                            .with_easing(bounce(ease_in_out)),
                                        |svg, delta| {
                                            svg.with_transformation(Transformation::rotate(
                                                percentage(delta),
                                            ))
                                        },
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .h(px(64.))
                            .w_full()
                            .p_2()
                            .justify_center()
                            .items_center()
                            .border_t_1()
                            .border_color(gpui::black().opacity(0.1))
                            .bg(gpui::black().opacity(0.05))
                            .child("Other Panel"),
                    ),
            )
    }
}

fn run_example() {
    application().with_assets(Assets {}).run(|cx: &mut App| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(300.), px(300.)),
                cx,
            ))),
            ..Default::default()
        };
        cx.open_window(options, |_, cx| {
            cx.activate(false);
            cx.new(|_| AnimationExample {})
        })
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/data_table.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use std::{ops::Range, rc::Rc, time::Duration};

use gpui::{
    App, Bounds, Context, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Render,
    SharedString, UniformListScrollHandle, Window, WindowBounds, WindowOptions, canvas, div, point,
    prelude::*, px, rgb, size, uniform_list,
};
use gpui_platform::application;

const TOTAL_ITEMS: usize = 10000;
const SCROLLBAR_THUMB_WIDTH: Pixels = px(8.);
const SCROLLBAR_THUMB_HEIGHT: Pixels = px(100.);

pub struct Quote {
    name: SharedString,
    symbol: SharedString,
    last_done: f64,
    prev_close: f64,
    open: f64,
    high: f64,
    low: f64,
    timestamp: Duration,
    volume: i64,
    turnover: f64,
    ttm: f64,
    market_cap: f64,
    float_cap: f64,
    shares: f64,
    pb: f64,
    pe: f64,
    eps: f64,
    dividend: f64,
    dividend_yield: f64,
    dividend_per_share: f64,
    dividend_date: SharedString,
    dividend_payment: f64,
}

impl Quote {
    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        // simulate a base price in a realistic range
        let prev_close = rng.random_range(100.0..200.0);
        let change = rng.random_range(-5.0..5.0);
        let last_done = prev_close + change;
        let open = prev_close + rng.random_range(-3.0..3.0);
        let high = (prev_close + rng.random_range::<f64, _>(0.0..10.0)).max(open);
        let low = (prev_close - rng.random_range::<f64, _>(0.0..10.0)).min(open);
        let timestamp = Duration::from_secs(rng.random_range(0..86400));
        let volume = rng.random_range(1_000_000..100_000_000);
        let turnover = last_done * volume as f64;
        let symbol = {
            let mut ticker = String::new();
            if rng.random_bool(0.5) {
                ticker.push_str(&format!(
                    "{:03}.{}",
                    rng.random_range(100..1000),
                    rng.random_range(0..10)
                ));
            } else {
                ticker.push_str(&format!(
                    "{}{}",
                    rng.random_range('A'..='Z'),
                    rng.random_range('A'..='Z')
                ));
            }
            ticker.push_str(&format!(".{}", rng.random_range('A'..='Z')));
            ticker
        };
        let name = format!(
            "{} {} - #{}",
            symbol,
            rng.random_range(1..100),
            rng.random_range(10000..100000)
        );
        let ttm = rng.random_range(0.0..10.0);
        let market_cap = rng.random_range(1_000_000.0..10_000_000.0);
        let float_cap = market_cap + rng.random_range(1_000.0..10_000.0);
        let shares = rng.random_range(100.0..1000.0);
        let pb = market_cap / shares;
        let pe = market_cap / shares;
        let eps = market_cap / shares;
        let dividend = rng.random_range(0.0..10.0);
        let dividend_yield = rng.random_range(0.0..10.0);
        let dividend_per_share = rng.random_range(0.0..10.0);
        let dividend_date = SharedString::new(format!(
            "{}-{}-{}",
            rng.random_range(2000..2023),
            rng.random_range(1..12),
            rng.random_range(1..28)
        ));
        let dividend_payment = rng.random_range(0.0..10.0);

        Self {
            name: name.into(),
            symbol: symbol.into(),
            last_done,
            prev_close,
            open,
            high,
            low,
            timestamp,
            volume,
            turnover,
            pb,
            pe,
            eps,
            ttm,
            market_cap,
            float_cap,
            shares,
            dividend,
            dividend_yield,
            dividend_per_share,
            dividend_date,
            dividend_payment,
        }
    }

    fn change(&self) -> f64 {
        (self.last_done - self.prev_close) / self.prev_close * 100.0
    }

    fn change_color(&self) -> gpui::Hsla {
        if self.change() > 0.0 {
            gpui::green()
        } else {
            gpui::red()
        }
    }

    fn turnover_ratio(&self) -> f64 {
        self.volume as f64 / self.turnover * 100.0
    }
}

#[derive(IntoElement)]
struct TableRow {
    ix: usize,
    quote: Rc<Quote>,
}
impl TableRow {
    fn new(ix: usize, quote: Rc<Quote>) -> Self {
        Self { ix, quote }
    }

    fn render_cell(&self, key: &str, width: Pixels, color: gpui::Hsla) -> impl IntoElement {
        div()
            .whitespace_nowrap()
            .truncate()
            .w(width)
            .px_1()
            .child(match key {
                "id" => div().child(format!("{}", self.ix)),
                "symbol" => div().child(self.quote.symbol.clone()),
                "name" => div().child(self.quote.name.clone()),
                "last_done" => div()
                    .text_color(color)
                    .child(format!("{:.3}", self.quote.last_done)),
                "prev_close" => div()
                    .text_color(color)
                    .child(format!("{:.3}", self.quote.prev_close)),
                "change" => div()
                    .text_color(color)
                    .child(format!("{:.2}%", self.quote.change())),
                "timestamp" => div()
                    .text_color(color)
                    .child(format!("{:?}", self.quote.timestamp.as_secs())),
                "open" => div()
                    .text_color(color)
                    .child(format!("{:.2}", self.quote.open)),
                "low" => div()
                    .text_color(color)
                    .child(format!("{:.2}", self.quote.low)),
                "high" => div()
                    .text_color(color)
                    .child(format!("{:.2}", self.quote.high)),
                "ttm" => div()
                    .text_color(color)
                    .child(format!("{:.2}", self.quote.ttm)),
                "eps" => div()
                    .text_color(color)
                    .child(format!("{:.2}", self.quote.eps)),
                "market_cap" => {
                    div().child(format!("{:.2} M", self.quote.market_cap / 1_000_000.0))
                }
                "float_cap" => div().child(format!("{:.2} M", self.quote.float_cap / 1_000_000.0)),
                "turnover" => div().child(format!("{:.2} M", self.quote.turnover / 1_000_000.0)),
                "volume" => div().child(format!("{:.2} M", self.quote.volume as f64 / 1_000_000.0)),
                "turnover_ratio" => div().child(format!("{:.2}%", self.quote.turnover_ratio())),
                "pe" => div().child(format!("{:.2}", self.quote.pe)),
                "pb" => div().child(format!("{:.2}", self.quote.pb)),
                "shares" => div().child(format!("{:.2}", self.quote.shares)),
                "dividend" => div().child(format!("{:.2}", self.quote.dividend)),
                "yield" => div().child(format!("{:.2}%", self.quote.dividend_yield)),
                "dividend_per_share" => {
                    div().child(format!("{:.2}", self.quote.dividend_per_share))
                }
                "dividend_date" => div().child(format!("{}", self.quote.dividend_date)),
                "dividend_payment" => div().child(format!("{:.2}", self.quote.dividend_payment)),
                _ => div().child("--"),
            })
    }
}

const FIELDS: [(&str, f32); 24] = [
    ("id", 64.),
    ("symbol", 64.),
    ("name", 180.),
    ("last_done", 80.),
    ("prev_close", 80.),
    ("open", 80.),
    ("low", 80.),
    ("high", 80.),
    ("ttm", 50.),
    ("market_cap", 96.),
    ("float_cap", 96.),
    ("turnover", 120.),
    ("volume", 100.),
    ("turnover_ratio", 96.),
    ("pe", 64.),
    ("pb", 64.),
    ("eps", 64.),
    ("shares", 96.),
    ("dividend", 64.),
    ("yield", 64.),
    ("dividend_per_share", 64.),
    ("dividend_date", 96.),
    ("dividend_payment", 64.),
    ("timestamp", 120.),
];

impl RenderOnce for TableRow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let color = self.quote.change_color();
        div()
            .flex()
            .flex_row()
            .border_b_1()
            .border_color(rgb(0xE0E0E0))
            .bg(if self.ix.is_multiple_of(2) {
                rgb(0xFFFFFF)
            } else {
                rgb(0xFAFAFA)
            })
            .py_0p5()
            .px_2()
            .children(FIELDS.map(|(key, width)| self.render_cell(key, px(width), color)))
    }
}

struct DataTable {
    /// Use `Rc` to share the same quote data across multiple items, avoid cloning.
    quotes: Vec<Rc<Quote>>,
    visible_range: Range<usize>,
    scroll_handle: UniformListScrollHandle,
    /// The position in thumb bounds when dragging start mouse down.
    drag_position: Option<Point<Pixels>>,
}

impl DataTable {
    fn new() -> Self {
        Self {
            quotes: Vec::new(),
            visible_range: 0..0,
            scroll_handle: UniformListScrollHandle::new(),
            drag_position: None,
        }
    }

    fn generate(&mut self) {
        self.quotes = (0..TOTAL_ITEMS).map(|_| Rc::new(Quote::random())).collect();
    }

    fn table_bounds(&self) -> Bounds<Pixels> {
        self.scroll_handle.0.borrow().base_handle.bounds()
    }

    fn scroll_top(&self) -> Pixels {
        self.scroll_handle.0.borrow().base_handle.offset().y
    }

    fn scroll_height(&self) -> Pixels {
        self.scroll_handle
            .0
            .borrow()
            .last_item_size
            .unwrap_or_default()
            .contents
            .height
    }

    fn render_scrollbar(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scroll_height = self.scroll_height();
        let table_bounds = self.table_bounds();
        let table_height = table_bounds.size.height;
        if table_height == px(0.) {
            return div().id("scrollbar");
        }

        let percentage = -self.scroll_top() / scroll_height;
        let offset_top = (table_height * percentage).clamp(
            px(4.),
            (table_height - SCROLLBAR_THUMB_HEIGHT - px(4.)).max(px(4.)),
        );
        let entity = cx.entity();
        let scroll_handle = self.scroll_handle.0.borrow().base_handle.clone();

        div()
            .id("scrollbar")
            .absolute()
            .top(offset_top)
            .right_1()
            .h(SCROLLBAR_THUMB_HEIGHT)
            .w(SCROLLBAR_THUMB_WIDTH)
            .bg(rgb(0xC0C0C0))
            .hover(|this| this.bg(rgb(0xA0A0A0)))
            .rounded_lg()
            .child(
                canvas(
                    |_, _, _| (),
                    move |thumb_bounds, _, window, _| {
                        window.on_mouse_event({
                            let entity = entity.clone();
                            move |ev: &MouseDownEvent, _, _, cx| {
                                if !thumb_bounds.contains(&ev.position) {
                                    return;
                                }

                                entity.update(cx, |this, _| {
                                    this.drag_position = Some(
                                        ev.position - thumb_bounds.origin - table_bounds.origin,
                                    );
                                })
                            }
                        });
                        window.on_mouse_event({
                            let entity = entity.clone();
                            move |_: &MouseUpEvent, _, _, cx| {
                                entity.update(cx, |this, _| {
                                    this.drag_position = None;
                                })
                            }
                        });

                        window.on_mouse_event(move |ev: &MouseMoveEvent, _, _, cx| {
                            if !ev.dragging() {
                                return;
                            }

                            let Some(drag_pos) = entity.read(cx).drag_position else {
                                return;
                            };

                            let inside_offset = drag_pos.y;
                            let percentage = ((ev.position.y - table_bounds.origin.y
                                + inside_offset)
                                / (table_bounds.size.height))
                                .clamp(0., 1.);

                            let offset_y = ((scroll_height - table_bounds.size.height)
                                * percentage)
                                .clamp(px(0.), scroll_height - SCROLLBAR_THUMB_HEIGHT);
                            scroll_handle.set_offset(point(px(0.), -offset_y));
                            cx.notify(entity.entity_id());
                        })
                    },
                )
                .size_full(),
            )
    }
}

impl Render for DataTable {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .bg(gpui::white())
            .text_sm()
            .size_full()
            .p_4()
            .gap_2()
            .flex()
            .flex_col()
            .child(format!(
                "Total {} items, visible range: {:?}",
                self.quotes.len(),
                self.visible_range
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_hidden()
                    .border_1()
                    .border_color(rgb(0xE0E0E0))
                    .rounded_sm()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .w_full()
                            .overflow_hidden()
                            .border_b_1()
                            .border_color(rgb(0xE0E0E0))
                            .text_color(rgb(0x555555))
                            .bg(rgb(0xF0F0F0))
                            .py_1()
                            .px_2()
                            .text_xs()
                            .children(FIELDS.map(|(key, width)| {
                                div()
                                    .whitespace_nowrap()
                                    .flex_shrink_0()
                                    .truncate()
                                    .px_1()
                                    .w(px(width))
                                    .child(key.replace("_", " ").to_uppercase())
                            })),
                    )
                    .child(
                        div()
                            .relative()
                            .size_full()
                            .child(
                                uniform_list(
                                    "items",
                                    self.quotes.len(),
                                    cx.processor(move |this, range: Range<usize>, _, _| {
                                        this.visible_range = range.clone();
                                        let mut items = Vec::with_capacity(range.end - range.start);
                                        for i in range {
                                            if let Some(quote) = this.quotes.get(i) {
                                                items.push(TableRow::new(i, quote.clone()));
                                            }
                                        }
                                        items
                                    }),
                                )
                                .size_full()
                                .track_scroll(&self.scroll_handle),
                            )
                            .child(self.render_scrollbar(window, cx)),
                    ),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                focus: true,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1280.0), px(1000.0)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| {
                    let mut table = DataTable::new();
                    table.generate();
                    table
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/drag_drop.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, Half, Hsla, Pixels, Point, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, rgb, size,
};
use gpui_platform::application;

#[derive(Clone, Copy)]
struct DragInfo {
    ix: usize,
    color: Hsla,
    position: Point<Pixels>,
}

impl DragInfo {
    fn new(ix: usize, color: Hsla) -> Self {
        Self {
            ix,
            color,
            position: Point::default(),
        }
    }

    fn position(mut self, pos: Point<Pixels>) -> Self {
        self.position = pos;
        self
    }
}

impl Render for DragInfo {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let size = gpui::size(px(120.), px(50.));

        div()
            .pl(self.position.x - size.width.half())
            .pt(self.position.y - size.height.half())
            .child(
                div()
                    .flex()
                    .justify_center()
                    .items_center()
                    .w(size.width)
                    .h(size.height)
                    .bg(self.color.opacity(0.5))
                    .text_color(gpui::white())
                    .text_xs()
                    .shadow_md()
                    .child(format!("Item {}", self.ix)),
            )
    }
}

struct DragDrop {
    drop_on: Option<DragInfo>,
}

impl DragDrop {
    fn new() -> Self {
        Self { drop_on: None }
    }
}

impl Render for DragDrop {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = [gpui::blue(), gpui::red(), gpui::green()];

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_5()
            .bg(gpui::white())
            .justify_center()
            .items_center()
            .text_color(rgb(0x333333))
            .child(div().text_xl().text_center().child("Drop & Drop"))
            .child(
                div()
                    .w_full()
                    .mb_10()
                    .justify_center()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .items_center()
                    .children(items.into_iter().enumerate().map(|(ix, color)| {
                        let drag_info = DragInfo::new(ix, color);

                        div()
                            .id(("item", ix))
                            .size_32()
                            .flex()
                            .justify_center()
                            .items_center()
                            .border_2()
                            .border_color(color)
                            .text_color(color)
                            .cursor_move()
                            .hover(|this| this.bg(color.opacity(0.2)))
                            .child(format!("Item ({})", ix))
                            .on_drag(drag_info, |info: &DragInfo, position, _, cx| {
                                cx.new(|_| info.position(position))
                            })
                    })),
            )
            .child(
                div()
                    .id("drop-target")
                    .w_128()
                    .h_32()
                    .flex()
                    .justify_center()
                    .items_center()
                    .border_3()
                    .border_color(self.drop_on.map(|info| info.color).unwrap_or(gpui::black()))
                    .when_some(self.drop_on, |this, info| this.bg(info.color.opacity(0.5)))
                    .on_drop(cx.listener(|this, info: &DragInfo, _, _| {
                        this.drop_on = Some(*info);
                    }))
                    .child("Drop items here"),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| DragDrop::new()),
        )
        .unwrap();

        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/focus_visible.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, Div, ElementId, FocusHandle, KeyBinding, SharedString, Stateful, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, size,
};
use gpui_platform::application;

actions!(example, [Tab, TabPrev, Quit]);

struct Example {
    focus_handle: FocusHandle,
    items: Vec<(FocusHandle, &'static str)>,
    message: SharedString,
}

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = vec![
            (
                cx.focus_handle().tab_index(1).tab_stop(true),
                "Button with .focus() - always shows border when focused",
            ),
            (
                cx.focus_handle().tab_index(2).tab_stop(true),
                "Button with .focus_visible() - only shows border with keyboard",
            ),
            (
                cx.focus_handle().tab_index(3).tab_stop(true),
                "Button with both .focus() and .focus_visible()",
            ),
        ];

        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);

        Self {
            focus_handle,
            items,
            message: SharedString::from(
                "Try clicking vs tabbing! Click shows no border, Tab shows border.",
            ),
        }
    }

    fn on_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_next(cx);
        self.message = SharedString::from("Pressed Tab - focus-visible border should appear!");
    }

    fn on_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_prev(cx);
        self.message =
            SharedString::from("Pressed Shift-Tab - focus-visible border should appear!");
    }

    fn on_quit(&mut self, _: &Quit, _window: &mut Window, cx: &mut Context<Self>) {
        cx.quit();
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        fn button_base(id: impl Into<ElementId>, label: &'static str) -> Stateful<Div> {
            div()
                .id(id)
                .h_16()
                .w_full()
                .flex()
                .justify_center()
                .items_center()
                .bg(gpui::rgb(0x2563eb))
                .text_color(gpui::white())
                .rounded_md()
                .cursor_pointer()
                .hover(|style| style.bg(gpui::rgb(0x1d4ed8)))
                .child(label)
        }

        div()
            .id("app")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_tab))
            .on_action(cx.listener(Self::on_tab_prev))
            .on_action(cx.listener(Self::on_quit))
            .size_full()
            .flex()
            .flex_col()
            .p_8()
            .gap_6()
            .bg(gpui::rgb(0xf3f4f6))
            .child(
                div()
                    .text_2xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(gpui::rgb(0x111827))
                    .child("CSS focus-visible Demo"),
            )
            .child(
                div()
                    .p_4()
                    .rounded_md()
                    .bg(gpui::rgb(0xdbeafe))
                    .text_color(gpui::rgb(0x1e3a8a))
                    .child(self.message.clone()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(gpui::rgb(0x374151))
                                    .child("1. Regular .focus() - always visible:"),
                            )
                            .child(
                                button_base("button1", self.items[0].1)
                                    .track_focus(&self.items[0].0)
                                    .focus(|style| {
                                        style.border_4().border_color(gpui::rgb(0xfbbf24))
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.message =
                                            "Clicked button 1 - focus border is visible!".into();
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(gpui::rgb(0x374151))
                                    .child("2. New .focus_visible() - only keyboard:"),
                            )
                            .child(
                                button_base("button2", self.items[1].1)
                                    .track_focus(&self.items[1].0)
                                    .focus_visible(|style| {
                                        style.border_4().border_color(gpui::rgb(0x10b981))
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.message =
                                            "Clicked button 2 - no border! Try Tab instead.".into();
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(gpui::rgb(0x374151))
                                    .child(
                                        "3. Both .focus() (yellow) and .focus_visible() (green):",
                                    ),
                            )
                            .child(
                                button_base("button3", self.items[2].1)
                                    .track_focus(&self.items[2].0)
                                    .focus(|style| {
                                        style.border_4().border_color(gpui::rgb(0xfbbf24))
                                    })
                                    .focus_visible(|style| {
                                        style.border_4().border_color(gpui::rgb(0x10b981))
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.message =
                                            "Clicked button 3 - yellow border. Tab shows green!"
                                                .into();
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("tab", Tab, None),
            KeyBinding::new("shift-tab", TabPrev, None),
            KeyBinding::new("cmd-q", Quit, None),
        ]);

        let bounds = Bounds::centered(None, size(px(800.), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Example::new(window, cx)),
        )
        .unwrap();

        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/gif_viewer.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{App, Context, Render, Window, WindowOptions, div, img, prelude::*};
use gpui_platform::application;
use std::path::PathBuf;

struct GifViewer {
    gif_path: PathBuf,
}

impl GifViewer {
    fn new(gif_path: PathBuf) -> Self {
        Self { gif_path }
    }
}

impl Render for GifViewer {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            img(self.gif_path.clone())
                .size_full()
                .object_fit(gpui::ObjectFit::Contain)
                .id("gif"),
        )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let gif_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/image/black-cat-typing.gif");

        cx.open_window(
            WindowOptions {
                focus: true,
                ..Default::default()
            },
            |_, cx| cx.new(|_| GifViewer::new(gif_path)),
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    env_logger::init();
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/gradient.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, ColorSpace, Context, Half, Render, Window, WindowOptions, canvas, div,
    linear_color_stop, linear_gradient, point, prelude::*, px, size,
};
use gpui_platform::application;

struct GradientViewer {
    color_space: ColorSpace,
}

impl GradientViewer {
    fn new() -> Self {
        Self {
            color_space: ColorSpace::default(),
        }
    }
}

impl Render for GradientViewer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let color_space = self.color_space;

        div()
            .bg(gpui::white())
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_between()
                    .items_center()
                    .child("Gradient Examples")
                    .child(
                        div().flex().gap_2().items_center().child(
                            div()
                                .id("method")
                                .flex()
                                .px_3()
                                .py_1()
                                .text_sm()
                                .bg(gpui::black())
                                .text_color(gpui::white())
                                .child(format!("{}", color_space))
                                .active(|this| this.opacity(0.8))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.color_space = match this.color_space {
                                        ColorSpace::Oklab => ColorSpace::Srgb,
                                        ColorSpace::Srgb => ColorSpace::Oklab,
                                    };
                                    cx.notify();
                                })),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .gap_3()
                    .child(
                        div()
                            .size_full()
                            .rounded_xl()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(gpui::red())
                            .text_color(gpui::white())
                            .child("Solid Color"),
                    )
                    .child(
                        div()
                            .size_full()
                            .rounded_xl()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(gpui::blue())
                            .text_color(gpui::white())
                            .child("Solid Color"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .gap_3()
                    .h_24()
                    .text_color(gpui::white())
                    .child(
                        div().flex_1().rounded_xl().bg(linear_gradient(
                            45.,
                            linear_color_stop(gpui::red(), 0.),
                            linear_color_stop(gpui::blue(), 1.),
                        )
                        .color_space(color_space)),
                    )
                    .child(
                        div().flex_1().rounded_xl().bg(linear_gradient(
                            135.,
                            linear_color_stop(gpui::red(), 0.),
                            linear_color_stop(gpui::green(), 1.),
                        )
                        .color_space(color_space)),
                    )
                    .child(
                        div().flex_1().rounded_xl().bg(linear_gradient(
                            225.,
                            linear_color_stop(gpui::green(), 0.),
                            linear_color_stop(gpui::blue(), 1.),
                        )
                        .color_space(color_space)),
                    )
                    .child(
                        div().flex_1().rounded_xl().bg(linear_gradient(
                            315.,
                            linear_color_stop(gpui::green(), 0.),
                            linear_color_stop(gpui::yellow(), 1.),
                        )
                        .color_space(color_space)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .gap_3()
                    .h_24()
                    .text_color(gpui::white())
                    .child(
                        div().flex_1().rounded_xl().bg(linear_gradient(
                            0.,
                            linear_color_stop(gpui::red(), 0.),
                            linear_color_stop(gpui::white(), 1.),
                        )
                        .color_space(color_space)),
                    )
                    .child(
                        div().flex_1().rounded_xl().bg(linear_gradient(
                            90.,
                            linear_color_stop(gpui::blue(), 0.),
                            linear_color_stop(gpui::white(), 1.),
                        )
                        .color_space(color_space)),
                    )
                    .child(
                        div().flex_1().rounded_xl().bg(linear_gradient(
                            180.,
                            linear_color_stop(gpui::green(), 0.),
                            linear_color_stop(gpui::white(), 1.),
                        )
                        .color_space(color_space)),
                    )
                    .child(
                        div().flex_1().rounded_xl().bg(linear_gradient(
                            360.,
                            linear_color_stop(gpui::yellow(), 0.),
                            linear_color_stop(gpui::white(), 1.),
                        )
                        .color_space(color_space)),
                    ),
            )
            .child(
                div().flex_1().rounded_xl().bg(linear_gradient(
                    0.,
                    linear_color_stop(gpui::green(), 0.05),
                    linear_color_stop(gpui::yellow(), 0.95),
                )
                .color_space(color_space)),
            )
            .child(
                div().flex_1().rounded_xl().bg(linear_gradient(
                    90.,
                    linear_color_stop(gpui::blue(), 0.05),
                    linear_color_stop(gpui::red(), 0.95),
                )
                .color_space(color_space)),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .gap_3()
                    .child(
                        div().flex().flex_1().gap_3().child(
                            div().flex_1().rounded_xl().bg(linear_gradient(
                                90.,
                                linear_color_stop(gpui::blue(), 0.5),
                                linear_color_stop(gpui::red(), 0.5),
                            )
                            .color_space(color_space)),
                        ),
                    )
                    .child(
                        div().flex_1().rounded_xl().bg(linear_gradient(
                            180.,
                            linear_color_stop(gpui::green(), 0.),
                            linear_color_stop(gpui::blue(), 0.5),
                        )
                        .color_space(color_space)),
                    ),
            )
            .child(div().h_24().child(canvas(
                move |_, _, _| {},
                move |bounds, _, window, _| {
                    let size = size(bounds.size.width * 0.8, px(80.));
                    let square_bounds = Bounds {
                        origin: point(
                            bounds.size.width.half() - size.width.half(),
                            bounds.origin.y,
                        ),
                        size,
                    };
                    let height = square_bounds.size.height;
                    let horizontal_offset = height;
                    let vertical_offset = px(30.);
                    let mut builder = gpui::PathBuilder::fill();
                    builder.move_to(square_bounds.bottom_left());
                    builder
                        .line_to(square_bounds.origin + point(horizontal_offset, vertical_offset));
                    builder.line_to(
                        square_bounds.top_right() + point(-horizontal_offset, vertical_offset),
                    );

                    builder.line_to(square_bounds.bottom_right());
                    builder.line_to(square_bounds.bottom_left());
                    let path = builder.build().unwrap();
                    window.paint_path(
                        path,
                        linear_gradient(
                            180.,
                            linear_color_stop(gpui::red(), 0.),
                            linear_color_stop(gpui::blue(), 1.),
                        )
                        .color_space(color_space),
                    );
                },
            )))
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                focus: true,
                ..Default::default()
            },
            |_, cx| cx.new(|_| GradientViewer::new()),
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/grid_layout.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, Hsla, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_platform::application;

// https://en.wikipedia.org/wiki/Holy_grail_(web_design)
struct HolyGrailExample {}

impl Render for HolyGrailExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let block = |color: Hsla| {
            div()
                .size_full()
                .bg(color)
                .border_1()
                .border_dashed()
                .rounded_md()
                .border_color(gpui::white())
                .items_center()
        };

        div()
            .gap_1()
            .grid()
            .bg(rgb(0x505050))
            .size(px(500.0))
            .shadow_lg()
            .border_1()
            .size_full()
            .grid_cols(5)
            .grid_rows(5)
            .child(
                block(gpui::white())
                    .row_span(1)
                    .col_span_full()
                    .child("Header"),
            )
            .child(
                block(gpui::red())
                    .col_span(1)
                    .h_56()
                    .child("Table of contents"),
            )
            .child(
                block(gpui::green())
                    .col_span(3)
                    .row_span(3)
                    .child("Content"),
            )
            .child(
                block(gpui::blue())
                    .col_span(1)
                    .row_span(3)
                    .child("AD :(")
                    .text_color(gpui::white()),
            )
            .child(
                block(gpui::black())
                    .row_span(1)
                    .col_span_full()
                    .text_color(gpui::white())
                    .child("Footer"),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| HolyGrailExample {}),
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/hello_world.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, SharedString, Window, WindowBounds, WindowOptions, div, prelude::*, px,
    rgb, size,
};
use gpui_platform::application;

struct HelloWorld {
    text: SharedString,
}

impl Render for HelloWorld {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x505050))
            .size(px(500.0))
            .justify_center()
            .items_center()
            .shadow_lg()
            .border_1()
            .border_color(rgb(0x0000ff))
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(format!("Hello, {}!", &self.text))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::red())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .border_color(gpui::white()),
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::green())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .border_color(gpui::white()),
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::blue())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .border_color(gpui::white()),
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::yellow())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .border_color(gpui::white()),
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::black())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .rounded_md()
                            .border_color(gpui::white()),
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::white())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .border_color(gpui::black()),
                    ),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| HelloWorld {
                    text: "World".into(),
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/image/image.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use gpui::{
    App, AppContext, AssetSource, Bounds, Context, ImageSource, KeyBinding, Menu, MenuItem, Point,
    SharedString, SharedUri, TitlebarOptions, Window, WindowBounds, WindowOptions, actions, div,
    img, prelude::*, px, rgb, size,
};
#[cfg(not(target_family = "wasm"))]
use reqwest_client::ReqwestClient;

struct Assets {
    base: PathBuf,
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        fs::read(self.base.join(path))
            .map(|data| Some(std::borrow::Cow::Owned(data)))
            .map_err(|e| e.into())
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        fs::read_dir(self.base.join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(|e| e.into())
    }
}

#[derive(IntoElement)]
struct ImageContainer {
    text: SharedString,
    src: ImageSource,
}

impl ImageContainer {
    pub fn new(text: impl Into<SharedString>, src: impl Into<ImageSource>) -> Self {
        Self {
            text: text.into(),
            src: src.into(),
        }
    }
}

impl RenderOnce for ImageContainer {
    fn render(self, _window: &mut Window, _: &mut App) -> impl IntoElement {
        div().child(
            div()
                .flex_row()
                .size_full()
                .gap_4()
                .child(self.text)
                .child(img(self.src).size(px(256.0))),
        )
    }
}

struct ImageShowcase {
    local_resource: Arc<std::path::Path>,
    remote_resource: SharedUri,
    asset_resource: SharedString,
}

impl Render for ImageShowcase {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("main")
            .bg(gpui::white())
            .overflow_y_scroll()
            .p_5()
            .size_full()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .items_center()
                    .gap_8()
                    .child(img(
                        "https://github.com/zed-industries/zed/actions/workflows/ci.yml/badge.svg",
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_center()
                            .items_center()
                            .gap_8()
                            .child(ImageContainer::new(
                                "Image loaded from a local file",
                                self.local_resource.clone(),
                            ))
                            .child(ImageContainer::new(
                                "Image loaded from a remote resource",
                                self.remote_resource.clone(),
                            ))
                            .child(ImageContainer::new(
                                "Image loaded from an asset",
                                self.asset_resource.clone(),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_8()
                            .child(
                                div()
                                    .flex_col()
                                    .child("Auto Width")
                                    .child(img("https://picsum.photos/800/400").h(px(180.))),
                            )
                            .child(
                                div()
                                    .flex_col()
                                    .child("Auto Height")
                                    .child(img("https://picsum.photos/800/400").w(px(180.))),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .items_center()
                            .w_full()
                            .border_1()
                            .border_color(rgb(0xC0C0C0))
                            .child("image with max width 100%")
                            .child(img("https://picsum.photos/800/400").max_w_full()),
                    ),
            )
    }
}

actions!(image, [Quit]);

fn run_example() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    #[cfg(not(target_family = "wasm"))]
    let app = gpui_platform::application();
    #[cfg(target_family = "wasm")]
    let app = gpui_platform::application();
    app.with_assets(Assets {
        base: manifest_dir.join("examples"),
    })
    .run(move |cx: &mut App| {
        #[cfg(not(target_family = "wasm"))]
        {
            let http_client = ReqwestClient::user_agent("gpui example").unwrap();
            cx.set_http_client(Arc::new(http_client));
        }
        #[cfg(target_family = "wasm")]
        {
            // Safety: the web examples run single-threaded; the client is
            // created and used exclusively on the main thread.
            let http_client = unsafe {
                gpui_web::FetchHttpClient::with_user_agent("gpui example")
                    .expect("failed to create FetchHttpClient")
            };
            cx.set_http_client(Arc::new(http_client));
        }

        cx.activate(true);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.set_menus(vec![Menu {
            name: "Image".into(),
            items: vec![MenuItem::action("Quit", Quit)],
            disabled: false,
        }]);

        let window_options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(SharedString::from("Image Example")),
                appears_transparent: false,
                ..Default::default()
            }),

            window_bounds: Some(WindowBounds::Windowed(Bounds {
                size: size(px(1100.), px(600.)),
                origin: Point::new(px(200.), px(200.)),
            })),

            ..Default::default()
        };

        cx.open_window(window_options, |_, cx| {
            cx.new(|_| ImageShowcase {
                // Relative path to your root project path
                local_resource: manifest_dir.join("examples/image/app-icon.png").into(),
                remote_resource: "https://picsum.photos/800/400".into(),
                asset_resource: "image/color.svg".into(),
            })
        })
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    env_logger::init();
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/image_gallery.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use futures::FutureExt;
use gpui::{
    App, AppContext, Asset as _, AssetLogger, Bounds, ClickEvent, Context, ElementId, Entity,
    ImageAssetLoader, ImageCache, ImageCacheProvider, KeyBinding, Menu, MenuItem,
    RetainAllImageCache, SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions,
    actions, div, hash, image_cache, img, prelude::*, px, rgb, size,
};
#[cfg(not(target_family = "wasm"))]
use reqwest_client::ReqwestClient;
use std::{collections::HashMap, sync::Arc};

const IMAGES_IN_GALLERY: usize = 30;

struct ImageGallery {
    image_key: String,
    items_count: usize,
    total_count: usize,
    image_cache: Entity<RetainAllImageCache>,
}

impl ImageGallery {
    fn on_next_image(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.image_cache
            .update(cx, |image_cache, cx| image_cache.clear(window, cx));

        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        self.image_key = format!("{}", t);
        self.total_count += self.items_count;
        cx.notify();
    }
}

impl Render for ImageGallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let image_url: SharedString =
            format!("https://picsum.photos/400/200?t={}", self.image_key).into();

        div()
            .flex()
            .flex_col()
            .text_color(gpui::white())
            .child("Manually managed image cache:")
            .child(
                div()
                    .image_cache(self.image_cache.clone())
                    .id("main")
                    .text_color(gpui::black())
                    .bg(rgb(0xE9E9E9))
                    .overflow_y_scroll()
                    .p_4()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .child(format!(
                                "Example to show images and test memory usage (Rendered: {} images).",
                                self.total_count
                            ))
                            .child(
                                div()
                                    .id("btn")
                                    .py_1()
                                    .px_4()
                                    .bg(gpui::black())
                                    .hover(|this| this.opacity(0.8))
                                    .text_color(gpui::white())
                                    .text_center()
                                    .w_40()
                                    .child("Next Photos")
                                    .on_click(cx.listener(Self::on_next_image)),
                            ),
                    )
                    .child(
                        div()
                            .id("image-gallery")
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap_x_4()
                            .gap_y_2()
                            .justify_around()
                            .children(
                                (0..self.items_count)
                                    .map(|ix| img(format!("{}-{}", image_url, ix)).size_20()),
                            ),
                    ),
            )
            .child(
                "Automatically managed image cache:"
            )
            .child(image_cache(simple_lru_cache("lru-cache", IMAGES_IN_GALLERY)).child(
                div()
                    .id("main")
                    .bg(rgb(0xE9E9E9))
                    .text_color(gpui::black())
                    .overflow_y_scroll()
                    .p_4()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("image-gallery")
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap_x_4()
                            .gap_y_2()
                            .justify_around()
                            .children(
                                (0..self.items_count)
                                    .map(|ix| img(format!("{}-{}", image_url, ix)).size_20()),
                            ),
                    )
            ))
    }
}

fn simple_lru_cache(id: impl Into<ElementId>, max_items: usize) -> SimpleLruCacheProvider {
    SimpleLruCacheProvider {
        id: id.into(),
        max_items,
    }
}

struct SimpleLruCacheProvider {
    id: ElementId,
    max_items: usize,
}

impl ImageCacheProvider for SimpleLruCacheProvider {
    fn provide(&mut self, window: &mut Window, cx: &mut App) -> gpui::AnyImageCache {
        window
            .with_global_id(self.id.clone(), |global_id, window| {
                window.with_element_state::<Entity<SimpleLruCache>, _>(
                    global_id,
                    |lru_cache, _window| {
                        let mut lru_cache = lru_cache.unwrap_or_else(|| {
                            cx.new(|cx| SimpleLruCache::new(self.max_items, cx))
                        });
                        if lru_cache.read(cx).max_items != self.max_items {
                            lru_cache = cx.new(|cx| SimpleLruCache::new(self.max_items, cx));
                        }
                        (lru_cache.clone(), lru_cache)
                    },
                )
            })
            .into()
    }
}

struct SimpleLruCache {
    max_items: usize,
    usages: Vec<u64>,
    cache: HashMap<u64, gpui::ImageCacheItem>,
}

impl SimpleLruCache {
    fn new(max_items: usize, cx: &mut Context<Self>) -> Self {
        cx.on_release(|simple_cache, cx| {
            for (_, mut item) in std::mem::take(&mut simple_cache.cache) {
                if let Some(Ok(image)) = item.get() {
                    cx.drop_image(image, None);
                }
            }
        })
        .detach();

        Self {
            max_items,
            usages: Vec::with_capacity(max_items),
            cache: HashMap::with_capacity(max_items),
        }
    }
}

impl ImageCache for SimpleLruCache {
    fn load(
        &mut self,
        resource: &gpui::Resource,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Result<Arc<gpui::RenderImage>, gpui::ImageCacheError>> {
        assert_eq!(self.usages.len(), self.cache.len());
        assert!(self.cache.len() <= self.max_items);

        let hash = hash(resource);

        if let Some(item) = self.cache.get_mut(&hash) {
            let current_ix = self
                .usages
                .iter()
                .position(|item| *item == hash)
                .expect("cache and usages must stay in sync");
            self.usages.remove(current_ix);
            self.usages.insert(0, hash);

            return item.get();
        }

        let fut = AssetLogger::<ImageAssetLoader>::load(resource.clone(), cx);
        let task = cx.background_executor().spawn(fut).shared();
        if self.usages.len() == self.max_items {
            let oldest = self.usages.pop().unwrap();
            let mut image = self
                .cache
                .remove(&oldest)
                .expect("cache and usages must be in sync");
            if let Some(Ok(image)) = image.get() {
                cx.drop_image(image, Some(window));
            }
        }
        self.cache
            .insert(hash, gpui::ImageCacheItem::Loading(task.clone()));
        self.usages.insert(0, hash);

        let entity = window.current_view();
        window
            .spawn(cx, {
                async move |cx| {
                    _ = task.await;
                    cx.on_next_frame(move |_, cx| {
                        cx.notify(entity);
                    });
                }
            })
            .detach();

        None
    }
}

actions!(image, [Quit]);

fn run_example() {
    #[cfg(not(target_family = "wasm"))]
    let app = gpui_platform::application();
    #[cfg(target_family = "wasm")]
    let app = gpui_platform::single_threaded_web();

    app.run(move |cx: &mut App| {
        #[cfg(not(target_family = "wasm"))]
        {
            let http_client = ReqwestClient::user_agent("gpui example").unwrap();
            cx.set_http_client(Arc::new(http_client));
        }
        #[cfg(target_family = "wasm")]
        {
            // Safety: the web examples run single-threaded; the client is
            // created and used exclusively on the main thread.
            let http_client = unsafe {
                gpui_web::FetchHttpClient::with_user_agent("gpui example")
                    .expect("failed to create FetchHttpClient")
            };
            cx.set_http_client(Arc::new(http_client));
        }

        cx.activate(true);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.set_menus([Menu::new("Image Gallery").items([MenuItem::action("Quit", Quit)])]);

        let window_options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(SharedString::from("Image Gallery")),
                appears_transparent: false,
                ..Default::default()
            }),

            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1100.), px(860.)),
                cx,
            ))),

            ..Default::default()
        };

        cx.open_window(window_options, |_, cx| {
            cx.new(|ctx| ImageGallery {
                image_key: "".into(),
                items_count: IMAGES_IN_GALLERY,
                total_count: 0,
                image_cache: RetainAllImageCache::new(ctx),
            })
        })
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    env_logger::init();
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/image_loading.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use std::{path::Path, sync::Arc, time::Duration};

use gpui::{
    Animation, AnimationExt, App, Asset, AssetLogger, AssetSource, Bounds, Context, Hsla,
    ImageAssetLoader, ImageCacheError, ImgResourceLoader, LOADING_DELAY, Length, RenderImage,
    Resource, SharedString, Window, WindowBounds, WindowOptions, black, div, img, prelude::*,
    pulsating_between, px, red, size,
};
use gpui_platform::application;

struct Assets {}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        std::fs::read(path)
            .map(Into::into)
            .map_err(Into::into)
            .map(Some)
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(std::fs::read_dir(path)?
            .filter_map(|entry| {
                Some(SharedString::from(
                    entry.ok()?.path().to_string_lossy().into_owned(),
                ))
            })
            .collect::<Vec<_>>())
    }
}

const IMAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/image/app-icon.png");

#[derive(Copy, Clone, Hash)]
struct LoadImageParameters {
    timeout: Duration,
    fail: bool,
}

struct LoadImageWithParameters {}

impl Asset for LoadImageWithParameters {
    type Source = LoadImageParameters;

    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    fn load(
        parameters: Self::Source,
        cx: &mut App,
    ) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
        let timer = cx.background_executor().timer(parameters.timeout);
        let data = AssetLogger::<ImageAssetLoader>::load(
            Resource::Path(Path::new(IMAGE).to_path_buf().into()),
            cx,
        );
        async move {
            timer.await;
            if parameters.fail {
                log::error!("Intentionally failed to load image");
                Err(anyhow::anyhow!("Failed to load image").into())
            } else {
                data.await
            }
        }
    }
}

struct ImageLoadingExample {}

impl ImageLoadingExample {
    fn loading_element() -> impl IntoElement {
        div().size_full().flex_none().p_0p5().rounded_xs().child(
            div().size_full().with_animation(
                "loading-bg",
                Animation::new(Duration::from_secs(3))
                    .repeat()
                    .with_easing(pulsating_between(0.04, 0.24)),
                move |this, delta| this.bg(black().opacity(delta)),
            ),
        )
    }

    fn fallback_element() -> impl IntoElement {
        let fallback_color: Hsla = black().opacity(0.5);

        div().size_full().flex_none().p_0p5().child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .rounded_xs()
                .text_sm()
                .text_color(fallback_color)
                .border_1()
                .border_color(fallback_color)
                .child("?"),
        )
    }
}

impl Render for ImageLoadingExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().size_full().justify_around().child(
            div().flex().flex_row().w_full().justify_around().child(
                div()
                    .flex()
                    .bg(gpui::white())
                    .size(Length::Definite(px(300.0).into()))
                    .justify_center()
                    .items_center()
                    .child({
                        let image_source = LoadImageParameters {
                            timeout: LOADING_DELAY.saturating_sub(Duration::from_millis(25)),
                            fail: false,
                        };

                        // Load within the 'loading delay', should not show loading fallback
                        img(move |window: &mut Window, cx: &mut App| {
                            window.use_asset::<LoadImageWithParameters>(&image_source, cx)
                        })
                        .id("image-1")
                        .border_1()
                        .size_12()
                        .with_fallback(|| Self::fallback_element().into_any_element())
                        .border_color(red())
                        .with_loading(|| Self::loading_element().into_any_element())
                        .on_click(move |_, _, cx| {
                            cx.remove_asset::<LoadImageWithParameters>(&image_source);
                        })
                    })
                    .child({
                        // Load after a long delay
                        let image_source = LoadImageParameters {
                            timeout: Duration::from_secs(5),
                            fail: false,
                        };

                        img(move |window: &mut Window, cx: &mut App| {
                            window.use_asset::<LoadImageWithParameters>(&image_source, cx)
                        })
                        .id("image-2")
                        .with_fallback(|| Self::fallback_element().into_any_element())
                        .with_loading(|| Self::loading_element().into_any_element())
                        .size_12()
                        .border_1()
                        .border_color(red())
                        .on_click(move |_, _, cx| {
                            cx.remove_asset::<LoadImageWithParameters>(&image_source);
                        })
                    })
                    .child({
                        // Fail to load image after a long delay
                        let image_source = LoadImageParameters {
                            timeout: Duration::from_secs(5),
                            fail: true,
                        };

                        // Fail to load after a long delay
                        img(move |window: &mut Window, cx: &mut App| {
                            window.use_asset::<LoadImageWithParameters>(&image_source, cx)
                        })
                        .id("image-3")
                        .with_fallback(|| Self::fallback_element().into_any_element())
                        .with_loading(|| Self::loading_element().into_any_element())
                        .size_12()
                        .border_1()
                        .border_color(red())
                        .on_click(move |_, _, cx| {
                            cx.remove_asset::<LoadImageWithParameters>(&image_source);
                        })
                    })
                    .child({
                        // Ensure that the normal image loader doesn't spam logs
                        let image_source = Path::new(
                            "this/file/really/shouldn't/exist/or/won't/be/an/image/I/hope",
                        )
                        .to_path_buf();
                        img(image_source.clone())
                            .id("image-4")
                            .border_1()
                            .size_12()
                            .with_fallback(|| Self::fallback_element().into_any_element())
                            .border_color(red())
                            .with_loading(|| Self::loading_element().into_any_element())
                            .on_click(move |_, _, cx| {
                                cx.remove_asset::<ImgResourceLoader>(&image_source.clone().into());
                            })
                    }),
            ),
        )
    }
}

fn run_example() {
    application().with_assets(Assets {}).run(|cx: &mut App| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(300.), px(300.)),
                cx,
            ))),
            ..Default::default()
        };
        cx.open_window(options, |_, cx| {
            cx.activate(false);
            cx.new(|_| ImageLoadingExample {})
        })
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    env_logger::init();
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/input.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, Focusable, GlobalElementId, KeyBinding, Keystroke, LayoutId,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    ShapedLine, SharedString, Style, TextRun, UTF16Selection, UnderlineStyle, Window, WindowBounds,
    WindowOptions, actions, black, div, fill, hsla, opaque_grey, point, prelude::*, px, relative,
    rgb, rgba, size, white, yellow,
};
use gpui_platform::application;
use unicode_segmentation::*;

actions!(
    text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
        Quit,
    ]
);

struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
}

impl TextInput {
    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                window.play_system_bell();
                return;
            }
            self.select_to(prev, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                window.play_system_bell();
                return;
            }
            self.select_to(next, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;

        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace("\n", " "), window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }
    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        line.closest_index_for_x(position.x - bounds.left())
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    fn reset(&mut self) {
        self.content = "".into();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.last_layout = None;
        self.last_bounds = None;
        self.is_selecting = false;
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;

        assert_eq!(last_layout.text, self.content);
        let utf8_index = last_layout.index_for_x(point.x - line_point.x)?;
        Some(self.offset_to_utf16(utf8_index))
    }
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            (input.placeholder.clone(), hsla(0., 0., 0., 0.2))
        } else {
            (content, style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(2.), bounds.bottom() - bounds.top()),
                    ),
                    gpui::blue(),
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    rgba(0x3311ff30),
                )),
                None,
            )
        };
        PrepaintState {
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let line = prepaint.line.take().unwrap();
        line.paint(
            bounds.origin,
            window.line_height(),
            gpui::TextAlign::Left,
            None,
            window,
            cx,
        )
        .unwrap();

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .key_context("TextInput")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .bg(rgb(0xeeeeee))
            .line_height(px(30.))
            .text_size(px(24.))
            .child(
                div()
                    .h(px(30. + 4. * 2.))
                    .w_full()
                    .p(px(4.))
                    .bg(white())
                    .child(TextElement { input: cx.entity() }),
            )
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

struct InputExample {
    text_input: Entity<TextInput>,
    recent_keystrokes: Vec<Keystroke>,
    focus_handle: FocusHandle,
}

impl Focusable for InputExample {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl InputExample {
    fn on_reset_click(&mut self, _: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.recent_keystrokes.clear();
        self.text_input
            .update(cx, |text_input, _cx| text_input.reset());
        cx.notify();
    }
}

impl Render for InputExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .bg(rgb(0xaaaaaa))
            .track_focus(&self.focus_handle(cx))
            .flex()
            .flex_col()
            .size_full()
            .child(
                div()
                    .bg(white())
                    .border_b_1()
                    .border_color(black())
                    .flex()
                    .flex_row()
                    .justify_between()
                    .child(format!("Keyboard {}", cx.keyboard_layout().name()))
                    .child(
                        div()
                            .border_1()
                            .border_color(black())
                            .px_2()
                            .bg(yellow())
                            .child("Reset")
                            .hover(|style| {
                                style
                                    .bg(yellow().blend(opaque_grey(0.5, 0.5)))
                                    .cursor_pointer()
                            })
                            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_reset_click)),
                    ),
            )
            .child(self.text_input.clone())
            .children(self.recent_keystrokes.iter().rev().map(|ks| {
                format!(
                    "{:} {}",
                    ks.unparse(),
                    if let Some(key_char) = ks.key_char.as_ref() {
                        format!("-> {:?}", key_char)
                    } else {
                        "".to_owned()
                    }
                )
            }))
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(300.0), px(300.0)), cx);
        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("shift-left", SelectLeft, None),
            KeyBinding::new("shift-right", SelectRight, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, None),
        ]);

        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| {
                    let text_input = cx.new(|cx| TextInput {
                        focus_handle: cx.focus_handle(),
                        content: "".into(),
                        placeholder: "Type here...".into(),
                        selected_range: 0..0,
                        selection_reversed: false,
                        marked_range: None,
                        last_layout: None,
                        last_bounds: None,
                        is_selecting: false,
                    });
                    cx.new(|cx| InputExample {
                        text_input,
                        recent_keystrokes: vec![],
                        focus_handle: cx.focus_handle(),
                    })
                },
            )
            .unwrap();
        let view = window.update(cx, |_, _, cx| cx.entity()).unwrap();
        cx.observe_keystrokes(move |ev, _, cx| {
            view.update(cx, |view, cx| {
                view.recent_keystrokes.push(ev.keystroke.clone());
                cx.notify();
            })
        })
        .detach();
        cx.on_keyboard_layout_change({
            move |cx| {
                window.update(cx, |_, _, cx| cx.notify()).ok();
            }
        })
        .detach();

        window
            .update(cx, |view, window, cx| {
                window.focus(&view.text_input.focus_handle(cx), cx);
                cx.activate(true);
            })
            .unwrap();
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/layer_shell.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

fn run_example() {
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    example::main();

    #[cfg(not(all(target_os = "linux", feature = "wayland")))]
    panic!("This example requires the `wayland` feature and a linux system.");
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
mod example {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use gpui::{
        App, Bounds, Context, FontWeight, Size, Window, WindowBackgroundAppearance, WindowBounds,
        WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px, rems, rgba, white,
    };
    use gpui_platform::application;

    struct LayerShellExample;

    impl LayerShellExample {
        fn new(cx: &mut Context<Self>) -> Self {
            cx.spawn(async move |this, cx| {
                loop {
                    let _ = this.update(cx, |_, cx| cx.notify());
                    cx.background_executor()
                        .timer(Duration::from_millis(500))
                        .await;
                }
            })
            .detach();

            LayerShellExample
        }
    }

    impl Render for LayerShellExample {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let hours = (now / 3600) % 24;
            let minutes = (now / 60) % 60;
            let seconds = now % 60;

            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(rems(4.5))
                .font_weight(FontWeight::EXTRA_BOLD)
                .text_color(white())
                .bg(rgba(0x0000044))
                .rounded_xl()
                .child(format!("{:02}:{:02}:{:02}", hours, minutes, seconds))
        }
    }

    pub fn main() {
        application().run(|cx: &mut App| {
            cx.open_window(
                WindowOptions {
                    titlebar: None,
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: point(px(0.), px(0.)),
                        size: Size::new(px(500.), px(200.)),
                    })),
                    app_id: Some("gpui-layer-shell-example".to_string()),
                    window_background: WindowBackgroundAppearance::Transparent,
                    kind: WindowKind::LayerShell(LayerShellOptions {
                        namespace: "gpui".to_string(),
                        anchor: Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM,
                        margin: Some((px(0.), px(0.), px(40.), px(0.))),
                        keyboard_interactivity: KeyboardInteractivity::None,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(LayerShellExample::new),
            )
            .unwrap();
        });
    }
}

```
---
## `example_file:crates/gpui/examples/list_example.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, ListAlignment, ListState, Render, Window, WindowBounds, WindowOptions,
    div, list, prelude::*, px, rgb, size,
};
use gpui_platform::application;

const ITEM_COUNT: usize = 40;
const SCROLLBAR_WIDTH: f32 = 12.;

struct BottomListDemo {
    list_state: ListState,
}

impl BottomListDemo {
    fn new() -> Self {
        Self {
            list_state: ListState::new(ITEM_COUNT, ListAlignment::Bottom, px(500.)).measure_all(),
        }
    }
}

impl Render for BottomListDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let max_offset = self.list_state.max_offset_for_scrollbar().y;
        let current_offset = -self.list_state.scroll_px_offset_for_scrollbar().y;

        let viewport_height = self.list_state.viewport_bounds().size.height;

        let raw_fraction = if max_offset > px(0.) {
            current_offset / max_offset
        } else {
            0.
        };

        let total_height = viewport_height + max_offset;
        let thumb_height = if total_height > px(0.) {
            px(viewport_height.as_f32() * viewport_height.as_f32() / total_height.as_f32())
                .max(px(30.))
        } else {
            px(30.)
        };

        let track_space = viewport_height - thumb_height;
        let thumb_top = track_space * raw_fraction;

        let bug_detected = raw_fraction > 1.0;

        div()
            .size_full()
            .bg(rgb(0xFFFFFF))
            .flex()
            .flex_col()
            .p_4()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(format!(
                        "offset: {:.0} / max: {:.0} | fraction: {:.3}",
                        current_offset.as_f32(),
                        max_offset.as_f32(),
                        raw_fraction,
                    ))
                    .child(
                        div()
                            .text_color(if bug_detected {
                                rgb(0xCC0000)
                            } else {
                                rgb(0x008800)
                            })
                            .child(if bug_detected {
                                format!(
                                    "BUG: fraction is {:.3} (> 1.0) — thumb is off-track!",
                                    raw_fraction
                                )
                            } else {
                                "OK: fraction <= 1.0 — thumb is within track.".to_string()
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    .border_1()
                    .border_color(rgb(0xCCCCCC))
                    .rounded_sm()
                    .child(
                        list(self.list_state.clone(), |index, _window, _cx| {
                            let height = px(30. + (index % 5) as f32 * 10.);
                            div()
                                .h(height)
                                .w_full()
                                .flex()
                                .items_center()
                                .px_3()
                                .border_b_1()
                                .border_color(rgb(0xEEEEEE))
                                .bg(if index % 2 == 0 {
                                    rgb(0xFAFAFA)
                                } else {
                                    rgb(0xFFFFFF)
                                })
                                .text_sm()
                                .child(format!("Item {index}"))
                                .into_any()
                        })
                        .flex_1(),
                    )
                    // Scrollbar track
                    .child(
                        div()
                            .w(px(SCROLLBAR_WIDTH))
                            .h_full()
                            .flex_shrink_0()
                            .bg(rgb(0xE0E0E0))
                            .relative()
                            .child(
                                // Thumb — position is unclamped to expose the bug
                                div()
                                    .absolute()
                                    .top(thumb_top)
                                    .w_full()
                                    .h(thumb_height)
                                    .bg(if bug_detected {
                                        rgb(0xCC0000)
                                    } else {
                                        rgb(0x888888)
                                    })
                                    .rounded_sm(),
                            ),
                    ),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(400.), px(500.)), cx);
        cx.open_window(
            WindowOptions {
                focus: true,
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| BottomListDemo::new()),
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/mouse_pressure.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, MousePressureEvent, PressureStage, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, rgb, size,
};
use gpui_platform::application;

struct MousePressureExample {
    pressure_stage: PressureStage,
    pressure_amount: f32,
}

impl Render for MousePressureExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x505050))
            .size(px(500.0))
            .justify_center()
            .items_center()
            .shadow_lg()
            .border_1()
            .border_color(rgb(0x0000ff))
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(format!("Pressure stage: {:?}", &self.pressure_stage))
            .child(format!("Pressure amount: {:.2}", &self.pressure_amount))
            .on_mouse_pressure(cx.listener(Self::on_mouse_pressure))
    }
}

impl MousePressureExample {
    fn on_mouse_pressure(
        &mut self,
        pressure_event: &MousePressureEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pressure_amount = pressure_event.pressure;
        self.pressure_stage = pressure_event.stage;

        cx.notify();
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| MousePressureExample {
                    pressure_stage: PressureStage::Zero,
                    pressure_amount: 0.0,
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/move_entity_between_windows.rs`

```rust
//! An entity registers callbacks via the `_in` API family and then gets
//! re-hosted in a new window via a click. The point of the example is to
//! demonstrate that callbacks dispatched after the move correctly target the
//! entity's *current* window rather than the window it was in at
//! registration time.
//!
//! To run:  cargo run -p gpui --example move_entity_between_windows

#![cfg_attr(target_family = "wasm", no_main)]

use std::time::Duration;

use gpui::{
    App, AppContext as _, Bounds, Context, EventEmitter, MouseButton, Render, SharedString,
    Subscription, Task, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_platform::application;

struct MoveToNewWindow;

struct HelloWorld {
    text: SharedString,
    tick_count: u32,
    move_count: u32,
    _tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<MoveToNewWindow> for HelloWorld {}

impl HelloWorld {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let self_entity = cx.entity();

        let task = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let result = this.update_in(cx, |this, window, _cx| {
                    this.tick_count += 1;
                    println!(
                        "tick #{} fired in entity's current window {}",
                        this.tick_count,
                        window.window_handle().window_id().as_u64(),
                    );
                });
                if let Err(err) = result {
                    println!("tick task giving up: {err}");
                    return;
                }
            }
        });

        let subscription = cx.subscribe_in::<_, MoveToNewWindow>(
            &self_entity,
            window,
            move |this, _emitter, _event, window, cx| {
                let entered_window_id = window.window_handle().window_id().as_u64();
                println!(
                    "MoveToNewWindow handler fired in entity's current window {entered_window_id}",
                );

                this.move_count += 1;
                cx.notify();

                let entity = cx.entity();
                let old_window = window.window_handle();
                cx.defer(move |cx| {
                    let bounds = Bounds::centered(None, size(px(500.0), px(500.0)), cx);
                    cx.open_window(
                        WindowOptions {
                            window_bounds: Some(WindowBounds::Windowed(bounds)),
                            ..Default::default()
                        },
                        move |_, _| entity,
                    )
                    .expect("failed to open new window");
                    old_window
                        .update(cx, |_, window, _| window.remove_window())
                        .ok();
                });
            },
        );

        Self {
            text: "World".into(),
            tick_count: 0,
            move_count: 0,
            _tasks: vec![task],
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for HelloWorld {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_id = window.window_handle().window_id().as_u64();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x505050))
            .size(px(500.0))
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(format!("Hello, {}!", &self.text))
            .child(format!("Rendering in window: {window_id}"))
            .child(format!("Ticks observed by entity: {}", self.tick_count))
            .child(format!("Moves observed by entity: {}", self.move_count))
            .child(
                div()
                    .px_4()
                    .py_2()
                    .bg(rgb(0x4040ff))
                    .rounded_md()
                    .child("Move me to a new window")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this, _, _window, cx| {
                            cx.emit(MoveToNewWindow);
                        }),
                    ),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.0), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| HelloWorld::new(window, cx)),
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/on_window_close_quit.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, FocusHandle, KeyBinding, Window, WindowBounds, WindowOptions, actions,
    div, prelude::*, px, rgb, size,
};
use gpui_platform::application;

actions!(example, [CloseWindow]);

struct ExampleWindow {
    focus_handle: FocusHandle,
}

impl Render for ExampleWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .on_action(|_: &CloseWindow, window, _| {
                window.remove_window();
            })
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x505050))
            .size(px(500.0))
            .justify_center()
            .items_center()
            .shadow_lg()
            .border_1()
            .border_color(rgb(0x0000ff))
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(
                "Closing this window with cmd-w or the traffic lights should quit the application!",
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let mut bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);

        cx.bind_keys([KeyBinding::new("cmd-w", CloseWindow, None)]);
        cx.on_window_closed(|cx, _window_id| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                cx.activate(false);
                cx.new(|cx| {
                    let focus_handle = cx.focus_handle();
                    focus_handle.focus(window, cx);
                    ExampleWindow { focus_handle }
                })
            },
        )
        .unwrap();

        bounds.origin.x += bounds.size.width;

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let focus_handle = cx.focus_handle();
                    focus_handle.focus(window, cx);
                    ExampleWindow { focus_handle }
                })
            },
        )
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/opacity.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use std::{fs, path::PathBuf};

use anyhow::Result;
use gpui::{
    App, AssetSource, Bounds, BoxShadow, ClickEvent, Context, SharedString, Task, Window,
    WindowBounds, WindowOptions, div, hsla, img, point, prelude::*, px, rgb, size, svg,
};
use gpui_platform::application;

struct Assets {
    base: PathBuf,
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        fs::read(self.base.join(path))
            .map(|data| Some(std::borrow::Cow::Owned(data)))
            .map_err(|e| e.into())
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        fs::read_dir(self.base.join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(|e| e.into())
    }
}

struct HelloWorld {
    _task: Option<Task<()>>,
    opacity: f32,
    animating: bool,
}

impl HelloWorld {
    fn new(_window: &mut Window, _: &mut Context<Self>) -> Self {
        Self {
            _task: None,
            opacity: 0.5,
            animating: false,
        }
    }

    fn start_animation(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.opacity = 0.0;
        self.animating = true;
        cx.notify();
    }
}

impl Render for HelloWorld {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.animating {
            self.opacity += 0.005;
            if self.opacity >= 1.0 {
                self.animating = false;
                self.opacity = 1.0;
            } else {
                window.request_animation_frame();
            }
        }

        div()
            .flex()
            .flex_row()
            .size_full()
            .bg(rgb(0xe0e0e0))
            .text_xl()
            .child(
                div()
                    .flex()
                    .size_full()
                    .justify_center()
                    .items_center()
                    .border_1()
                    .text_color(gpui::blue())
                    .child(div().child("This is background text.")),
            )
            .child(
                div()
                    .id("panel")
                    .on_click(cx.listener(Self::start_animation))
                    .absolute()
                    .top_8()
                    .left_8()
                    .right_8()
                    .bottom_8()
                    .opacity(self.opacity)
                    .flex()
                    .justify_center()
                    .items_center()
                    .bg(gpui::white())
                    .border_3()
                    .border_color(gpui::red())
                    .text_color(gpui::yellow())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .justify_center()
                            .items_center()
                            .size(px(300.))
                            .bg(gpui::blue())
                            .border_3()
                            .border_color(gpui::black())
                            .shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.5),
                                blur_radius: px(1.0),
                                spread_radius: px(5.0),
                                offset: point(px(10.0), px(10.0)),
                            }])
                            .child(img("image/app-icon.png").size_8())
                            .child("Opacity Panel (Click to test)")
                            .child(
                                div()
                                    .id("deep-level-text")
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .p_4()
                                    .bg(gpui::black())
                                    .text_color(gpui::white())
                                    .text_decoration_2()
                                    .text_decoration_wavy()
                                    .text_decoration_color(gpui::red())
                                    .child(format!("opacity: {:.1}", self.opacity)),
                            )
                            .child(
                                svg()
                                    .path("image/arrow_circle.svg")
                                    .text_color(gpui::black())
                                    .text_2xl()
                                    .size_8(),
                            )
                            .child(
                                div()
                                    .flex()
                                    .children(["🎊", "✈️", "🎉", "🎈", "🎁", "🎂"].map(|emoji| {
                                        div()
                                            .child(emoji.to_string())
                                            .hover(|style| style.opacity(0.5))
                                    })),
                            )
                            .child(img("image/black-cat-typing.gif").size_12()),
                    ),
            )
    }
}

fn run_example() {
    application()
        .with_assets(Assets {
            base: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples"),
        })
        .run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(500.0), px(500.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| HelloWorld::new(window, cx)),
            )
            .unwrap();
            cx.activate(true);
        });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/ownership_post.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{App, Context, Entity, EventEmitter, prelude::*};
use gpui_platform::application;

struct Counter {
    count: usize,
}

struct Change {
    increment: usize,
}

impl EventEmitter<Change> for Counter {}

fn run_example() {
    application().run(|cx: &mut App| {
        let counter: Entity<Counter> = cx.new(|_cx| Counter { count: 0 });
        let subscriber = cx.new(|cx: &mut Context<Counter>| {
            cx.subscribe(&counter, |subscriber, _emitter, event, _cx| {
                subscriber.count += event.increment * 2;
            })
            .detach();

            Counter {
                count: counter.read(cx).count * 2,
            }
        });

        counter.update(cx, |counter, cx| {
            counter.count += 2;
            cx.notify();
            cx.emit(Change { increment: 2 });
        });

        assert_eq!(subscriber.read(cx).count, 4);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/painting.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    Background, Bounds, ColorSpace, Context, MouseDownEvent, Path, PathBuilder, PathStyle, Pixels,
    Point, Render, StrokeOptions, Window, WindowOptions, canvas, div, linear_color_stop,
    linear_gradient, point, prelude::*, px, quad, rgb, size,
};
use gpui_platform::application;

struct PaintingViewer {
    default_lines: Vec<(Path<Pixels>, Background)>,
    background_quads: Vec<(Bounds<Pixels>, Background)>,
    lines: Vec<Vec<Point<Pixels>>>,
    start: Point<Pixels>,
    dashed: bool,
    _painting: bool,
}

impl PaintingViewer {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let mut lines = vec![];

        // Black squares beneath transparent paths.
        let background_quads = vec![
            (
                Bounds {
                    origin: point(px(70.), px(70.)),
                    size: size(px(40.), px(40.)),
                },
                gpui::black().into(),
            ),
            (
                Bounds {
                    origin: point(px(170.), px(70.)),
                    size: size(px(40.), px(40.)),
                },
                gpui::black().into(),
            ),
            (
                Bounds {
                    origin: point(px(270.), px(70.)),
                    size: size(px(40.), px(40.)),
                },
                gpui::black().into(),
            ),
            (
                Bounds {
                    origin: point(px(370.), px(70.)),
                    size: size(px(40.), px(40.)),
                },
                gpui::black().into(),
            ),
            (
                Bounds {
                    origin: point(px(450.), px(50.)),
                    size: size(px(80.), px(80.)),
                },
                gpui::black().into(),
            ),
        ];

        // 50% opaque red path that extends across black quad.
        let mut builder = PathBuilder::fill();
        builder.move_to(point(px(50.), px(50.)));
        builder.line_to(point(px(130.), px(50.)));
        builder.line_to(point(px(130.), px(130.)));
        builder.line_to(point(px(50.), px(130.)));
        builder.close();
        let path = builder.build().unwrap();
        let mut red = rgb(0xFF0000);
        red.a = 0.5;
        lines.push((path, red.into()));

        // 50% opaque blue path that extends across black quad.
        let mut builder = PathBuilder::fill();
        builder.move_to(point(px(150.), px(50.)));
        builder.line_to(point(px(230.), px(50.)));
        builder.line_to(point(px(230.), px(130.)));
        builder.line_to(point(px(150.), px(130.)));
        builder.close();
        let path = builder.build().unwrap();
        let mut blue = rgb(0x0000FF);
        blue.a = 0.5;
        lines.push((path, blue.into()));

        // 50% opaque green path that extends across black quad.
        let mut builder = PathBuilder::fill();
        builder.move_to(point(px(250.), px(50.)));
        builder.line_to(point(px(330.), px(50.)));
        builder.line_to(point(px(330.), px(130.)));
        builder.line_to(point(px(250.), px(130.)));
        builder.close();
        let path = builder.build().unwrap();
        let mut green = rgb(0x00FF00);
        green.a = 0.5;
        lines.push((path, green.into()));

        // 50% opaque black path that extends across black quad.
        let mut builder = PathBuilder::fill();
        builder.move_to(point(px(350.), px(50.)));
        builder.line_to(point(px(430.), px(50.)));
        builder.line_to(point(px(430.), px(130.)));
        builder.line_to(point(px(350.), px(130.)));
        builder.close();
        let path = builder.build().unwrap();
        let mut black = rgb(0x000000);
        black.a = 0.5;
        lines.push((path, black.into()));

        // Two 50% opaque red circles overlapping - center should be darker red
        let mut builder = PathBuilder::fill();
        let center = point(px(530.), px(85.));
        let radius = px(30.);
        builder.move_to(point(center.x + radius, center.y));
        builder.arc_to(
            point(radius, radius),
            px(0.),
            false,
            false,
            point(center.x - radius, center.y),
        );
        builder.arc_to(
            point(radius, radius),
            px(0.),
            false,
            false,
            point(center.x + radius, center.y),
        );
        builder.close();
        let path = builder.build().unwrap();
        let mut red1 = rgb(0xFF0000);
        red1.a = 0.5;
        lines.push((path, red1.into()));

        let mut builder = PathBuilder::fill();
        let center = point(px(570.), px(85.));
        let radius = px(30.);
        builder.move_to(point(center.x + radius, center.y));
        builder.arc_to(
            point(radius, radius),
            px(0.),
            false,
            false,
            point(center.x - radius, center.y),
        );
        builder.arc_to(
            point(radius, radius),
            px(0.),
            false,
            false,
            point(center.x + radius, center.y),
        );
        builder.close();
        let path = builder.build().unwrap();
        let mut red2 = rgb(0xFF0000);
        red2.a = 0.5;
        lines.push((path, red2.into()));

        // draw a Rust logo
        let mut builder = lyon::path::Path::svg_builder();
        lyon::extra::rust_logo::build_logo_path(&mut builder);
        // move down the Path
        let mut builder: PathBuilder = builder.into();
        builder.translate(point(px(10.), px(200.)));
        builder.scale(0.9);
        let path = builder.build().unwrap();
        lines.push((path, gpui::black().into()));

        // draw a lightening bolt ⚡
        let mut builder = PathBuilder::fill();
        builder.add_polygon(
            &[
                point(px(150.), px(300.)),
                point(px(200.), px(225.)),
                point(px(200.), px(275.)),
                point(px(250.), px(200.)),
            ],
            false,
        );
        let path = builder.build().unwrap();
        lines.push((path, rgb(0x1d4ed8).into()));

        // draw a ⭐
        let mut builder = PathBuilder::fill();
        builder.move_to(point(px(350.), px(200.)));
        builder.line_to(point(px(370.), px(260.)));
        builder.line_to(point(px(430.), px(260.)));
        builder.line_to(point(px(380.), px(300.)));
        builder.line_to(point(px(400.), px(360.)));
        builder.line_to(point(px(350.), px(320.)));
        builder.line_to(point(px(300.), px(360.)));
        builder.line_to(point(px(320.), px(300.)));
        builder.line_to(point(px(270.), px(260.)));
        builder.line_to(point(px(330.), px(260.)));
        builder.line_to(point(px(350.), px(200.)));
        let path = builder.build().unwrap();
        lines.push((
            path,
            linear_gradient(
                180.,
                linear_color_stop(rgb(0xFACC15), 0.7),
                linear_color_stop(rgb(0xD56D0C), 1.),
            )
            .color_space(ColorSpace::Oklab),
        ));

        // draw linear gradient
        let square_bounds = Bounds {
            origin: point(px(450.), px(200.)),
            size: size(px(200.), px(80.)),
        };
        let height = square_bounds.size.height;
        let horizontal_offset = height;
        let vertical_offset = px(30.);
        let mut builder = PathBuilder::fill();
        builder.move_to(square_bounds.bottom_left());
        builder.curve_to(
            square_bounds.origin + point(horizontal_offset, vertical_offset),
            square_bounds.origin + point(px(0.0), vertical_offset),
        );
        builder.line_to(square_bounds.top_right() + point(-horizontal_offset, vertical_offset));
        builder.curve_to(
            square_bounds.bottom_right(),
            square_bounds.top_right() + point(px(0.0), vertical_offset),
        );
        builder.line_to(square_bounds.bottom_left());
        let path = builder.build().unwrap();
        lines.push((
            path,
            linear_gradient(
                180.,
                linear_color_stop(gpui::blue(), 0.4),
                linear_color_stop(gpui::red(), 1.),
            ),
        ));

        // draw a pie chart
        let center = point(px(96.), px(96.));
        let pie_center = point(px(775.), px(255.));
        let segments = [
            (
                point(px(871.), px(255.)),
                point(px(747.), px(163.)),
                rgb(0x1374e9),
            ),
            (
                point(px(747.), px(163.)),
                point(px(679.), px(263.)),
                rgb(0xe13527),
            ),
            (
                point(px(679.), px(263.)),
                point(px(754.), px(349.)),
                rgb(0x0751ce),
            ),
            (
                point(px(754.), px(349.)),
                point(px(854.), px(310.)),
                rgb(0x209742),
            ),
            (
                point(px(854.), px(310.)),
                point(px(871.), px(255.)),
                rgb(0xfbc10a),
            ),
        ];

        for (start, end, color) in segments {
            let mut builder = PathBuilder::fill();
            builder.move_to(start);
            builder.arc_to(center, px(0.), false, false, end);
            builder.line_to(pie_center);
            builder.close();
            let path = builder.build().unwrap();
            lines.push((path, color.into()));
        }

        // draw a wave
        let options = StrokeOptions::default()
            .with_line_width(1.)
            .with_line_join(lyon::path::LineJoin::Bevel);
        let mut builder = PathBuilder::stroke(px(1.)).with_style(PathStyle::Stroke(options));
        builder.move_to(point(px(40.), px(420.)));
        for i in 1..50 {
            builder.line_to(point(
                px(40.0 + i as f32 * 10.0),
                px(420.0 + (i as f32 * 10.0).sin() * 40.0),
            ));
        }
        let path = builder.build().unwrap();
        lines.push((path, gpui::green().into()));

        Self {
            default_lines: lines.clone(),
            background_quads,
            lines: vec![],
            start: point(px(0.), px(0.)),
            dashed: false,
            _painting: false,
        }
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        self.lines.clear();
        cx.notify();
    }
}

fn button(
    text: &str,
    cx: &mut Context<PaintingViewer>,
    on_click: impl Fn(&mut PaintingViewer, &mut Context<PaintingViewer>) + 'static,
) -> impl IntoElement {
    div()
        .id(text.to_string())
        .child(text.to_string())
        .bg(gpui::black())
        .text_color(gpui::white())
        .active(|this| this.opacity(0.8))
        .flex()
        .px_3()
        .py_1()
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
}

impl Render for PaintingViewer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let default_lines = self.default_lines.clone();
        let background_quads = self.background_quads.clone();
        let lines = self.lines.clone();
        let dashed = self.dashed;

        div()
            .bg(gpui::white())
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_between()
                    .items_center()
                    .child("Mouse down any point and drag to draw lines (Hold on shift key to draw straight lines)")
                    .child(
                        div()
                            .flex()
                            .gap_x_2()
                            .child(button(
                                if dashed { "Solid" } else { "Dashed" },
                                cx,
                                move |this, _| this.dashed = !dashed,
                            ))
                            .child(button("Clear", cx, |this, cx| this.clear(cx))),
                    ),
            )
            .child(
                div()
                    .size_full()
                    .child(
                        canvas(
                            move |_, _, _| {},
                            move |_, _, window, _| {
                                // First draw background quads
                                for (bounds, color) in background_quads.iter() {
                                    window.paint_quad(quad(
                                        *bounds,
                                        px(0.),
                                        *color,
                                        px(0.),
                                        gpui::transparent_black(),
                                        Default::default(),
                                    ));
                                }

                                // Then draw the default paths on top
                                for (path, color) in default_lines {
                                    window.paint_path(path, color);
                                }

                                for points in lines {
                                    if points.len() < 2 {
                                        continue;
                                    }

                                    let mut builder = PathBuilder::stroke(px(1.));
                                    if dashed {
                                        builder = builder.dash_array(&[px(4.), px(2.)]);
                                    }
                                    for (i, p) in points.into_iter().enumerate() {
                                        if i == 0 {
                                            builder.move_to(p);
                                        } else {
                                            builder.line_to(p);
                                        }
                                    }

                                    if let Ok(path) = builder.build() {
                                        window.paint_path(path, gpui::black());
                                    }
                                }
                            },
                        )
                        .size_full(),
                    )
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, ev: &MouseDownEvent, _, _| {
                            this._painting = true;
                            this.start = ev.position;
                            let path = vec![ev.position];
                            this.lines.push(path);
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| {
                        if !this._painting {
                            return;
                        }

                        let is_shifted = ev.modifiers.shift;
                        let mut pos = ev.position;
                        // When holding shift, draw a straight line
                        if is_shifted {
                            let dx = pos.x - this.start.x;
                            let dy = pos.y - this.start.y;
                            if dx.abs() > dy.abs() {
                                pos.y = this.start.y;
                            } else {
                                pos.x = this.start.x;
                            }
                        }

                        if let Some(path) = this.lines.last_mut() {
                            path.push(pos);
                        }

                        cx.notify();
                    }))
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _, _, _| {
                            this._painting = false;
                        }),
                    ),
            )
    }
}

fn run_example() {
    application().run(|cx| {
        cx.open_window(
            WindowOptions {
                focus: true,
                ..Default::default()
            },
            |window, cx| cx.new(|cx| PaintingViewer::new(window, cx)),
        )
        .unwrap();
        cx.on_window_closed(|cx, _window_id| {
            cx.quit();
        })
        .detach();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/paths_bench.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    Background, Bounds, ColorSpace, Context, Path, PathBuilder, Pixels, Render, TitlebarOptions,
    Window, WindowBounds, WindowOptions, canvas, div, linear_color_stop, linear_gradient, point,
    prelude::*, px, rgb, size,
};
use gpui_platform::application;

const DEFAULT_WINDOW_WIDTH: Pixels = px(1024.0);
const DEFAULT_WINDOW_HEIGHT: Pixels = px(768.0);

struct PaintingViewer {
    default_lines: Vec<(Path<Pixels>, Background)>,
    _painting: bool,
}

impl PaintingViewer {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let mut lines = vec![];

        // draw a lightening bolt ⚡
        for _ in 0..2000 {
            // draw a ⭐
            let mut builder = PathBuilder::fill();
            builder.move_to(point(px(350.), px(100.)));
            builder.line_to(point(px(370.), px(160.)));
            builder.line_to(point(px(430.), px(160.)));
            builder.line_to(point(px(380.), px(200.)));
            builder.line_to(point(px(400.), px(260.)));
            builder.line_to(point(px(350.), px(220.)));
            builder.line_to(point(px(300.), px(260.)));
            builder.line_to(point(px(320.), px(200.)));
            builder.line_to(point(px(270.), px(160.)));
            builder.line_to(point(px(330.), px(160.)));
            builder.line_to(point(px(350.), px(100.)));
            let path = builder.build().unwrap();
            lines.push((
                path,
                linear_gradient(
                    180.,
                    linear_color_stop(rgb(0xFACC15), 0.7),
                    linear_color_stop(rgb(0xD56D0C), 1.),
                )
                .color_space(ColorSpace::Oklab),
            ));
        }

        Self {
            default_lines: lines,
            _painting: false,
        }
    }
}

impl Render for PaintingViewer {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        window.request_animation_frame();
        let lines = self.default_lines.clone();
        div().size_full().child(
            canvas(
                move |_, _, _| {},
                move |_, _, window, _| {
                    for (path, color) in lines {
                        window.paint_path(path, color);
                    }
                },
            )
            .size_full(),
        )
    }
}

fn run_example() {
    application().run(|cx| {
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Vulkan".into()),
                    ..Default::default()
                }),
                focus: true,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| PaintingViewer::new(window, cx)),
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/pattern.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, AppContext, Bounds, Context, Window, WindowBounds, WindowOptions, div, linear_color_stop,
    linear_gradient, pattern_slash, prelude::*, px, rgb, size,
};
use gpui_platform::application;

struct PatternExample;

impl Render for PatternExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0xffffff))
            .size(px(600.0))
            .justify_center()
            .items_center()
            .shadow_lg()
            .text_xl()
            .text_color(rgb(0x000000))
            .child("Pattern Example")
            .child(
                div()
                    .flex()
                    .flex_col()
                    .border_1()
                    .border_color(gpui::blue())
                    .child(div().w(px(54.0)).h(px(18.0)).bg(pattern_slash(
                        gpui::red(),
                        18.0 / 4.0,
                        18.0 / 4.0,
                    )))
                    .child(div().w(px(54.0)).h(px(18.0)).bg(pattern_slash(
                        gpui::red(),
                        18.0 / 4.0,
                        18.0 / 4.0,
                    )))
                    .child(div().w(px(54.0)).h(px(18.0)).bg(pattern_slash(
                        gpui::red(),
                        18.0 / 4.0,
                        18.0 / 4.0,
                    )))
                    .child(div().w(px(54.0)).h(px(18.0)).bg(pattern_slash(
                        gpui::red(),
                        18.0 / 4.0,
                        18.0 / 2.0,
                    ))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .border_1()
                    .border_color(gpui::blue())
                    .bg(gpui::green().opacity(0.16))
                    .child("Elements the same height should align")
                    .child(div().w(px(256.0)).h(px(56.0)).bg(pattern_slash(
                        gpui::red(),
                        56.0 / 6.0,
                        56.0 / 6.0,
                    )))
                    .child(div().w(px(256.0)).h(px(56.0)).bg(pattern_slash(
                        gpui::green(),
                        56.0 / 6.0,
                        56.0 / 6.0,
                    )))
                    .child(div().w(px(256.0)).h(px(56.0)).bg(pattern_slash(
                        gpui::blue(),
                        56.0 / 6.0,
                        56.0 / 6.0,
                    )))
                    .child(div().w(px(256.0)).h(px(26.0)).bg(pattern_slash(
                        gpui::yellow(),
                        56.0 / 6.0,
                        56.0 / 6.0,
                    ))),
            )
            .child(
                div()
                    .border_1()
                    .border_color(gpui::blue())
                    .w(px(240.0))
                    .h(px(40.0))
                    .bg(gpui::red()),
            )
            .child(
                div()
                    .border_1()
                    .border_color(gpui::blue())
                    .w(px(240.0))
                    .h(px(40.0))
                    .bg(linear_gradient(
                        45.,
                        linear_color_stop(gpui::red(), 0.),
                        linear_color_stop(gpui::blue(), 1.),
                    )),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(600.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| PatternExample),
        )
        .unwrap();

        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/popover.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    Anchor, App, Context, Div, Hsla, Stateful, Window, WindowOptions, anchored, deferred, div,
    prelude::*, px,
};
use gpui_platform::application;

/// An example show use deferred to create a floating layers.
struct HelloWorld {
    open: bool,
    secondary_open: bool,
}

fn button(id: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .bg(gpui::black())
        .text_color(gpui::white())
        .px_3()
        .py_1()
}

fn popover() -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .shadow_lg()
        .p_3()
        .rounded_md()
        .bg(gpui::white())
        .text_color(gpui::black())
        .border_1()
        .text_sm()
        .border_color(gpui::black().opacity(0.1))
}

fn line(color: Hsla) -> Div {
    div().w(px(480.)).h_2().bg(color.opacity(0.25))
}

impl HelloWorld {
    fn render_secondary_popover(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        button("secondary-btn")
            .mt_2()
            .child("Child Popover")
            .on_click(cx.listener(|this, _, _, cx| {
                this.secondary_open = true;
                cx.notify();
            }))
            .when(self.secondary_open, |this| {
                this.child(
                    // Now GPUI supports nested deferred!
                    deferred(
                        anchored()
                            .anchor(Anchor::TopLeft)
                            .snap_to_window_with_margin(px(8.))
                            .child(
                                popover()
                                    .child("This is second level Popover with nested deferred!")
                                    .bg(gpui::white())
                                    .border_color(gpui::blue())
                                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                        this.secondary_open = false;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .priority(2),
                )
            })
    }
}

impl Render for HelloWorld {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .size_full()
            .bg(gpui::white())
            .text_color(gpui::black())
            .justify_center()
            .items_center()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .child(
                        button("popover0").child("Opened Popover").child(
                            deferred(
                                anchored()
                                    .anchor(Anchor::TopLeft)
                                    .snap_to_window_with_margin(px(8.))
                                    .child(popover().w_96().gap_3().child(
                                        "This is a default opened Popover, \
                                        we can use deferred to render it \
                                        in a floating layer.",
                                    )),
                            )
                            .priority(0),
                        ),
                    )
                    .child(
                        button("popover1")
                            .child("Open Popover")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open = true;
                                cx.notify();
                            }))
                            .when(self.open, |this| {
                                this.child(
                                    deferred(
                                        anchored()
                                            .anchor(Anchor::TopLeft)
                                            .snap_to_window_with_margin(px(8.))
                                            .child(
                                                popover()
                                                    .w_96()
                                                    .gap_3()
                                                    .child(
                                                        "This is first level Popover, \
                                                   we can use deferred to render it \
                                                   in a floating layer.\n\
                                                   Click outside to close.",
                                                    )
                                                    .when(!self.secondary_open, |this| {
                                                        this.on_mouse_down_out(cx.listener(
                                                            |this, _, _, cx| {
                                                                this.open = false;
                                                                cx.notify();
                                                            },
                                                        ))
                                                    })
                                                    // Here we need render popover after the content
                                                    // to ensure it will be on top layer.
                                                    .child(
                                                        self.render_secondary_popover(window, cx),
                                                    ),
                                            ),
                                    )
                                    .priority(1),
                                )
                            }),
                    ),
            )
            .child(
                "Here is an example text rendered, \
                to ensure the Popover will float above this contents.",
            )
            .children([
                line(gpui::red()),
                line(gpui::yellow()),
                line(gpui::blue()),
                line(gpui::green()),
            ])
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(|_| HelloWorld {
                open: false,
                secondary_open: false,
            })
        })
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/scrollable.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{App, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px, size};
use gpui_platform::application;

struct Scrollable {}

impl Render for Scrollable {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .id("vertical")
            .p_4()
            .overflow_scroll()
            .bg(gpui::white())
            .child("Example for test 2 way scroll in nested layout")
            .child(
                div()
                    .h(px(5000.))
                    .border_1()
                    .border_color(gpui::blue())
                    .bg(gpui::blue().opacity(0.05))
                    .p_4()
                    .child(
                        div()
                            .mb_5()
                            .w_full()
                            .id("horizontal")
                            .overflow_scroll()
                            .child(
                                div()
                                    .w(px(2000.))
                                    .h(px(150.))
                                    .bg(gpui::green().opacity(0.1))
                                    .hover(|this| this.bg(gpui::green().opacity(0.2)))
                                    .border_1()
                                    .border_color(gpui::green())
                                    .p_4()
                                    .child("Scroll Horizontal"),
                            ),
                    )
                    .child("Scroll Vertical"),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Scrollable {}),
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/set_menus.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Context, Global, Menu, MenuItem, SharedString, SystemMenuType, Window, WindowOptions,
    actions, div, prelude::*,
};
use gpui_platform::application;

struct SetMenus;

impl Render for SetMenus {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .bg(gpui::white())
            .size_full()
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(gpui::black())
            .child("Set Menus Example")
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        cx.set_global(AppState::new());

        // Bring the menu bar to the foreground (so you can see the menu bar)
        cx.activate(true);
        // Register the `quit` function so it can be referenced
        // by the `MenuItem::action` in the menu bar
        cx.on_action(quit);
        cx.on_action(toggle_check);
        // Add menu items
        set_app_menus(cx);
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| SetMenus {}))
            .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

#[derive(PartialEq)]
enum ViewMode {
    List,
    Grid,
}

impl ViewMode {
    fn toggle(&mut self) {
        *self = match self {
            ViewMode::List => ViewMode::Grid,
            ViewMode::Grid => ViewMode::List,
        }
    }
}

impl Into<SharedString> for ViewMode {
    fn into(self) -> SharedString {
        match self {
            ViewMode::List => "List",
            ViewMode::Grid => "Grid",
        }
        .into()
    }
}

struct AppState {
    view_mode: ViewMode,
}

impl AppState {
    fn new() -> Self {
        Self {
            view_mode: ViewMode::List,
        }
    }
}

impl Global for AppState {}

fn set_app_menus(cx: &mut App) {
    let app_state = cx.global::<AppState>();
    cx.set_menus([Menu::new("set_menus").items([
        MenuItem::os_submenu("Services", SystemMenuType::Services),
        MenuItem::separator(),
        MenuItem::action("Disabled Item", gpui::NoAction).disabled(true),
        MenuItem::submenu(Menu::new("Disabled Submenu").disabled(true)),
        MenuItem::separator(),
        MenuItem::action("List Mode", ToggleCheck).checked(app_state.view_mode == ViewMode::List),
        MenuItem::submenu(
            Menu::new("Mode").items([
                MenuItem::action(ViewMode::List, ToggleCheck)
                    .checked(app_state.view_mode == ViewMode::List),
                MenuItem::action(ViewMode::Grid, ToggleCheck)
                    .checked(app_state.view_mode == ViewMode::Grid),
            ]),
        ),
        MenuItem::separator(),
        MenuItem::action("Quit", Quit),
    ])]);
}

// Associate actions using the `actions!` macro (or `Action` derive macro)
actions!(set_menus, [Quit, ToggleCheck]);

// Define the quit function that is registered with the App
fn quit(_: &Quit, cx: &mut App) {
    println!("Gracefully quitting the application...");
    cx.quit();
}

fn toggle_check(_: &ToggleCheck, cx: &mut App) {
    let app_state = cx.global_mut::<AppState>();
    app_state.view_mode.toggle();
    set_app_menus(cx);
}

```
---
## `example_file:crates/gpui/examples/shadow.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, BoxShadow, Context, Div, SharedString, Window, WindowBounds, WindowOptions, div,
    hsla, point, prelude::*, px, relative, rgb, size,
};
use gpui_platform::application;

struct Shadow {}

impl Shadow {
    fn base() -> Div {
        div()
            .size_16()
            .bg(rgb(0xffffff))
            .rounded_full()
            .border_1()
            .border_color(hsla(0.0, 0.0, 0.0, 0.1))
    }

    fn square() -> Div {
        div()
            .size_16()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(hsla(0.0, 0.0, 0.0, 0.1))
    }

    fn rounded_small() -> Div {
        div()
            .size_16()
            .bg(rgb(0xffffff))
            .rounded(px(4.))
            .border_1()
            .border_color(hsla(0.0, 0.0, 0.0, 0.1))
    }

    fn rounded_medium() -> Div {
        div()
            .size_16()
            .bg(rgb(0xffffff))
            .rounded(px(8.))
            .border_1()
            .border_color(hsla(0.0, 0.0, 0.0, 0.1))
    }

    fn rounded_large() -> Div {
        div()
            .size_16()
            .bg(rgb(0xffffff))
            .rounded(px(12.))
            .border_1()
            .border_color(hsla(0.0, 0.0, 0.0, 0.1))
    }
}

fn example(label: impl Into<SharedString>, example: impl IntoElement) -> impl IntoElement {
    let label = label.into();

    div()
        .flex()
        .flex_col()
        .justify_center()
        .items_center()
        .w(relative(1. / 6.))
        .border_r_1()
        .border_color(hsla(0.0, 0.0, 0.0, 1.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .flex_1()
                .py_12()
                .child(example),
        )
        .child(
            div()
                .w_full()
                .border_t_1()
                .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                .p_1()
                .flex()
                .items_center()
                .child(label),
        )
}

impl Render for Shadow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("shadow-example")
            .overflow_y_scroll()
            .bg(rgb(0xffffff))
            .size_full()
            .text_xs()
            .child(div().flex().flex_col().w_full().children(vec![
                div()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                    .flex()
                    .flex_row()
                    .children(vec![
                        example(
                            "Square",
                            Shadow::square()
                                .shadow(vec![BoxShadow {
                                    color: hsla(0.0, 0.5, 0.5, 0.3),
                                    offset: point(px(0.), px(8.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(0.),
                                }]),
                        ),
                        example(
                            "Rounded 4",
                            Shadow::rounded_small()
                                .shadow(vec![BoxShadow {
                                    color: hsla(0.0, 0.5, 0.5, 0.3),
                                    offset: point(px(0.), px(8.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(0.),
                                }]),
                        ),
                        example(
                            "Rounded 8",
                            Shadow::rounded_medium()
                                .shadow(vec![BoxShadow {
                                    color: hsla(0.0, 0.5, 0.5, 0.3),
                                    offset: point(px(0.), px(8.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(0.),
                                }]),
                        ),
                        example(
                            "Rounded 16",
                            Shadow::rounded_large()
                                .shadow(vec![BoxShadow {
                                    color: hsla(0.0, 0.5, 0.5, 0.3),
                                    offset: point(px(0.), px(8.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(0.),
                                }]),
                        ),
                        example(
                            "Circle",
                            Shadow::base()
                                .shadow(vec![BoxShadow {
                                    color: hsla(0.0, 0.5, 0.5, 0.3),
                                    offset: point(px(0.), px(8.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(0.),
                                }]),
                        ),
                    ]),
                div()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                    .flex()
                    .w_full()
                    .children(vec![
                        example("None", Shadow::base()),
                        // 2Xsmall shadow
                        example("2X Small", Shadow::base().shadow_2xs()),
                        // Xsmall shadow
                        example("Extra Small", Shadow::base().shadow_xs()),
                        // Small shadow
                        example("Small", Shadow::base().shadow_sm()),
                        // Medium shadow
                        example("Medium", Shadow::base().shadow_md()),
                        // Large shadow
                        example("Large", Shadow::base().shadow_lg()),
                        example("Extra Large", Shadow::base().shadow_xl()),
                        example("2X Large", Shadow::base().shadow_2xl()),
                    ]),
                // Horizontal list of increasing blur radii
                div()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                    .flex()
                    .children(vec![
                        example(
                            "Blur 0",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(0.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Blur 2",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(2.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Blur 4",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(4.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Blur 8",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Blur 16",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(16.),
                                spread_radius: px(0.),
                            }]),
                        ),
                    ]),
                // Horizontal list of increasing spread radii
                div()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                    .flex()
                    .children(vec![
                        example(
                            "Spread 0",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Spread 2",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(2.),
                            }]),
                        ),
                        example(
                            "Spread 4",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(4.),
                            }]),
                        ),
                        example(
                            "Spread 8",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(8.),
                            }]),
                        ),
                        example(
                            "Spread 16",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(16.),
                            }]),
                        ),
                    ]),
                // Square spread examples
                div()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                    .flex()
                    .children(vec![
                        example(
                            "Square Spread 0",
                            Shadow::square().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Square Spread 8",
                            Shadow::square().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(8.),
                            }]),
                        ),
                        example(
                            "Square Spread 16",
                            Shadow::square().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(16.),
                            }]),
                        ),
                    ]),
                // Rounded large spread examples
                div()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                    .flex()
                    .children(vec![
                        example(
                            "Rounded Large Spread 0",
                            Shadow::rounded_large().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Rounded Large Spread 8",
                            Shadow::rounded_large().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(8.),
                            }]),
                        ),
                        example(
                            "Rounded Large Spread 16",
                            Shadow::rounded_large().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.0, 0.0, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(16.),
                            }]),
                        ),
                    ]),
                // Directional shadows
                div()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                    .flex()
                    .children(vec![
                        example(
                            "Left",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(-8.), px(0.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Right",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(8.), px(0.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Top",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(0.), px(-8.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Bottom",
                            Shadow::base().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                    ]),
                // Square directional shadows
                div()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                    .flex()
                    .children(vec![
                        example(
                            "Square Left",
                            Shadow::square().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(-8.), px(0.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Square Right",
                            Shadow::square().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(8.), px(0.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Square Top",
                            Shadow::square().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(0.), px(-8.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Square Bottom",
                            Shadow::square().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                    ]),
                // Rounded large directional shadows
                div()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                    .flex()
                    .children(vec![
                        example(
                            "Rounded Large Left",
                            Shadow::rounded_large().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(-8.), px(0.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Rounded Large Right",
                            Shadow::rounded_large().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(8.), px(0.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Rounded Large Top",
                            Shadow::rounded_large().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(0.), px(-8.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                        example(
                            "Rounded Large Bottom",
                            Shadow::rounded_large().shadow(vec![BoxShadow {
                                color: hsla(0.0, 0.5, 0.5, 0.3),
                                offset: point(px(0.), px(8.)),
                                blur_radius: px(8.),
                                spread_radius: px(0.),
                            }]),
                        ),
                    ]),
                // Multiple shadows for different shapes
                div()
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.0, 1.0))
                    .flex()
                    .children(vec![
                        example(
                            "Circle Multiple",
                            Shadow::base().shadow(vec![
                                BoxShadow {
                                    color: hsla(0.0 / 360., 1.0, 0.5, 0.3), // Red
                                    offset: point(px(0.), px(-12.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                                BoxShadow {
                                    color: hsla(60.0 / 360., 1.0, 0.5, 0.3), // Yellow
                                    offset: point(px(12.), px(0.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                                BoxShadow {
                                    color: hsla(120.0 / 360., 1.0, 0.5, 0.3), // Green
                                    offset: point(px(0.), px(12.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                                BoxShadow {
                                    color: hsla(240.0 / 360., 1.0, 0.5, 0.3), // Blue
                                    offset: point(px(-12.), px(0.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                            ]),
                        ),
                        example(
                            "Square Multiple",
                            Shadow::square().shadow(vec![
                                BoxShadow {
                                    color: hsla(0.0 / 360., 1.0, 0.5, 0.3), // Red
                                    offset: point(px(0.), px(-12.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                                BoxShadow {
                                    color: hsla(60.0 / 360., 1.0, 0.5, 0.3), // Yellow
                                    offset: point(px(12.), px(0.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                                BoxShadow {
                                    color: hsla(120.0 / 360., 1.0, 0.5, 0.3), // Green
                                    offset: point(px(0.), px(12.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                                BoxShadow {
                                    color: hsla(240.0 / 360., 1.0, 0.5, 0.3), // Blue
                                    offset: point(px(-12.), px(0.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                            ]),
                        ),
                        example(
                            "Rounded Large Multiple",
                            Shadow::rounded_large().shadow(vec![
                                BoxShadow {
                                    color: hsla(0.0 / 360., 1.0, 0.5, 0.3), // Red
                                    offset: point(px(0.), px(-12.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                                BoxShadow {
                                    color: hsla(60.0 / 360., 1.0, 0.5, 0.3), // Yellow
                                    offset: point(px(12.), px(0.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                                BoxShadow {
                                    color: hsla(120.0 / 360., 1.0, 0.5, 0.3), // Green
                                    offset: point(px(0.), px(12.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                                BoxShadow {
                                    color: hsla(240.0 / 360., 1.0, 0.5, 0.3), // Blue
                                    offset: point(px(-12.), px(0.)),
                                    blur_radius: px(8.),
                                    spread_radius: px(2.),
                                },
                            ]),
                        ),
                    ]),
            ]))
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1000.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Shadow {}),
        )
        .unwrap();

        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/svg/svg.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use gpui::{
    App, AssetSource, Bounds, Context, SharedString, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, rgb, size, svg,
};
use gpui_platform::application;

struct Assets {
    base: PathBuf,
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        fs::read(self.base.join(path))
            .map(|data| Some(std::borrow::Cow::Owned(data)))
            .map_err(|err| err.into())
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        fs::read_dir(self.base.join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(|err| err.into())
    }
}

struct SvgExample;

impl Render for SvgExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .size_full()
            .justify_center()
            .items_center()
            .gap_8()
            .bg(rgb(0xffffff))
            .child(
                svg()
                    .path("svg/dragon.svg")
                    .size_8()
                    .text_color(rgb(0xff0000)),
            )
            .child(
                svg()
                    .path("svg/dragon.svg")
                    .size_8()
                    .text_color(rgb(0x00ff00)),
            )
            .child(
                svg()
                    .path("svg/dragon.svg")
                    .size_8()
                    .text_color(rgb(0x0000ff)),
            )
    }
}

fn run_example() {
    application()
        .with_assets(Assets {
            base: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples"),
        })
        .run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(300.0), px(300.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| SvgExample),
            )
            .unwrap();
            cx.activate(true);
        });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/tab_stop.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, Div, ElementId, FocusHandle, KeyBinding, SharedString, Stateful, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, size,
};
use gpui_platform::application;

actions!(example, [Tab, TabPrev]);

struct Example {
    focus_handle: FocusHandle,
    items: Vec<FocusHandle>,
    message: SharedString,
}

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = vec![
            cx.focus_handle().tab_index(1).tab_stop(true),
            cx.focus_handle().tab_index(2).tab_stop(true),
            cx.focus_handle().tab_index(3).tab_stop(true),
            cx.focus_handle(),
            cx.focus_handle().tab_index(2).tab_stop(true),
        ];

        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);

        Self {
            focus_handle,
            items,
            message: SharedString::from("Press `Tab`, `Shift-Tab` to switch focus."),
        }
    }

    fn on_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_next(cx);
        self.message = SharedString::from("You have pressed `Tab`.");
    }

    fn on_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_prev(cx);
        self.message = SharedString::from("You have pressed `Shift-Tab`.");
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        fn tab_stop_style<T: Styled>(this: T) -> T {
            this.border_3().border_color(gpui::blue())
        }

        fn button(id: impl Into<ElementId>) -> Stateful<Div> {
            div()
                .id(id)
                .h_10()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .border_1()
                .border_color(gpui::black())
                .bg(gpui::black())
                .text_color(gpui::white())
                .focus(tab_stop_style)
                .shadow_sm()
        }

        div()
            .id("app")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_tab))
            .on_action(cx.listener(Self::on_tab_prev))
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_3()
            .bg(gpui::white())
            .text_color(gpui::black())
            .child(self.message.clone())
            .children(
                self.items
                    .clone()
                    .into_iter()
                    .enumerate()
                    .map(|(ix, item_handle)| {
                        div()
                            .id(("item", ix))
                            .track_focus(&item_handle)
                            .h_10()
                            .w_full()
                            .flex()
                            .justify_center()
                            .items_center()
                            .border_1()
                            .border_color(gpui::black())
                            .when(
                                item_handle.tab_stop && item_handle.is_focused(window),
                                tab_stop_style,
                            )
                            .map(|this| match item_handle.tab_stop {
                                true => this
                                    .hover(|this| this.bg(gpui::black().opacity(0.1)))
                                    .child(format!("tab_index: {}", item_handle.tab_index)),
                                false => this.opacity(0.4).child("tab_stop: false"),
                            })
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .items_center()
                    .child(
                        button("el1")
                            .tab_index(4)
                            .child("Button 1")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.message = "You have clicked Button 1.".into();
                                cx.notify();
                            })),
                    )
                    .child(
                        button("el2")
                            .tab_index(5)
                            .child("Button 2")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.message = "You have clicked Button 2.".into();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("group-1")
                    .tab_index(6)
                    .tab_group()
                    .tab_stop(false)
                    .child(
                        button("group-1-button-1")
                            .tab_index(1)
                            .child("Tab index [6, 1]"),
                    )
                    .child(
                        button("group-1-button-2")
                            .tab_index(2)
                            .child("Tab index [6, 2]"),
                    )
                    .child(
                        button("group-1-button-3")
                            .tab_index(3)
                            .child("Tab index [6, 3]"),
                    ),
            )
            .child(
                div()
                    .id("group-2")
                    .tab_index(7)
                    .tab_group()
                    .tab_stop(false)
                    .child(
                        button("group-2-button-1")
                            .tab_index(1)
                            .child("Tab index [7, 1]"),
                    )
                    .child(
                        button("group-2-button-2")
                            .tab_index(2)
                            .child("Tab index [7, 2]"),
                    )
                    .child(
                        button("group-2-button-3")
                            .tab_index(3)
                            .child("Tab index [7, 3]"),
                    ),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("tab", Tab, None),
            KeyBinding::new("shift-tab", TabPrev, None),
        ]);

        let bounds = Bounds::centered(None, size(px(800.), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Example::new(window, cx)),
        )
        .unwrap();

        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/testing.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]
//! Example demonstrating GPUI's testing infrastructure.
//!
//! When run normally, this displays an interactive counter window.
//! The tests below demonstrate various GPUI testing patterns.
//!
//! Run the app: cargo run -p gpui --example testing
//! Run tests:   cargo test -p gpui --example testing --features test-support

use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, Render, Task, Window, WindowBounds,
    WindowOptions, actions, div, prelude::*, px, rgb, size,
};
use gpui_platform::application;

actions!(counter, [Increment, Decrement]);

struct Counter {
    count: i32,
    focus_handle: FocusHandle,
    _subscription: gpui::Subscription,
}

/// Event emitted by Counter
struct CounterEvent;

impl gpui::EventEmitter<CounterEvent> for Counter {}

impl Counter {
    fn new(cx: &mut Context<Self>) -> Self {
        let subscription = cx.subscribe_self(|this: &mut Self, _event: &CounterEvent, _cx| {
            this.count = 999;
        });

        Self {
            count: 0,
            focus_handle: cx.focus_handle(),
            _subscription: subscription,
        }
    }

    fn increment(&mut self, _: &Increment, _window: &mut Window, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }

    fn decrement(&mut self, _: &Decrement, _window: &mut Window, cx: &mut Context<Self>) {
        self.count -= 1;
        cx.notify();
    }

    fn load(&self, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            // Simulate loading data (e.g., from disk or network)
            this.update(cx, |counter, _| {
                counter.count = 100;
            })
            .ok();
        })
    }

    fn reload(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            // Simulate reloading data in the background
            this.update(cx, |counter, _| {
                counter.count += 50;
            })
            .ok();
        })
        .detach();
    }
}

impl Focusable for Counter {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Counter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("counter")
            .key_context("Counter")
            .on_action(cx.listener(Self::increment))
            .on_action(cx.listener(Self::decrement))
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .gap_4()
            .bg(rgb(0x1e1e2e))
            .size_full()
            .justify_center()
            .items_center()
            .child(
                div()
                    .text_3xl()
                    .text_color(rgb(0xcdd6f4))
                    .child(format!("{}", self.count)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("decrement")
                            .px_4()
                            .py_2()
                            .bg(rgb(0x313244))
                            .hover(|s| s.bg(rgb(0x45475a)))
                            .rounded_md()
                            .cursor_pointer()
                            .text_color(rgb(0xcdd6f4))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.decrement(&Decrement, window, cx)
                            }))
                            .child("−"),
                    )
                    .child(
                        div()
                            .id("increment")
                            .px_4()
                            .py_2()
                            .bg(rgb(0x313244))
                            .hover(|s| s.bg(rgb(0x45475a)))
                            .rounded_md()
                            .cursor_pointer()
                            .text_color(rgb(0xcdd6f4))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.increment(&Increment, window, cx)
                            }))
                            .child("+"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("load")
                            .px_4()
                            .py_2()
                            .bg(rgb(0x313244))
                            .hover(|s| s.bg(rgb(0x45475a)))
                            .rounded_md()
                            .cursor_pointer()
                            .text_color(rgb(0xcdd6f4))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.load(cx).detach();
                            }))
                            .child("Load"),
                    )
                    .child(
                        div()
                            .id("reload")
                            .px_4()
                            .py_2()
                            .bg(rgb(0x313244))
                            .hover(|s| s.bg(rgb(0x45475a)))
                            .rounded_md()
                            .cursor_pointer()
                            .text_color(rgb(0xcdd6f4))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.reload(cx);
                            }))
                            .child("Reload"),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6c7086))
                    .child("Press ↑/↓ or click buttons"),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        cx.bind_keys([
            gpui::KeyBinding::new("up", Increment, Some("Counter")),
            gpui::KeyBinding::new("down", Decrement, Some("Counter")),
        ]);

        let bounds = Bounds::centered(None, size(px(300.), px(200.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let counter = cx.new(|cx| Counter::new(cx));
                counter.focus_handle(cx).focus(window, cx);
                counter
            },
        )
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};
    use rand::prelude::*;

    /// Here's a basic GPUI test. Just add the macro and take a TestAppContext as an argument!
    ///
    /// Note that synchronous side effects run immediately after your "update*" calls complete.
    #[gpui::test]
    fn basic_testing(cx: &mut TestAppContext) {
        let counter = cx.new(|cx| Counter::new(cx));

        counter.update(cx, |counter, _| {
            counter.count = 42;
        });

        // Note that TestAppContext doesn't support `read(cx)`
        let updated = counter.read_with(cx, |counter, _| counter.count);
        assert_eq!(updated, 42);

        // Emit an event - the subscriber will run immediately after the update finishes
        counter.update(cx, |_, cx| {
            cx.emit(CounterEvent);
        });

        let count_after_update = counter.read_with(cx, |counter, _| counter.count);
        assert_eq!(
            count_after_update, 999,
            "Side effects should run after update completes"
        );
    }

    /// Tests which involve the window require you to construct a VisualTestContext.
    /// Just like synchronous side effects, the window will be drawn after every "update*"
    /// call, so you can test render-dependent behavior.
    #[gpui::test]
    fn test_counter_in_window(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|cx| Counter::new(cx)))
                .unwrap()
        });

        let mut cx = VisualTestContext::from_window(window.into(), cx);
        let counter = window.root(&mut cx).unwrap();

        // Action dispatch depends on the element tree to resolve which action handler
        // to call, and this works exactly as you'd expect in a test.
        let focus_handle = counter.read_with(&cx, |counter, _| counter.focus_handle.clone());
        cx.update(|window, cx| {
            focus_handle.dispatch_action(&Increment, window, cx);
        });

        let count_after = counter.read_with(&cx, |counter, _| counter.count);
        assert_eq!(
            count_after, 1,
            "Action dispatched via focus handle should increment"
        );
    }

    /// GPUI tests can also be async, simply add the async keyword before the test.
    /// Note that the test executor is single thread, so async side effects (including
    /// background tasks) won't run until you explicitly yield control.
    #[gpui::test]
    async fn test_async_operations(cx: &mut TestAppContext) {
        let counter = cx.new(|cx| Counter::new(cx));

        // Tasks can be awaited directly
        counter.update(cx, |counter, cx| counter.load(cx)).await;

        let count = counter.read_with(cx, |counter, _| counter.count);
        assert_eq!(count, 100, "Load task should have set count to 100");

        // But side effects don't run until you yield control
        counter.update(cx, |counter, cx| counter.reload(cx));

        let count = counter.read_with(cx, |counter, _| counter.count);
        assert_eq!(count, 100, "Detached reload task shouldn't have run yet");

        // This runs all pending tasks
        cx.run_until_parked();

        let count = counter.read_with(cx, |counter, _| counter.count);
        assert_eq!(count, 150, "Reload task should have run after parking");
    }

    /// Note that the test executor panics if you await a future that waits on
    /// something outside GPUI's control, like a reading a file or network IO.
    /// You should mock external systems where possible, as this feature can be used
    /// to detect potential deadlocks in your async code.
    ///
    /// However, if you want to disable this check use `allow_parking()`
    #[gpui::test]
    async fn test_allow_parking(cx: &mut TestAppContext) {
        // Allow the thread to park
        cx.executor().allow_parking();

        // Simulate an external system (like a file system) with an OS thread
        let (tx, rx) = futures::channel::oneshot::channel();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(5));
            tx.send(42).ok();
        });

        // Without allow_parking(), this await would panic because GPUI's
        // scheduler runs out of tasks while waiting for the external thread.
        let result = rx.await.unwrap();
        assert_eq!(result, 42);
    }

    /// GPUI also provides support for property testing, via the iterations flag
    #[gpui::test(iterations = 10)]
    fn test_counter_random_operations(cx: &mut TestAppContext, mut rng: StdRng) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|cx| Counter::new(cx)))
                .unwrap()
        });
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        let counter = cx.new(|cx| Counter::new(cx));

        // Perform random increments/decrements
        let mut expected = 0i32;
        for _ in 0..100 {
            if rng.random_bool(0.5) {
                expected += 1;
                counter.update_in(&mut cx, |counter, window, cx| {
                    counter.increment(&Increment, window, cx)
                });
            } else {
                expected -= 1;
                counter.update_in(&mut cx, |counter, window, cx| {
                    counter.decrement(&Decrement, window, cx)
                });
            }
        }

        let actual = counter.read_with(&cx, |counter, _| counter.count);
        assert_eq!(
            actual, expected,
            "Counter should match expected after random ops"
        );
    }

    /// Now, all of those tests are good, but GPUI also provides strong support for testing distributed systems.
    /// Let's setup a mock network and enhance the counter to send messages over it.
    mod distributed_systems {
        use std::sync::{Arc, Mutex};

        /// The state of the mock network.
        struct MockNetworkState {
            ordering: Vec<i32>,
            a_to_b: Vec<i32>,
            b_to_a: Vec<i32>,
        }

        /// A mock network that delivers messages between two peers.
        #[derive(Clone)]
        struct MockNetwork {
            state: Arc<Mutex<MockNetworkState>>,
        }

        impl MockNetwork {
            fn new() -> Self {
                Self {
                    state: Arc::new(Mutex::new(MockNetworkState {
                        ordering: Vec::new(),
                        a_to_b: Vec::new(),
                        b_to_a: Vec::new(),
                    })),
                }
            }

            fn a_client(&self) -> NetworkClient {
                NetworkClient {
                    network: self.clone(),
                    is_a: true,
                }
            }

            fn b_client(&self) -> NetworkClient {
                NetworkClient {
                    network: self.clone(),
                    is_a: false,
                }
            }
        }

        /// A client handle for sending/receiving messages over the mock network.
        #[derive(Clone)]
        struct NetworkClient {
            network: MockNetwork,
            is_a: bool,
        }

        // See, networking is easy!
        impl NetworkClient {
            fn send(&self, value: i32) {
                let mut network = self.network.state.lock().unwrap();
                network.ordering.push(value);
                if self.is_a {
                    network.b_to_a.push(value);
                } else {
                    network.a_to_b.push(value);
                }
            }

            fn receive_all(&self) -> Vec<i32> {
                let mut network = self.network.state.lock().unwrap();
                if self.is_a {
                    network.a_to_b.drain(..).collect()
                } else {
                    network.b_to_a.drain(..).collect()
                }
            }
        }

        use gpui::Context;

        /// A networked counter that can send/receive over a mock network.
        struct NetworkedCounter {
            count: i32,
            client: NetworkClient,
        }

        impl NetworkedCounter {
            fn new(client: NetworkClient) -> Self {
                Self { count: 0, client }
            }

            /// Increment the counter and broadcast the change.
            fn increment(&mut self, delta: i32, cx: &mut Context<Self>) {
                self.count += delta;

                cx.background_spawn({
                    let client = self.client.clone();
                    async move {
                        client.send(delta);
                    }
                })
                .detach();
            }

            /// Process incoming increment requests.
            fn sync(&mut self) {
                for delta in self.client.receive_all() {
                    self.count += delta;
                }
            }
        }

        use super::*;

        /// You can simulate distributed systems with multiple app contexts, simply by adding
        /// additional parameters.
        #[gpui::test]
        fn test_app_sync(cx_a: &mut TestAppContext, cx_b: &mut TestAppContext) {
            let network = MockNetwork::new();

            let a = cx_a.new(|_| NetworkedCounter::new(network.a_client()));
            let b = cx_b.new(|_| NetworkedCounter::new(network.b_client()));

            // B increments locally and broadcasts the delta
            b.update(cx_b, |b, cx| b.increment(42, cx));
            b.read_with(cx_b, |b, _| assert_eq!(b.count, 42)); // B's count is set immediately
            a.read_with(cx_a, |a, _| assert_eq!(a.count, 0)); // A's count is in a side effect

            cx_b.run_until_parked(); // Send the delta from B
            a.update(cx_a, |a, _| a.sync()); // Receive the delta at A

            b.read_with(cx_b, |b, _| assert_eq!(b.count, 42)); // Both counts now match
            a.read_with(cx_a, |a, _| assert_eq!(a.count, 42));
        }

        /// Multiple apps can run concurrently, and to capture this each test app shares
        /// a dispatcher. Whenever you call `run_until_parked`, the dispatcher will randomly
        /// pick which app's tasks to run next. This allows you to test that your distributed code
        /// is robust to different execution orderings.
        #[gpui::test(iterations = 10)]
        fn test_random_interleaving(
            cx_a: &mut TestAppContext,
            cx_b: &mut TestAppContext,
            mut rng: StdRng,
        ) {
            let network = MockNetwork::new();

            // Track execution order
            let mut original_order = Vec::new();
            let a = cx_a.new(|_| NetworkedCounter::new(MockNetwork::a_client(&network)));
            let b = cx_b.new(|_| NetworkedCounter::new(MockNetwork::b_client(&network)));

            let num_operations: usize = rng.random_range(3..8);

            for i in 0..num_operations {
                let i = i as i32;
                let which = rng.random_bool(0.5);

                original_order.push(i);
                if which {
                    b.update(cx_b, |b, cx| b.increment(i, cx));
                } else {
                    a.update(cx_a, |a, cx| a.increment(i, cx));
                }
            }

            // This will send all of the pending increment messages, from both a and b
            cx_a.run_until_parked();

            a.update(cx_a, |a, _| a.sync());
            b.update(cx_b, |b, _| b.sync());

            let a_count = a.read_with(cx_a, |a, _| a.count);
            let b_count = b.read_with(cx_b, |b, _| b.count);

            assert_eq!(a_count, b_count, "A and B should have the same count");

            // Nicely format the execution order output.
            // Run this test with `-- --nocapture` to see it!
            let actual = network.state.lock().unwrap().ordering.clone();
            let spawned: Vec<_> = original_order.iter().map(|n| format!("{}", n)).collect();
            let ran: Vec<_> = actual.iter().map(|n| format!("{}", n)).collect();
            let diff: Vec<_> = original_order
                .iter()
                .zip(actual.iter())
                .map(|(o, a)| {
                    if o == a {
                        " ".to_string()
                    } else {
                        "^".to_string()
                    }
                })
                .collect();
            println!("spawned: [{}]", spawned.join(", "));
            println!("ran:     [{}]", ran.join(", "));
            println!("         [{}]", diff.join(", "));
        }
    }
}

```
---
## `example_file:crates/gpui/examples/text.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use gpui::{
    AbsoluteLength, App, Context, DefiniteLength, ElementId, Global, Hsla, Menu, SharedString,
    TextStyle, TitlebarOptions, Window, WindowBounds, WindowOptions, bounds, colors::DefaultColors,
    div, point, prelude::*, px, relative, rgb, size,
};
use gpui_platform::application;
use std::iter;

#[derive(Clone, Debug)]
pub struct TextContext {
    font_size: f32,
    line_height: f32,
    type_scale: f32,
}

impl Default for TextContext {
    fn default() -> Self {
        TextContext {
            font_size: 16.0,
            line_height: 1.3,
            type_scale: 1.33,
        }
    }
}

impl TextContext {
    pub fn get_global(cx: &App) -> &Arc<TextContext> {
        &cx.global::<GlobalTextContext>().0
    }
}

#[derive(Clone, Debug)]
pub struct GlobalTextContext(pub Arc<TextContext>);

impl Deref for GlobalTextContext {
    type Target = Arc<TextContext>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GlobalTextContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Global for GlobalTextContext {}

pub trait ActiveTextContext {
    fn text_context(&self) -> &Arc<TextContext>;
}

impl ActiveTextContext for App {
    fn text_context(&self) -> &Arc<TextContext> {
        &self.global::<GlobalTextContext>().0
    }
}

#[derive(Clone, PartialEq)]
pub struct SpecimenTheme {
    pub bg: Hsla,
    pub fg: Hsla,
}

impl Default for SpecimenTheme {
    fn default() -> Self {
        Self {
            bg: gpui::white(),
            fg: gpui::black(),
        }
    }
}

impl SpecimenTheme {
    pub fn invert(&self) -> Self {
        Self {
            bg: self.fg,
            fg: self.bg,
        }
    }
}

#[derive(Debug, Clone, PartialEq, IntoElement)]
struct Specimen {
    id: ElementId,
    scale: f32,
    text_style: Option<TextStyle>,
    string: SharedString,
    invert: bool,
}

impl Specimen {
    pub fn new(id: usize) -> Self {
        let string = SharedString::new_static("The quick brown fox jumps over the lazy dog");
        let id_string = format!("specimen-{}", id);
        let id = ElementId::Name(id_string.into());
        Self {
            id,
            scale: 1.0,
            text_style: None,
            string,
            invert: false,
        }
    }

    pub fn invert(mut self) -> Self {
        self.invert = !self.invert;
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
}

impl RenderOnce for Specimen {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let rem_size = window.rem_size();
        let scale = self.scale;
        let global_style = cx.text_context();

        let style_override = self.text_style;

        let mut font_size = global_style.font_size;
        let mut line_height = global_style.line_height;

        if let Some(style_override) = style_override {
            font_size = style_override.font_size.to_pixels(rem_size).into();
            line_height = match style_override.line_height {
                DefiniteLength::Absolute(absolute_len) => match absolute_len {
                    AbsoluteLength::Rems(absolute_len) => absolute_len.to_pixels(rem_size).into(),
                    AbsoluteLength::Pixels(absolute_len) => absolute_len.into(),
                },
                DefiniteLength::Fraction(value) => value,
            };
        }

        let mut theme = SpecimenTheme::default();

        if self.invert {
            theme = theme.invert();
        }

        div()
            .id(self.id)
            .bg(theme.bg)
            .text_color(theme.fg)
            .text_size(px(font_size * scale))
            .line_height(relative(line_height))
            .p(px(10.0))
            .child(self.string)
    }
}

#[derive(Debug, Clone, PartialEq, IntoElement)]
struct CharacterGrid {
    scale: f32,
    invert: bool,
    text_style: Option<TextStyle>,
}

impl CharacterGrid {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            invert: false,
            text_style: None,
        }
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
}

impl RenderOnce for CharacterGrid {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut theme = SpecimenTheme::default();

        if self.invert {
            theme = theme.invert();
        }

        let characters = vec![
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "A", "B", "C", "D", "E", "F", "G",
            "H", "I", "J", "K", "L", "M", "N", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y",
            "Z", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "p", "q",
            "r", "s", "t", "u", "v", "w", "x", "y", "z", "ẞ", "ſ", "ß", "ð", "Þ", "þ", "α", "β",
            "Γ", "γ", "Δ", "δ", "η", "θ", "ι", "κ", "Λ", "λ", "μ", "ν", "ξ", "π", "τ", "υ", "φ",
            "χ", "ψ", "∂", "а", "в", "Ж", "ж", "З", "з", "К", "к", "л", "м", "Н", "н", "Р", "р",
            "У", "у", "ф", "ч", "ь", "ы", "Э", "э", "Я", "я", "ij", "öẋ", ".,", "⣝⣑", "~", "*",
            "_", "^", "`", "'", "(", "{", "«", "#", "&", "@", "$", "¢", "%", "|", "?", "¶", "µ",
            "❮", "<=", "!=", "==", "--", "++", "=>", "->", "🏀", "🎊", "😍", "❤️", "👍", "👎",
        ];

        let columns = 20;
        let rows = characters.len().div_ceil(columns);

        let grid_rows = (0..rows).map(|row_idx| {
            let start_idx = row_idx * columns;
            let end_idx = (start_idx + columns).min(characters.len());

            div()
                .w_full()
                .flex()
                .flex_row()
                .children((start_idx..end_idx).map(|i| {
                    div()
                        .text_center()
                        .size(px(62.))
                        .bg(theme.bg)
                        .text_color(theme.fg)
                        .text_size(px(24.0))
                        .line_height(relative(1.0))
                        .child(characters[i])
                }))
                .when(end_idx - start_idx < columns, |d| {
                    d.children(
                        iter::repeat_with(|| div().flex_1()).take(columns - (end_idx - start_idx)),
                    )
                })
        });

        div().p_4().gap_2().flex().flex_col().children(grid_rows)
    }
}

struct TextExample {
    next_id: usize,
    font_family: SharedString,
}

impl TextExample {
    fn next_id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id
    }

    fn button(
        text: &str,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .id(text.to_string())
            .flex_none()
            .child(text.to_string())
            .bg(gpui::black())
            .text_color(gpui::white())
            .active(|this| this.opacity(0.8))
            .px_3()
            .py_1()
            .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
    }
}

const FONT_FAMILIES: [&str; 5] = [
    ".ZedMono",
    ".SystemUIFont",
    "Menlo",
    "Monaco",
    "Courier New",
];

impl Render for TextExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tcx = cx.text_context();
        let colors = cx.default_colors().clone();

        let type_scale = tcx.type_scale;

        let step_down_2 = 1.0 / (type_scale * type_scale);
        let step_down_1 = 1.0 / type_scale;
        let base = 1.0;
        let step_up_1 = base * type_scale;
        let step_up_2 = step_up_1 * type_scale;
        let step_up_3 = step_up_2 * type_scale;
        let step_up_4 = step_up_3 * type_scale;
        let step_up_5 = step_up_4 * type_scale;
        let step_up_6 = step_up_5 * type_scale;

        div()
            .font_family(self.font_family.clone())
            .size_full()
            .child(
                div()
                    .bg(gpui::white())
                    .border_b_1()
                    .border_color(gpui::black())
                    .p_3()
                    .flex()
                    .child(Self::button(&self.font_family, cx, |this, cx| {
                        let new_family = FONT_FAMILIES
                            .iter()
                            .position(|f| *f == this.font_family.as_str())
                            .map(|idx| FONT_FAMILIES[(idx + 1) % FONT_FAMILIES.len()])
                            .unwrap_or(FONT_FAMILIES[0]);

                        this.font_family = SharedString::new(new_family);
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("text-example")
                    .overflow_y_scroll()
                    .overflow_x_hidden()
                    .bg(rgb(0xffffff))
                    .size_full()
                    .child(div().child(CharacterGrid::new().scale(base)))
                    .child(
                        div()
                            .child(Specimen::new(self.next_id()).scale(step_down_2))
                            .child(Specimen::new(self.next_id()).scale(step_down_2).invert())
                            .child(Specimen::new(self.next_id()).scale(step_down_1))
                            .child(Specimen::new(self.next_id()).scale(step_down_1).invert())
                            .child(Specimen::new(self.next_id()).scale(base))
                            .child(Specimen::new(self.next_id()).scale(base).invert())
                            .child(Specimen::new(self.next_id()).scale(step_up_1))
                            .child(Specimen::new(self.next_id()).scale(step_up_1).invert())
                            .child(Specimen::new(self.next_id()).scale(step_up_2))
                            .child(Specimen::new(self.next_id()).scale(step_up_2).invert())
                            .child(Specimen::new(self.next_id()).scale(step_up_3))
                            .child(Specimen::new(self.next_id()).scale(step_up_3).invert())
                            .child(Specimen::new(self.next_id()).scale(step_up_4))
                            .child(Specimen::new(self.next_id()).scale(step_up_4).invert())
                            .child(Specimen::new(self.next_id()).scale(step_up_5))
                            .child(Specimen::new(self.next_id()).scale(step_up_5).invert())
                            .child(Specimen::new(self.next_id()).scale(step_up_6))
                            .child(Specimen::new(self.next_id()).scale(step_up_6).invert()),
                    ),
            )
            .child(div().w(px(240.)).h_full().bg(colors.container))
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        cx.set_menus(vec![Menu {
            name: "GPUI Typography".into(),
            disabled: false,
            items: vec![],
        }]);

        let fonts = [include_bytes!(
            "../../../assets/fonts/lilex/Lilex-Regular.ttf"
        )]
        .iter()
        .map(|b| Cow::Borrowed(&b[..]))
        .collect();

        _ = cx.text_system().add_fonts(fonts);

        cx.init_colors();
        cx.set_global(GlobalTextContext(Arc::new(TextContext::default())));

        let window = cx
            .open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("GPUI Typography".into()),
                        ..Default::default()
                    }),
                    window_bounds: Some(WindowBounds::Windowed(bounds(
                        point(px(0.0), px(0.0)),
                        size(px(920.), px(720.)),
                    ))),
                    ..Default::default()
                },
                |_window, cx| {
                    cx.new(|_cx| TextExample {
                        next_id: 0,
                        font_family: ".ZedMono".into(),
                    })
                },
            )
            .unwrap();

        window
            .update(cx, |_view, _window, cx| {
                cx.activate(true);
            })
            .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/text_layout.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, FontStyle, FontWeight, StyledText, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, size,
};
use gpui_platform::application;

struct HelloWorld {}

impl Render for HelloWorld {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .bg(gpui::white())
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .size_full()
            .child(div().child("Text left"))
            .child(div().text_center().child("Text center"))
            .child(div().text_right().child("Text right"))
            .child(div().text_decoration_1().child("Text left (underline)"))
            .child(
                div()
                    .text_center()
                    .text_decoration_1()
                    .child("Text center (underline)"),
            )
            .child(
                div()
                    .text_right()
                    .text_decoration_1()
                    .child("Text right (underline)"),
            )
            .child(div().line_through().child("Text left (line_through)"))
            .child(
                div()
                    .text_center()
                    .line_through()
                    .child("Text center (line_through)"),
            )
            .child(
                div()
                    .text_right()
                    .line_through()
                    .child("Text right (line_through)"),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_between()
                    .child(
                        div()
                            .w(px(400.))
                            .border_1()
                            .border_color(gpui::blue())
                            .p_1()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_center()
                            .child("A long non-wrapping text align center"),
                    )
                    .child(
                        div()
                            .w_32()
                            .border_1()
                            .border_color(gpui::blue())
                            .p_1()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_right()
                            .child("100%"),
                    ),
            )
            .child(div().flex().gap_2().justify_between().child(
                StyledText::new("ABCD").with_highlights([
                    (0..1, FontWeight::EXTRA_BOLD.into()),
                    (2..3, FontStyle::Italic.into()),
                ]),
            ))
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| HelloWorld {}),
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/text_wrapper.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, TextOverflow, Window, WindowBounds, WindowOptions, div, prelude::*, px,
    size,
};
use gpui_platform::application;

struct HelloWorld {}

impl Render for HelloWorld {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let text = "The longest word 你好世界这段是中文，こんにちはこの段落は日本語です in any of the major \
            English language dictionaries is pneumonoultramicroscopicsilicovolcanoconiosis, a word that \
            refers to a lung disease contracted from the inhalation of very fine silica particles, \
            a url https://github.com/zed-industries/zed/pull/35724?query=foo&bar=2, \
            specifically from a volcano; medically, it is the same as silicosis.";
        div()
            .id("page")
            .size_full()
            .flex()
            .flex_col()
            .p_2()
            .gap_2()
            .bg(gpui::white())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_shrink_0()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .border_1()
                            .border_color(gpui::red())
                            .text_ellipsis()
                            .child("longer text in flex 1"),
                    )
                    .child(
                        div()
                            .flex()
                            .border_1()
                            .border_color(gpui::red())
                            .text_ellipsis()
                            .child("short flex"),
                    )
                    .child(
                        div()
                            .overflow_hidden()
                            .border_1()
                            .border_color(gpui::red())
                            .text_ellipsis()
                            .w_full()
                            .child("A short text in normal div"),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_xl()
                    .truncate()
                    .border_1()
                    .border_color(gpui::blue())
                    .child("ELLIPSIS: ".to_owned() + text),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_xl()
                    .overflow_hidden()
                    .text_ellipsis()
                    .line_clamp(2)
                    .border_1()
                    .border_color(gpui::blue())
                    .child("ELLIPSIS 2 lines: ".to_owned() + text),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_xl()
                    .overflow_hidden()
                    .text_overflow(TextOverflow::Truncate("".into()))
                    .border_1()
                    .border_color(gpui::green())
                    .child("TRUNCATE: ".to_owned() + text),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_xl()
                    .overflow_hidden()
                    .text_overflow(TextOverflow::Truncate("".into()))
                    .line_clamp(3)
                    .border_1()
                    .border_color(gpui::green())
                    .child("TRUNCATE 3 lines: ".to_owned() + text),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_xl()
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .border_1()
                    .border_color(gpui::black())
                    .child("NOWRAP: ".to_owned() + text),
            )
            .child(div().text_xl().w_full().child(text))
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| HelloWorld {}),
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/tree.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]
//! Renders a div with deep children hierarchy. This example is useful to exemplify that Zed can
//! handle deep hierarchies (even though it cannot just yet!).
use std::sync::LazyLock;

use gpui::{App, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px, size};
use gpui_platform::application;

struct Tree {}

static DEPTH: LazyLock<u64> = LazyLock::new(|| {
    std::env::var("GPUI_TREE_DEPTH")
        .ok()
        .and_then(|depth| depth.parse().ok())
        .unwrap_or_else(|| 50)
});

impl Render for Tree {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut depth = *DEPTH;
        static COLORS: [gpui::Hsla; 4] = [gpui::red(), gpui::blue(), gpui::green(), gpui::yellow()];
        let mut colors = COLORS.iter().cycle().copied();
        let mut next_div = || div().p_0p5().bg(colors.next().unwrap());
        let mut innermost_node = next_div();
        while depth > 0 {
            innermost_node = next_div().child(innermost_node);
            depth -= 1;
        }
        innermost_node
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(300.0), px(300.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Tree {}),
        )
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/uniform_list.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
    uniform_list,
};
use gpui_platform::application;

struct UniformListExample {}

impl Render for UniformListExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(rgb(0xffffff)).child(
            uniform_list(
                "entries",
                50,
                cx.processor(|_this, range, _window, _cx| {
                    let mut items = Vec::new();
                    for ix in range {
                        let item = ix + 1;

                        items.push(
                            div()
                                .id(ix)
                                .px_2()
                                .cursor_pointer()
                                .on_click(move |_event, _window, _cx| {
                                    println!("clicked Item {item:?}");
                                })
                                .child(format!("Item {item}")),
                        );
                    }
                    items
                }),
            )
            .h_full(),
        )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(300.0), px(300.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| UniformListExample {}),
        )
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/window.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, KeyBinding, PromptButton, PromptLevel, Window, WindowBounds, WindowKind,
    WindowOptions, actions, div, prelude::*, px, rgb, size,
};
use gpui_platform::application;

struct SubWindow {
    custom_titlebar: bool,
    is_dialog: bool,
}

fn button(text: &str, on_click: impl Fn(&mut Window, &mut App) + 'static) -> impl IntoElement {
    div()
        .id(text.to_string())
        .flex_none()
        .px_2()
        .bg(rgb(0xf7f7f7))
        .active(|this| this.opacity(0.85))
        .border_1()
        .border_color(rgb(0xe0e0e0))
        .rounded_sm()
        .cursor_pointer()
        .child(text.to_string())
        .on_click(move |_, window, cx| on_click(window, cx))
}

impl Render for SubWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_bounds =
            WindowBounds::Windowed(Bounds::centered(None, size(px(250.0), px(200.0)), cx));

        div()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .size_full()
            .gap_2()
            .when(self.custom_titlebar, |cx| {
                cx.child(
                    div()
                        .flex()
                        .h(px(32.))
                        .px_4()
                        .bg(gpui::blue())
                        .text_color(gpui::white())
                        .w_full()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size_full()
                                .child("Custom Titlebar"),
                        ),
                )
            })
            .child(
                div()
                    .p_8()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child("SubWindow")
                    .when(self.is_dialog, |div| {
                        div.child(button("Open Nested Dialog", move |_, cx| {
                            cx.open_window(
                                WindowOptions {
                                    window_bounds: Some(window_bounds),
                                    kind: WindowKind::Dialog,
                                    ..Default::default()
                                },
                                |_, cx| {
                                    cx.new(|_| SubWindow {
                                        custom_titlebar: false,
                                        is_dialog: true,
                                    })
                                },
                            )
                            .unwrap();
                        }))
                    })
                    .child(button("Close", |window, _| {
                        window.remove_window();
                    })),
            )
    }
}

struct WindowDemo {}

impl Render for WindowDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_bounds =
            WindowBounds::Windowed(Bounds::centered(None, size(px(300.0), px(300.0)), cx));

        div()
            .p_4()
            .flex()
            .flex_wrap()
            .bg(rgb(0xffffff))
            .size_full()
            .justify_center()
            .content_center()
            .gap_2()
            .child(button("Normal", move |_, cx| {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(window_bounds),
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|_| SubWindow {
                            custom_titlebar: false,
                            is_dialog: false,
                        })
                    },
                )
                .unwrap();
            }))
            .child(button("Popup", move |_, cx| {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(window_bounds),
                        kind: WindowKind::PopUp,
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|_| SubWindow {
                            custom_titlebar: false,
                            is_dialog: false,
                        })
                    },
                )
                .unwrap();
            }))
            .child(button("Floating", move |_, cx| {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(window_bounds),
                        kind: WindowKind::Floating,
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|_| SubWindow {
                            custom_titlebar: false,
                            is_dialog: false,
                        })
                    },
                )
                .unwrap();
            }))
            .child(button("Dialog", move |_, cx| {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(window_bounds),
                        kind: WindowKind::Dialog,
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|_| SubWindow {
                            custom_titlebar: false,
                            is_dialog: true,
                        })
                    },
                )
                .unwrap();
            }))
            .child(button("Custom Titlebar", move |_, cx| {
                cx.open_window(
                    WindowOptions {
                        titlebar: None,
                        window_bounds: Some(window_bounds),
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|_| SubWindow {
                            custom_titlebar: true,
                            is_dialog: false,
                        })
                    },
                )
                .unwrap();
            }))
            .child(button("Invisible", move |_, cx| {
                cx.open_window(
                    WindowOptions {
                        show: false,
                        window_bounds: Some(window_bounds),
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|_| SubWindow {
                            custom_titlebar: false,
                            is_dialog: false,
                        })
                    },
                )
                .unwrap();
            }))
            .child(button("Unmovable", move |_, cx| {
                cx.open_window(
                    WindowOptions {
                        is_movable: false,
                        titlebar: None,
                        window_bounds: Some(window_bounds),
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|_| SubWindow {
                            custom_titlebar: false,
                            is_dialog: false,
                        })
                    },
                )
                .unwrap();
            }))
            .child(button("Unresizable", move |_, cx| {
                cx.open_window(
                    WindowOptions {
                        is_resizable: false,
                        window_bounds: Some(window_bounds),
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|_| SubWindow {
                            custom_titlebar: false,
                            is_dialog: false,
                        })
                    },
                )
                .unwrap();
            }))
            .child(button("Unminimizable", move |_, cx| {
                cx.open_window(
                    WindowOptions {
                        is_minimizable: false,
                        window_bounds: Some(window_bounds),
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|_| SubWindow {
                            custom_titlebar: false,
                            is_dialog: false,
                        })
                    },
                )
                .unwrap();
            }))
            .child(button("Hide Application", |window, cx| {
                cx.hide();

                // Restore the application after 3 seconds
                window
                    .spawn(cx, async move |cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_secs(3))
                            .await;
                        cx.update(|_, cx| {
                            cx.activate(false);
                        })
                    })
                    .detach();
            }))
            .child(button("Resize", |window, _| {
                let content_size = window.bounds().size;
                window.resize(size(content_size.height, content_size.width));
            }))
            .child(button("Prompt", |window, cx| {
                let answer = window.prompt(
                    PromptLevel::Info,
                    "Are you sure?",
                    None,
                    &["Ok", "Cancel"],
                    cx,
                );

                cx.spawn(async move |_| {
                    if answer.await.unwrap() == 0 {
                        println!("You have clicked Ok");
                    } else {
                        println!("You have clicked Cancel");
                    }
                })
                .detach();
            }))
            .child(button("Prompt (non-English)", |window, cx| {
                let answer = window.prompt(
                    PromptLevel::Info,
                    "Are you sure?",
                    None,
                    &[PromptButton::ok("确定"), PromptButton::cancel("取消")],
                    cx,
                );

                cx.spawn(async move |_| {
                    if answer.await.unwrap() == 0 {
                        println!("You have clicked Ok");
                    } else {
                        println!("You have clicked Cancel");
                    }
                })
                .detach();
            }))
    }
}

actions!(window, [Quit]);

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    cx.observe_window_bounds(window, move |_, window, _| {
                        println!("Window bounds changed: {:?}", window.bounds());
                    })
                    .detach();

                    WindowDemo {}
                })
            },
        )
        .unwrap();

        cx.activate(true);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/window_positioning.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, DisplayId, Hsla, Pixels, SharedString, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, point, prelude::*,
    px, rgb,
};
use gpui_platform::application;

struct WindowContent {
    text: SharedString,
    bounds: Bounds<Pixels>,
    bg: Hsla,
}

impl Render for WindowContent {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let window_bounds = window.bounds();

        div()
            .flex()
            .flex_col()
            .bg(self.bg)
            .size_full()
            .items_center()
            .text_color(rgb(0xffffff))
            .child(self.text.clone())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .text_sm()
                    .items_center()
                    .size_full()
                    .child(format!(
                        "origin: {}, {} size: {}, {}",
                        self.bounds.origin.x,
                        self.bounds.origin.y,
                        self.bounds.size.width,
                        self.bounds.size.height
                    ))
                    .child(format!(
                        "cx.bounds() origin: {}, {} size {}, {}",
                        window_bounds.origin.x,
                        window_bounds.origin.y,
                        window_bounds.size.width,
                        window_bounds.size.height
                    )),
            )
    }
}

fn build_window_options(display_id: DisplayId, bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        // Set the bounds of the window in screen coordinates
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        // Specify the display_id to ensure the window is created on the correct screen
        display_id: Some(display_id),
        titlebar: None,
        window_background: WindowBackgroundAppearance::Transparent,
        focus: false,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: false,
        app_id: None,
        window_min_size: None,
        window_decorations: None,
        tabbing_identifier: None,
        ..Default::default()
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        // Create several new windows, positioned in the top right corner of each screen
        let size = Size {
            width: px(350.),
            height: px(75.),
        };
        let margin_offset = px(150.);

        for screen in cx.displays() {
            let bounds = Bounds {
                origin: point(margin_offset, margin_offset),
                size,
            };

            cx.open_window(build_window_options(screen.id(), bounds), |_, cx| {
                cx.new(|_| WindowContent {
                    text: format!("Top Left {:?}", screen.id()).into(),
                    bg: gpui::red(),
                    bounds,
                })
            })
            .unwrap();

            let bounds = Bounds {
                origin: screen.bounds().top_right()
                    - point(size.width + margin_offset, -margin_offset),
                size,
            };

            cx.open_window(build_window_options(screen.id(), bounds), |_, cx| {
                cx.new(|_| WindowContent {
                    text: format!("Top Right {:?}", screen.id()).into(),
                    bg: gpui::red(),
                    bounds,
                })
            })
            .unwrap();

            let bounds = Bounds {
                origin: screen.bounds().bottom_left()
                    - point(-margin_offset, size.height + margin_offset),
                size,
            };

            cx.open_window(build_window_options(screen.id(), bounds), |_, cx| {
                cx.new(|_| WindowContent {
                    text: format!("Bottom Left {:?}", screen.id()).into(),
                    bg: gpui::blue(),
                    bounds,
                })
            })
            .unwrap();

            let bounds = Bounds {
                origin: screen.bounds().bottom_right()
                    - point(size.width + margin_offset, size.height + margin_offset),
                size,
            };

            cx.open_window(build_window_options(screen.id(), bounds), |_, cx| {
                cx.new(|_| WindowContent {
                    text: format!("Bottom Right {:?}", screen.id()).into(),
                    bg: gpui::blue(),
                    bounds,
                })
            })
            .unwrap();

            let bounds = Bounds {
                origin: point(screen.bounds().center().x - size.center().x, margin_offset),
                size,
            };

            cx.open_window(build_window_options(screen.id(), bounds), |_, cx| {
                cx.new(|_| WindowContent {
                    text: format!("Top Center {:?}", screen.id()).into(),
                    bg: gpui::black(),
                    bounds,
                })
            })
            .unwrap();

            let bounds = Bounds {
                origin: point(margin_offset, screen.bounds().center().y - size.center().y),
                size,
            };

            cx.open_window(build_window_options(screen.id(), bounds), |_, cx| {
                cx.new(|_| WindowContent {
                    text: format!("Left Center {:?}", screen.id()).into(),
                    bg: gpui::black(),
                    bounds,
                })
            })
            .unwrap();

            let bounds = Bounds {
                origin: point(
                    screen.bounds().center().x - size.center().x,
                    screen.bounds().center().y - size.center().y,
                ),
                size,
            };

            cx.open_window(build_window_options(screen.id(), bounds), |_, cx| {
                cx.new(|_| WindowContent {
                    text: format!("Center {:?}", screen.id()).into(),
                    bg: gpui::black(),
                    bounds,
                })
            })
            .unwrap();

            let bounds = Bounds {
                origin: point(
                    screen.bounds().size.width - size.width - margin_offset,
                    screen.bounds().center().y - size.center().y,
                ),
                size,
            };

            cx.open_window(build_window_options(screen.id(), bounds), |_, cx| {
                cx.new(|_| WindowContent {
                    text: format!("Right Center {:?}", screen.id()).into(),
                    bg: gpui::black(),
                    bounds,
                })
            })
            .unwrap();

            let bounds = Bounds {
                origin: point(
                    screen.bounds().center().x - size.center().x,
                    screen.bounds().size.height - size.height - margin_offset,
                ),
                size,
            };

            cx.open_window(build_window_options(screen.id(), bounds), |_, cx| {
                cx.new(|_| WindowContent {
                    text: format!("Bottom Center {:?}", screen.id()).into(),
                    bg: gpui::black(),
                    bounds,
                })
            })
            .unwrap();
        }
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
---
## `example_file:crates/gpui/examples/window_shadow.rs`

```rust
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, CursorStyle, Decorations, HitboxBehavior, Hsla, MouseButton, Pixels,
    Point, ResizeEdge, Size, Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations,
    WindowOptions, black, canvas, div, green, point, prelude::*, px, rgb, size, transparent_black,
    white,
};
use gpui_platform::application;

struct WindowShadow {}

// Things to do:
// 1. We need a way of calculating which edge or corner the mouse is on,
//    and then dispatch on that
// 2. We need to improve the shadow rendering significantly
// 3. We need to implement the techniques in here in Zed

impl Render for WindowShadow {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let decorations = window.window_decorations();
        let rounding = px(10.0);
        let shadow_size = px(10.0);
        let border_size = px(1.0);
        let grey = rgb(0x808080);
        window.set_client_inset(shadow_size);

        div()
            .id("window-backdrop")
            .bg(transparent_black())
            .map(|div| match decorations {
                Decorations::Server => div,
                Decorations::Client { tiling, .. } => div
                    .bg(gpui::transparent_black())
                    .child(
                        canvas(
                            |_bounds, window, _cx| {
                                window.insert_hitbox(
                                    Bounds::new(
                                        point(px(0.0), px(0.0)),
                                        window.window_bounds().get_bounds().size,
                                    ),
                                    HitboxBehavior::Normal,
                                )
                            },
                            move |_bounds, hitbox, window, _cx| {
                                let mouse = window.mouse_position();
                                let size = window.window_bounds().get_bounds().size;
                                let Some(edge) = resize_edge(mouse, shadow_size, size) else {
                                    return;
                                };
                                window.set_cursor_style(
                                    match edge {
                                        ResizeEdge::Top | ResizeEdge::Bottom => {
                                            CursorStyle::ResizeUpDown
                                        }
                                        ResizeEdge::Left | ResizeEdge::Right => {
                                            CursorStyle::ResizeLeftRight
                                        }
                                        ResizeEdge::TopLeft | ResizeEdge::BottomRight => {
                                            CursorStyle::ResizeUpLeftDownRight
                                        }
                                        ResizeEdge::TopRight | ResizeEdge::BottomLeft => {
                                            CursorStyle::ResizeUpRightDownLeft
                                        }
                                    },
                                    &hitbox,
                                );
                            },
                        )
                        .size_full()
                        .absolute(),
                    )
                    .when(!(tiling.top || tiling.right), |div| {
                        div.rounded_tr(rounding)
                    })
                    .when(!(tiling.top || tiling.left), |div| div.rounded_tl(rounding))
                    .when(!tiling.top, |div| div.pt(shadow_size))
                    .when(!tiling.bottom, |div| div.pb(shadow_size))
                    .when(!tiling.left, |div| div.pl(shadow_size))
                    .when(!tiling.right, |div| div.pr(shadow_size))
                    .on_mouse_move(|_e, window, _cx| window.refresh())
                    .on_mouse_down(MouseButton::Left, move |e, window, _cx| {
                        let size = window.window_bounds().get_bounds().size;
                        let pos = e.position;

                        match resize_edge(pos, shadow_size, size) {
                            Some(edge) => window.start_window_resize(edge),
                            None => window.start_window_move(),
                        };
                    }),
            })
            .size_full()
            .child(
                div()
                    .cursor(CursorStyle::Arrow)
                    .map(|div| match decorations {
                        Decorations::Server => div,
                        Decorations::Client { tiling } => div
                            .border_color(grey)
                            .when(!(tiling.top || tiling.right), |div| {
                                div.rounded_tr(rounding)
                            })
                            .when(!(tiling.top || tiling.left), |div| div.rounded_tl(rounding))
                            .when(!tiling.top, |div| div.border_t(border_size))
                            .when(!tiling.bottom, |div| div.border_b(border_size))
                            .when(!tiling.left, |div| div.border_l(border_size))
                            .when(!tiling.right, |div| div.border_r(border_size))
                            .when(!tiling.is_tiled(), |div| {
                                div.shadow(vec![gpui::BoxShadow {
                                    color: Hsla {
                                        h: 0.,
                                        s: 0.,
                                        l: 0.,
                                        a: 0.4,
                                    },
                                    blur_radius: shadow_size / 2.,
                                    spread_radius: px(0.),
                                    offset: point(px(0.0), px(0.0)),
                                }])
                            }),
                    })
                    .on_mouse_move(|_e, _, cx| {
                        cx.stop_propagation();
                    })
                    .bg(gpui::rgb(0xCCCCFF))
                    .size_full()
                    .flex()
                    .flex_col()
                    .justify_around()
                    .child(
                        div().w_full().flex().flex_row().justify_around().child(
                            div()
                                .flex()
                                .bg(white())
                                .size(px(300.0))
                                .justify_center()
                                .items_center()
                                .shadow_lg()
                                .border_1()
                                .border_color(rgb(0x0000ff))
                                .text_xl()
                                .text_color(rgb(0xffffff))
                                .child(
                                    div()
                                        .id("hello")
                                        .w(px(200.0))
                                        .h(px(100.0))
                                        .bg(green())
                                        .shadow(vec![gpui::BoxShadow {
                                            color: Hsla {
                                                h: 0.,
                                                s: 0.,
                                                l: 0.,
                                                a: 1.0,
                                            },
                                            blur_radius: px(20.0),
                                            spread_radius: px(0.0),
                                            offset: point(px(0.0), px(0.0)),
                                        }])
                                        .map(|div| match decorations {
                                            Decorations::Server => div,
                                            Decorations::Client { .. } => div
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    |_e, window, _| {
                                                        window.start_window_move();
                                                    },
                                                )
                                                .on_click(|e, window, _| {
                                                    if e.is_right_click() {
                                                        window.show_window_menu(e.position());
                                                    }
                                                })
                                                .text_color(black())
                                                .child("this is the custom titlebar"),
                                        }),
                                ),
                        ),
                    ),
            )
    }
}

fn resize_edge(pos: Point<Pixels>, shadow_size: Pixels, size: Size<Pixels>) -> Option<ResizeEdge> {
    let edge = if pos.y < shadow_size && pos.x < shadow_size {
        ResizeEdge::TopLeft
    } else if pos.y < shadow_size && pos.x > size.width - shadow_size {
        ResizeEdge::TopRight
    } else if pos.y < shadow_size {
        ResizeEdge::Top
    } else if pos.y > size.height - shadow_size && pos.x < shadow_size {
        ResizeEdge::BottomLeft
    } else if pos.y > size.height - shadow_size && pos.x > size.width - shadow_size {
        ResizeEdge::BottomRight
    } else if pos.y > size.height - shadow_size {
        ResizeEdge::Bottom
    } else if pos.x < shadow_size {
        ResizeEdge::Left
    } else if pos.x > size.width - shadow_size {
        ResizeEdge::Right
    } else {
        return None;
    };
    Some(edge)
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(600.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background: WindowBackgroundAppearance::Opaque,
                window_decorations: Some(WindowDecorations::Client),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    cx.observe_window_appearance(window, |_, window, _| {
                        window.refresh();
                    })
                    .detach();
                    WindowShadow {}
                })
            },
        )
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

```
