#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamSpec {
    pub name: &'static str,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub unit: &'static str,
}

/// Name to value, in the order of the space that made it.
#[derive(Clone, Debug, PartialEq)]
pub struct Params(pub Vec<(String, f64)>);

impl Params {
    pub fn get(&self, name: &str) -> Option<f64> {
        self.0.iter().find(|(n, _)| n == name).map(|(_, v)| *v)
    }
    pub fn set(&mut self, name: &str, value: f64) -> bool {
        match self.0.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => {
                slot.1 = value;
                true
            }
            None => false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ParamSpace(pub &'static [ParamSpec]);

impl ParamSpace {
    pub fn specs(&self) -> &'static [ParamSpec] {
        self.0
    }

    pub fn defaults(&self) -> Params {
        Params(self.0.iter().map(|s| (s.name.to_string(), s.default)).collect())
    }

    /// Same names in the same order, finite, inside bounds.
    pub fn validate(&self, p: &Params) -> Result<(), String> {
        if p.0.len() != self.0.len() {
            return Err(format!("expected {} params, got {}", self.0.len(), p.0.len()));
        }
        for (s, (n, v)) in self.0.iter().zip(&p.0) {
            if s.name != n {
                return Err(format!("param `{n}` where `{}` was expected", s.name));
            }
            if !v.is_finite() || *v < s.min || *v > s.max {
                return Err(format!("param `{n}` = {v} outside [{}, {}]", s.min, s.max));
            }
        }
        Ok(())
    }

    /// Each value scaled to [0, 1] across its bounds, for optimizers.
    pub fn to_unit(&self, p: &Params) -> Vec<f64> {
        self.0
            .iter()
            .map(|s| {
                let v = p.get(s.name).unwrap_or(s.default);
                if s.max > s.min { ((v - s.min) / (s.max - s.min)).clamp(0.0, 1.0) } else { 0.0 }
            })
            .collect()
    }

    /// Inverse of `to_unit`; inputs are clipped to [0, 1]. Missing entries take the default.
    pub fn from_unit(&self, u: &[f64]) -> Params {
        Params(
            self.0
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let v = match u.get(i) {
                        Some(x) => s.min + x.clamp(0.0, 1.0) * (s.max - s.min),
                        None => s.default,
                    };
                    (s.name.to_string(), v)
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SPACE: ParamSpace = ParamSpace(&[
        ParamSpec { name: "a", min: 1.0, max: 3.0, default: 2.0, unit: "x" },
        ParamSpec { name: "b", min: 0.0, max: 40.0, default: 10.0, unit: "ticks" },
    ]);

    #[test]
    fn unit_round_trip_and_clip() {
        let d = SPACE.defaults();
        assert_eq!(SPACE.to_unit(&d), vec![0.5, 0.25]);
        let back = SPACE.from_unit(&SPACE.to_unit(&d));
        assert_eq!(back, d);
        let clipped = SPACE.from_unit(&[-3.0, 9.0]);
        assert_eq!(clipped.get("a"), Some(1.0));
        assert_eq!(clipped.get("b"), Some(40.0));
        assert!(SPACE.validate(&clipped).is_ok());
    }

    #[test]
    fn validate_rejects() {
        let mut p = SPACE.defaults();
        p.set("a", 5.0);
        assert!(SPACE.validate(&p).is_err());
        p.set("a", f64::NAN);
        assert!(SPACE.validate(&p).is_err());
        let short = Params(vec![("a".into(), 2.0)]);
        assert!(SPACE.validate(&short).is_err());
    }
}
