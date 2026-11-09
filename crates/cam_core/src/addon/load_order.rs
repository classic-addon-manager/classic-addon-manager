use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow};

use super::AddonDir;

#[derive(Clone, Copy, PartialEq)]
enum Color {
    White,
    Gray,
    Black,
}

/// Orders `names` so every addon appears after all addons it (transitively) depends on.
/// Only edges whose target is also in `names` are followed. A cycle yields an error.
pub fn resolve_load_order(names: &[String], deps_by_name: &HashMap<String, Vec<String>>) -> Result<Vec<String>> {
    fn visit(
        name: &str,
        path: &mut Vec<String>,
        present: &HashSet<&str>,
        deps: &HashMap<String, Vec<String>>,
        color: &mut HashMap<String, Color>,
        ordered: &mut Vec<String>,
    ) -> Result<()> {
        match color.get(name).copied().unwrap_or(Color::White) {
            Color::Black => return Ok(()),
            Color::Gray => {
                let mut cycle = path.clone();
                cycle.push(name.to_string());
                return Err(anyhow!("circular dependency detected: {}", cycle.join(" -> ")));
            }
            Color::White => {}
        }
        color.insert(name.to_string(), Color::Gray);
        if let Some(list) = deps.get(name) {
            for dep in list {
                if !present.contains(dep.as_str()) {
                    continue;
                }
                path.push(name.to_string());
                let r = visit(dep, path, present, deps, color, ordered);
                path.pop();
                r?;
            }
        }
        color.insert(name.to_string(), Color::Black);
        ordered.push(name.to_string());
        Ok(())
    }

    let present: HashSet<&str> = names.iter().map(String::as_str).collect();
    let mut color = HashMap::new();
    let mut ordered = Vec::with_capacity(names.len());
    for name in names {
        visit(name, &mut Vec::new(), &present, deps_by_name, &mut color, &mut ordered)?;
    }
    Ok(ordered)
}

impl AddonDir<'_> {
    /// Rewrites addons.txt so dependencies load before their dependents (managed addons' dependency data).
    /// Only reorders listed addons. On a cycle the file is left untouched and the error returned.
    pub fn sort_addons_txt(&self) -> Result<()> {
        let _lists = self.lock();
        let names = self.read_lines()?;
        if names.is_empty() {
            return Ok(());
        }

        let deps: HashMap<String, Vec<String>> =
            self.read_managed()?.addons.into_iter().map(|(name, a)| (name, a.dependencies)).collect();

        let ordered = resolve_load_order(&names, &deps)?;
        if ordered == names {
            return Ok(());
        }
        self.write_lines(&ordered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }
    fn deps(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs.iter().map(|(k, v)| (k.to_string(), s(v))).collect()
    }

    #[test]
    fn dependencies_come_first() {
        let out = resolve_load_order(&s(&["a", "b", "c"]), &deps(&[("a", &["b"]), ("b", &["c"])])).unwrap();
        assert_eq!(out, s(&["c", "b", "a"]));
    }

    #[test]
    fn stable_without_dependencies_and_ignores_absent_targets() {
        let out = resolve_load_order(&s(&["x", "y", "z"]), &deps(&[("x", &["missing"])])).unwrap();
        assert_eq!(out, s(&["x", "y", "z"]));
    }

    #[test]
    fn cycle_is_an_error() {
        let err = resolve_load_order(&s(&["a", "b"]), &deps(&[("a", &["b"]), ("b", &["a"])])).unwrap_err();
        assert_eq!(err.to_string(), "circular dependency detected: a -> b -> a");
    }

    #[test]
    fn self_cycle() {
        let err = resolve_load_order(&s(&["a"]), &deps(&[("a", &["a"])])).unwrap_err();
        assert_eq!(err.to_string(), "circular dependency detected: a -> a");
    }

    #[test]
    fn diamond_listed_once() {
        let out = resolve_load_order(
            &s(&["top", "l", "r", "base"]),
            &deps(&[("top", &["l", "r"]), ("l", &["base"]), ("r", &["base"])]),
        )
        .unwrap();
        assert_eq!(out, s(&["base", "l", "r", "top"]));
    }
}
