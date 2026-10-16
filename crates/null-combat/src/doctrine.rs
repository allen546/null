use crate::{ParamSpace, Params};

/// One technique version as a small text file:
/// `technique`, `version`, optional `parent`, then a `[params]` section.
/// Blank lines and `#` comments are accepted on input but not kept by `to_string`.
/// Params left out take the space default; `to_string` always writes all, in space order.
#[derive(Clone, Debug, PartialEq)]
pub struct Doctrine {
    pub technique: String,
    pub version: u32,
    pub parent: Option<u32>,
    pub params: Params,
}

/// Param space of each known technique.
pub fn space_for(technique: &str) -> Option<ParamSpace> {
    match technique {
        "melee_basic" => Some(crate::melee::MELEE_BASIC_SPACE),
        _ => None,
    }
}

impl Doctrine {
    pub fn parse(text: &str) -> Result<Doctrine, String> {
        let (mut technique, mut version, mut parent) = (None, None, None);
        let mut raw: Vec<(String, f64)> = Vec::new();
        let mut in_params = false;
        for (i, line) in text.lines().enumerate() {
            let n = i + 1;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "[params]" {
                in_params = true;
                continue;
            }
            let (k, v) = line.split_once('=').ok_or(format!("line {n}: expected `key = value`"))?;
            let (k, v) = (k.trim(), v.trim());
            if in_params {
                let x: f64 = v.parse().map_err(|_| format!("line {n}: bad number `{v}`"))?;
                if raw.iter().any(|(name, _)| name == k) {
                    return Err(format!("line {n}: duplicate param `{k}`"));
                }
                raw.push((k.to_string(), x));
                continue;
            }
            let num = || v.parse::<u32>().map_err(|_| format!("line {n}: bad integer `{v}`"));
            match k {
                "technique" => technique = Some(v.to_string()),
                "version" => version = Some(num()?),
                "parent" => parent = Some(num()?),
                _ => return Err(format!("line {n}: unknown key `{k}`")),
            }
        }
        let technique = technique.ok_or("missing `technique`")?;
        let version = version.ok_or("missing `version`")?;
        let space = space_for(&technique).ok_or(format!("unknown technique `{technique}`"))?;
        let mut params = space.defaults();
        for (name, x) in raw {
            if !params.set(&name, x) {
                return Err(format!("unknown param `{name}` for {technique}"));
            }
        }
        space.validate(&params)?;
        Ok(Doctrine { technique, version, parent, params })
    }

    pub fn space(&self) -> Option<ParamSpace> {
        space_for(&self.technique)
    }
}

impl std::fmt::Display for Doctrine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "technique = {}", self.technique)?;
        writeln!(f, "version = {}", self.version)?;
        if let Some(p) = self.parent {
            writeln!(f, "parent = {p}")?;
        }
        writeln!(f, "[params]")?;
        for (n, v) in &self.params.0 {
            writeln!(f, "{n} = {v}")?;
        }
        Ok(())
    }
}
