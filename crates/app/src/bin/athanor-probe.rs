use rune::tok::type_;
use rune::{
    App, AppHandle, AppSpec, Color, Cx, El, Key, KeyEvent, TextEdit, Tx, WindowOptions, WindowSpec,
    col, row, text, text_input,
};

struct Probe {
    query: TextEdit,
    handle: AppHandle<Probe>,
}
fn view(s: &Probe, _: &mut Cx<Probe>) -> El {
    let input = text_input(&s.query, type_::BODY)
        .key("probe-input")
        .single_line()
        .grow(1.0)
        .color(Color::WHITE)
        .placeholder("Type to check keyboard input", Color::WHITE.alpha(0.75))
        .on_edit(|s: &mut Probe| &mut s.query)
        .on_input_key(|_: &mut Probe, event: &KeyEvent, tx: &mut Tx| {
            if event.key == Key::Escape {
                tx.close_window();
                true
            } else {
                false
            }
        });
    col()
        .w_full()
        .h_full()
        .radius(13.0)
        .clip_rounded(13.0)
        .fill(Color {
            r: 0.08,
            g: 0.13,
            b: 0.17,
            a: 0.65,
        })
        .border(1.0, Color::WHITE.alpha(0.2))
        .child(row().h(53.0).px(15.0).child(input))
        .child(
            text(s.query.text.clone())
                .font(type_::BODY)
                .color(Color::WHITE),
        )
}
fn main() {
    rune::run_windows(|_| {
        let app = App::new(AppSpec::new(
            "Athanor capability probe",
            (330.0, 100.0),
            || unreachable!(),
            view,
        ))
        .windowless()
        .keep_running();
        let handle = app.handle();
        let shortcut = if cfg!(target_os = "macos") {
            "alt+space"
        } else {
            "ctrl+alt+space"
        };
        let app = app
            .init(move || {
                let mut query = TextEdit::new("");
                query.focused = true;
                Probe { query, handle }
            })
            .global_shortcut(shortcut, |s, tx| {
                s.query.focused = true;
                if let Some(token) = tx.activation_token() {
                    s.handle.activate(token);
                } else {
                    s.handle.open();
                }
            })
            .on_focus(|_, focused, tx| {
                if !focused {
                    tx.close_window();
                }
            });
        vec![WindowSpec::from_app("probe", app).options(WindowOptions {
            visible: false,
            transparent: true,
            borderless: true,
            always_on_top: true,
            min_size: Some((0.0, 0.0)),
            resizable: false,
            ..Default::default()
        })]
    });
}
