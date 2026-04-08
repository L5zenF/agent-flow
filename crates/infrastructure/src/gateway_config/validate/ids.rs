use std::collections::HashSet;

pub(crate) fn unique_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    kind: &str,
) -> Result<HashSet<&'a str>, Box<dyn std::error::Error>> {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(format!("{kind} id cannot be empty").into());
        }
        if !seen.insert(id) {
            return Err(format!("duplicate {kind} id '{id}'").into());
        }
    }
    Ok(seen)
}
