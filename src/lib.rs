#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::CircuitApp;
//mod camera;
mod circuit_widget;
mod components;

fn to_metric_prefix(value: f32, unit: char) -> String {
    // WARNING: Chatgpt did this lol
    let prefixes = [
        (-24, "y"),
        (-21, "z"),
        (-18, "a"),
        (-15, "f"),
        (-12, "p"),
        (-9, "n"),
        (-6, "μ"),
        (-3, "m"),
        (0, ""),
        (3, "k"),
        (6, "M"),
        (9, "G"),
        (12, "T"),
        (15, "P"),
        (18, "E"),
        (21, "Z"),
        (24, "Y"),
    ];

    if value == 0.0 {
        return "0".to_string();
    }

    let exponent = (value.abs().log10() / 3.0).floor() as i32 * 3;
    let prefix = prefixes.iter().find(|&&(e, _)| e == exponent);

    if let Some((e, symbol)) = prefix {
        format!("{} {}{unit}", value / 10_f32.powi(*e), symbol)
    } else {
        format!("{} {unit}", value) // Fallback in case exponent is out of range
    }
}
