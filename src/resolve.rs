use std::{fs, path::Path};

pub fn resolve_nested_mod(path: &Path) -> String {
    // TODO: when we find `mod foo;`, we should read the corresponding
    // file (see <https://doc.rust-lang.org/reference/items/modules.html>)
    // and return `mod foo { ... }`. The procedure should be called
    // recursively.
    fs::read_to_string(path).unwrap()
}
