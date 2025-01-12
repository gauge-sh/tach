// for serde
pub fn default_true() -> bool {
    true
}

pub fn global_visibility() -> Vec<String> {
    vec!["*".to_string()]
}

pub fn is_true(value: &bool) -> bool {
    *value
}

pub fn is_false(value: &bool) -> bool {
    !*value
}

pub fn is_empty<T>(value: &[T]) -> bool {
    value.is_empty()
}
