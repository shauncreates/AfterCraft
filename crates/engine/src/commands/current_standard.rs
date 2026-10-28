//! Standard easing families, evaluated directly rather than approximated by a single cubic.
use super::{CommandSpec, bad, has_keys};
use crate::{Result, Session, cmd};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StandardFamily {
    Ease,
    Linear,
    #[default]
    Quad,
    Cubic,
    Quart,
    Quint,
    Sine,
    Expo,
    Circ,
    Back,
}

impl StandardFamily {
    pub const ALL: [Self; 10] = [Self::Ease, Self::Linear, Self::Quad, Self::Cubic, Self::Quart, Self::Quint, Self::Sine, Self::Expo, Self::Circ, Self::Back];
    pub fn label(self) -> &'static str {
        match self {
            Self::Ease => "Ease",
            Self::Linear => "Linear",
            Self::Quad => "Quad",
            Self::Cubic => "Cubic",
            Self::Quart => "Quart",
            Self::Quint => "Quint",
            Self::Sine => "Sine",
            Self::Expo => "Expo",
            Self::Circ => "Circ",
            Self::Back => "Back",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    In,
    Out,
    #[default]
    InOut,
}
impl Direction {
    pub const ALL: [Self; 3] = [Self::In, Self::Out, Self::InOut];
    pub fn label(self) -> &'static str {
        match self {
            Self::In => "In",
            Self::Out => "Out",
            Self::InOut => "In / Out",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Standard {
    pub family: StandardFamily,
    pub direction: Direction,
}

pub(super) fn bezier_sample(curve: [f64; 4], time: f64) -> f64 {
    let cubic = |a: f64, b: f64, u: f64| 3.0 * (1.0 - u).powi(2) * u * a + 3.0 * (1.0 - u) * u * u * b + u.powi(3);
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..48 {
        let middle = (low + high) * 0.5;
        if cubic(curve[0], curve[2], middle) < time {
            low = middle;
        } else {
            high = middle;
        }
    }
    cubic(curve[1], curve[3], (low + high) * 0.5)
}

impl Standard {
    pub fn sample(self, time: f64) -> f64 {
        if !time.is_finite() || time <= 0.0 {
            return 0.0;
        }
        if time >= 1.0 {
            return 1.0;
        }
        if self.family == StandardFamily::Ease {
            let curve = match self.direction {
                Direction::In => [1.0 / 3.0, 0.0, 2.0 / 3.0, 2.0 / 3.0],
                Direction::Out => [1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 1.0],
                Direction::InOut => [1.0 / 3.0, 0.0, 2.0 / 3.0, 1.0],
            };
            return bezier_sample(curve, time);
        }
        let base = |t: f64| match self.family {
            StandardFamily::Ease | StandardFamily::Linear => t,
            StandardFamily::Quad => t * t,
            StandardFamily::Cubic => t.powi(3),
            StandardFamily::Quart => t.powi(4),
            StandardFamily::Quint => t.powi(5),
            StandardFamily::Sine => 1.0 - (std::f64::consts::FRAC_PI_2 * t).cos(),
            StandardFamily::Expo => {
                if t == 0.0 {
                    0.0
                } else {
                    (10.0 * t - 10.0).exp2()
                }
            }
            StandardFamily::Circ => 1.0 - (1.0 - t * t).max(0.0).sqrt(),
            StandardFamily::Back => t * t * ((1.70158 + 1.0) * t - 1.70158),
        };
        match self.direction {
            Direction::In => base(time),
            Direction::Out => 1.0 - base(1.0 - time),
            Direction::InOut if time < 0.5 => base(time * 2.0) * 0.5,
            Direction::InOut => 1.0 - base((1.0 - time) * 2.0) * 0.5,
        }
    }
}

fn apply(s: &mut Session, p: &Value) -> Result<Value> {
    let settings: Standard = serde_json::from_value(p.clone()).map_err(|e| bad("keys.currentStandard", format!("Invalid standard easing settings: {e}")))?;
    let mut result = super::current_physics::bake(s, p, |time| settings.sample(time), "Current: Apply Standard Easing")?;
    result["standard"] = json!(settings);
    Ok(result)
}

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![cmd!("keys.currentStandard", "Current: Apply Standard Easing", [], None, "{family,direction?,density?,token?}", has_keys, apply)]
}
