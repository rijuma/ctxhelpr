/// A simple point in 2D space
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Calculates distance between two points
pub fn distance(a: &Point, b: &Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    sqrt(dx * dx + dy * dy)
}

fn helper() -> bool {
    validate()
}

/// Represents different shapes
pub enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Triangle,
}

/// Trait for things that have area
pub trait HasArea {
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;
}

impl HasArea for Shape {
    fn area(&self) -> f64 {
        compute_area()
    }

    fn perimeter(&self) -> f64 {
        0.0
    }
}

impl Shape {
    pub fn describe(&self) -> String {
        format_description(self)
    }
}

/// Configuration settings
pub const MAX_SIZE: usize = 1024;

pub static GLOBAL_NAME: &str = "ctxhelpr";

/// An alias for results
pub type Result<T> = std::result::Result<T, Error>;

pub mod utils {
    pub fn clamp(val: f64, min: f64, max: f64) -> f64 {
        if val < min { min } else if val > max { max } else { val }
    }
}
