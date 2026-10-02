//! Patterns that come from data (the anchor rules of profiles), compiled once per process: the
//! same rule runs on every file of a run, and compiling it each time cost more than matching.
//! Patterns written in code are `static`s next to their use instead.

use regex::Regex;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};

/// Patterns kept at most; the cache starts over when it is full (a run uses a handful).
const CAPACITY: usize = 256;

/// The compiled `pattern`, reused when it was compiled before. Cloning a `Regex` is cheap.
pub(crate) fn compiled(pattern: &str) -> Result<Regex, regex::Error> {
    static CACHE: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(found) = cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(pattern)
    {
        return Ok(found.clone());
    }
    let regex = Regex::new(pattern)?;
    let mut map = cache.lock().unwrap_or_else(PoisonError::into_inner);
    if map.len() >= CAPACITY {
        map.clear();
    }
    map.insert(pattern.to_string(), regex.clone());
    Ok(regex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_once_and_reports_errors() {
        let first = compiled(r"valor\s*liquido").unwrap();
        let again = compiled(r"valor\s*liquido").unwrap();
        assert_eq!(first.as_str(), again.as_str());
        assert!(again.is_match("valor liquido"));
        assert!(compiled("(sem fechar").is_err());
    }
}
