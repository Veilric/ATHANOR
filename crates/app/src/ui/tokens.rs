use rune::{Color, TextStyle};
pub fn color(value: &str) -> Color {
    rune::color::hex(value).expect("Static UI color is valid")
}
pub fn ink() -> Color {
    color("#f0f5f7")
}
pub fn muted() -> Color {
    color("#edf4f7")
}
pub fn accent() -> Color {
    color("#a8d0e5")
}
pub fn panel(alpha: f32) -> Color {
    color("#15222b").alpha(alpha)
}
pub fn body() -> TextStyle {
    TextStyle::new(13.0, 400.0).lh(19.0)
}
pub fn small() -> TextStyle {
    TextStyle::new(10.0, 450.0).lh(15.0)
}
pub fn label() -> TextStyle {
    TextStyle::new(12.0, 550.0).lh(18.0)
}
pub fn large() -> TextStyle {
    TextStyle::new(32.0, 450.0).lh(41.0).features("'tnum' 1")
}
