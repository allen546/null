use crate::ClampReport;

/// One tick of a duel: what the controller saw, and the action after clamps.
#[derive(Clone, Debug, PartialEq)]
pub struct TraceRecord {
    pub tick: u64,
    /// Order is `OBS_FIELDS`.
    pub obs: Vec<f32>,
    /// Order is `ACTION_FIELDS`; this is the clamped action that was sent.
    pub action: Vec<f32>,
    pub clamped: ClampReport,
}

fn nums(v: &[f32]) -> String {
    let parts: Vec<String> =
        v.iter().map(|x| if x.is_finite() { x.to_string() } else { "null".to_string() }).collect();
    format!("[{}]", parts.join(","))
}

impl TraceRecord {
    pub fn to_json_line(&self) -> String {
        let c = &self.clamped;
        format!(
            "{{\"tick\":{},\"obs\":{},\"action\":{},\"clamped\":{{\"attacks_dropped_rate\":{},\"attacks_dropped_reach\":{},\"looks_limited\":{},\"attacks_dropped_cargo\":{}}}}}",
            self.tick,
            nums(&self.obs),
            nums(&self.action),
            c.attacks_dropped_rate,
            c.attacks_dropped_reach,
            c.looks_limited,
            c.attacks_dropped_cargo
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_line_shape() {
        let r = TraceRecord {
            tick: 7,
            obs: vec![1.0, 2.5, f32::NAN],
            action: vec![0.0, -1.0],
            clamped: ClampReport { attacks_dropped_rate: 2, ..Default::default() },
        };
        let s = r.to_json_line();
        assert!(s.starts_with('{') && s.ends_with('}') && !s.contains('\n'));
        for k in ["\"tick\":7", "\"obs\":[1,2.5,null]", "\"action\":[0,-1]", "\"clamped\":{", "\"attacks_dropped_rate\":2", "\"attacks_dropped_reach\":0", "\"looks_limited\":0", "\"attacks_dropped_cargo\":0"] {
            assert!(s.contains(k), "{k} in {s}");
        }
        assert_eq!(s.matches('{').count(), s.matches('}').count());
    }
}
