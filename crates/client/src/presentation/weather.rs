//! Weather visual descriptors; effects must not mutate simulation state.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeatherVisual { pub rain: f32, pub rainfall: f32 }
