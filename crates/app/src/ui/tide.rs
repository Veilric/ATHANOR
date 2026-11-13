use super::{AppState, event, flush, tokens as t};
use athanor_core::{feature::Presentation, panel::PanelEvent};
use rune::{Color, Cx, El, Key, KeyEvent, ScrollSpec, Tx, col, el, row, spacer, text, text_input};

pub fn button(
    label: &str,
    key: &str,
    caption: &str,
    action: impl Fn(&mut AppState, &mut Tx) + 'static,
) -> El {
    el().key(key)
        .role("button")
        .label(caption)
        .focusable(true)
        .padding(6.0)
        .radius(6.0)
        .child(text(label).font(t::small()).color(t::muted()))
        .on_press_tx(action)
}
pub fn on_key(s: &mut AppState, key: &KeyEvent, tx: &mut Tx) -> bool {
    if s.input.is_composing() {
        return false;
    }
    if key.is_command(',') {
        s.show_settings();
        flush(s, tx);
        return true;
    }
    match key.key {
        Key::Escape => {
            event(s, PanelEvent::Escape, tx);
            true
        }
        Key::Down if !s.items.is_empty() => {
            s.selected = (s.selected + 1) % s.items.len();
            tx.scroll_into_view(format!("result-{}", s.selected));
            true
        }
        Key::Up if !s.items.is_empty() => {
            s.selected = (s.selected + s.items.len() - 1) % s.items.len();
            tx.scroll_into_view(format!("result-{}", s.selected));
            true
        }
        _ => false,
    }
}
pub fn tide_view(s: &AppState, _: &mut Cx<AppState>) -> El {
    if !s.panel.visible {
        return el().w_full().h_full();
    }
    if s.settings_open {
        return super::settings::settings_view(s);
    }
    let input = text_input(&s.input, t::body())
        .key("query")
        .label("Search, calculate, or convert")
        .single_line()
        .grow(1.0)
        .color(t::ink())
        .placeholder("Search, calculate, convert…", t::muted())
        .on_edit(|s: &mut AppState| &mut s.input)
        .on_input_change(|s: &mut AppState, tx| {
            event(s, PanelEvent::QueryChanged(s.input.text.clone()), tx)
        })
        .on_input_key(on_key)
        .on_submit(|s: &mut AppState, tx| {
            s.submit();
            flush(s, tx)
        });
    let mark = el()
        .key("drag-handle")
        .size(16.0, 22.0)
        .window_drag()
        .label("Drag panel")
        .child(
            text("›")
                .font(rune::TextStyle::new(21.0, 500.0))
                .color(t::accent()),
        );
    let pin = button(
        if s.panel.pinned { "⌖" } else { "⊙" },
        "pin",
        if s.panel.pinned {
            "Unpin panel"
        } else {
            "Keep on top"
        },
        |s, tx| {
            event(s, PanelEvent::Pin(!s.panel.pinned), tx);
        },
    )
    .size(25.0, 27.0)
    .fill(if s.panel.pinned {
        t::accent().alpha(0.09)
    } else {
        Color::TRANSPARENT
    });
    let mut composer = row()
        .key("composer")
        .w_full()
        .h(53.0)
        .px(14.0)
        .gap(8.0)
        .center_y()
        .child(mark)
        .child(input)
        .child(pin);
    if !s.panel.compact {
        composer = composer.child(
            button("↵", "submit", "Run selected action", |s, tx| {
                s.submit();
                flush(s, tx)
            })
            .size(25.0, 27.0)
            .fill(t::accent().alpha(0.08)),
        );
    }
    let radius = if s.panel.compact { 13.0 } else { 16.0 };
    let mut panel = col()
        .key("tide-panel")
        .w_full()
        .h_full()
        .radius(radius)
        .clip_rounded(radius)
        .fill(t::panel(s.panel_alpha()))
        .border(1.0, t::muted().alpha(0.24))
        .child(composer);
    if s.panel.compact {
        return panel;
    }
    panel = panel.child(el().w_full().h(1.0).fill(t::muted().alpha(0.14)));
    let mut results = col()
        .key("results")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .clip_rounded(0.0);
    if let Some(error) = &s.panel.error {
        results = results.child(
            col()
                .padding(18.0)
                .gap(8.0)
                .child(
                    text("Could not complete the action")
                        .font(t::label())
                        .color(t::ink()),
                )
                .child(text(error.clone()).font(t::body()).color(t::muted())),
        );
    } else if s.panel.query_staged {
        results = results.child(
            col()
                .key("staged-example")
                .padding(20.0)
                .gap(9.0)
                .child(text("Edit this example").font(t::label()).color(t::ink()))
                .child(
                    text("Press Enter when you are ready to evaluate it.")
                        .font(t::small())
                        .color(t::muted())
                        .wrap()
                        .w_full(),
                ),
        );
    } else if s.query_pending {
        results = results.child(
            col()
                .key("pending")
                .padding(20.0)
                .child(text("Working…").font(t::small()).color(t::muted())),
        );
    } else if s.items.len() == 1 && s.items[0].presentation == Presentation::Answer {
        let item = &s.items[0];
        results = results.child(
            col()
                .w_full()
                .padding(22.0)
                .gap(10.0)
                .child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(el().size(4.0, 4.0).radius(2.0).fill(t::accent()))
                        .child(text("Result").font(t::small()).color(t::muted())),
                )
                .child(
                    text(item.title.clone())
                        .key("answer")
                        .font(t::large())
                        .color(t::ink())
                        .w_full(),
                )
                .child(
                    text(item.detail.clone())
                        .font(t::small())
                        .color(t::muted())
                        .w_full(),
                ),
        );
    } else if s.items.is_empty() {
        let (title, detail) = if s.panel.query.trim().is_empty() {
            (
                "A little help, right here.",
                "Type help to explore commands and examples.",
            )
        } else {
            (
                "No matching result",
                "Try another name, expression, or conversion.",
            )
        };
        results = results.child(
            col()
                .padding(20.0)
                .gap(9.0)
                .child(text(title).font(t::label()).color(t::ink()))
                .child(text(detail).font(t::small()).color(t::muted()).w_full()),
        );
    } else {
        let mut list = col().w_full().padding(7.0).gap(3.0);
        for (i, item) in s.items.iter().enumerate() {
            if item.presentation == Presentation::Reference {
                let actionable = !item.actions.is_empty();
                let mut card = col()
                    .key(format!("result-{i}"))
                    .role(if actionable { "button" } else { "group" })
                    .label(format!("{} — {}", item.title, item.detail))
                    .focusable(actionable)
                    .w_full()
                    .min_h(49.0)
                    .padding(12.0)
                    .gap(5.0)
                    .radius(8.0)
                    .fill(if i == s.selected {
                        t::accent().alpha(0.12)
                    } else {
                        Color::TRANSPARENT
                    })
                    .child(
                        text(item.title.clone())
                            .font(t::label())
                            .color(t::ink())
                            .wrap()
                            .w_full(),
                    )
                    .child(
                        text(item.detail.clone())
                            .font(t::small())
                            .color(t::muted())
                            .wrap()
                            .w_full(),
                    );
                if actionable {
                    card = card.on_press_tx(move |s: &mut AppState, tx| {
                        s.selected = i;
                        s.act(0);
                        flush(s, tx)
                    });
                }
                list = list.child(card);
                continue;
            }
            let icon = match item.presentation {
                Presentation::Application => "◇",
                Presentation::Folder => "▱",
                Presentation::File => "▤",
                Presentation::Answer => "=",
                Presentation::Explanation => "?",
                Presentation::Reference => "?",
            };
            let selected = i == s.selected;
            list = list.child(
                row()
                    .key(format!("result-{i}"))
                    .role("button")
                    .label(format!("{} — {}", item.title, item.detail))
                    .focusable(true)
                    .w_full()
                    .h(49.0)
                    .px(10.0)
                    .gap(10.0)
                    .center_y()
                    .radius(8.0)
                    .fill(if selected {
                        t::accent().alpha(0.12)
                    } else {
                        Color::TRANSPARENT
                    })
                    .child(
                        el().size(25.0, 29.0)
                            .radius(6.0)
                            .fill(t::accent().alpha(0.07))
                            .center()
                            .child(text(icon).font(t::label()).color(t::accent())),
                    )
                    .child(
                        col()
                            .grow(1.0)
                            .clip_rounded(0.0)
                            .gap(2.0)
                            .child(text(item.title.clone()).font(t::label()).color(t::ink()))
                            .child(text(item.detail.clone()).font(t::small()).color(t::muted())),
                    )
                    .on_press_tx(move |s: &mut AppState, tx| {
                        s.selected = i;
                        s.act(0);
                        flush(s, tx)
                    }),
            );
        }
        results = results.child(
            list.scroll(ScrollSpec {
                stick_bottom: false,
            })
            .grow(1.0)
            .min_h(0.0)
            .w_full(),
        );
    }
    let mut footer = row()
        .key("footer")
        .w_full()
        .h(33.0)
        .px(10.0)
        .gap(5.0)
        .center_y()
        .fill(Color::BLACK.alpha(0.04));
    if let Some(item) = s.items.get(s.selected) {
        for (index, action) in item.actions.iter().enumerate().take(3) {
            footer = footer.child(button(
                action.label(),
                &format!("action-{index}"),
                action.label(),
                move |s, tx| {
                    s.act(index);
                    flush(s, tx)
                },
            ));
        }
    } else {
        footer = footer.child(
            text(if s.panel.pinned {
                "Pinned"
            } else {
                "Esc to close"
            })
            .font(t::small())
            .color(t::muted()),
        );
    }
    footer = footer
        .child(spacer())
        .child(button("⚙", "settings", "Open settings", |s, tx| {
            s.show_settings();
            flush(s, tx)
        }));
    panel.child(results).child(footer)
}
