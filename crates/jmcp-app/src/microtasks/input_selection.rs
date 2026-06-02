use super::{local_model_roots, MicrotaskDefinition};

pub(super) fn default_model_roots(definition: &MicrotaskDefinition) -> Vec<String> {
    if definition.accepts_model_root {
        local_model_roots()
    } else {
        Vec::new()
    }
}

pub(super) fn microtask_repo(
    definition: &MicrotaskDefinition,
    override_repo: Option<String>,
) -> Option<String> {
    match override_repo {
        Some(repo) => Some(repo),
        None if definition.accepts_repo => Some(".".to_owned()),
        None => None,
    }
}

pub(super) fn microtask_concept(
    definition: &MicrotaskDefinition,
    override_concept: Option<String>,
) -> Option<String> {
    match override_concept {
        Some(concept) => Some(concept),
        None => definition.default_concept.map(str::to_owned),
    }
}

pub(super) fn microtask_model_roots(
    definition: &MicrotaskDefinition,
    override_root: Option<String>,
) -> Vec<String> {
    if !definition.accepts_model_root {
        return Vec::new();
    }
    match override_root {
        Some(root) => vec![root],
        None => local_model_roots(),
    }
}

pub(super) fn optional_label<'a>(value: Option<&'a str>, default_label: &'a str) -> &'a str {
    match value {
        Some(value) => value,
        None => default_label,
    }
}
