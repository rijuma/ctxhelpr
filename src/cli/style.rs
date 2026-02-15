use owo_colors::{OwoColorize, Stream::Stdout};

pub fn heading(text: &str) -> String {
    text.if_supports_color(Stdout, |t| t.bold()).to_string()
}

pub fn success(text: &str) -> String {
    text.if_supports_color(Stdout, |t| t.green()).to_string()
}

pub fn info(text: &str) -> String {
    text.if_supports_color(Stdout, |t| t.dimmed()).to_string()
}

pub fn warn(text: &str) -> String {
    text.if_supports_color(Stdout, |t| t.yellow()).to_string()
}

pub fn error(text: &str) -> String {
    text.if_supports_color(Stdout, |t| t.red()).to_string()
}

pub fn done() -> String {
    success("done")
}
