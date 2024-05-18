use std::{
    fs,
    path::{Path, PathBuf},
};

use syn::{parse_file, spanned::Spanned};

pub fn resolve_nested_mod(path: &Path) -> String {
    // When we find `mod foo;`, we should read the corresponding file
    // (see <https://doc.rust-lang.org/reference/items/modules.html>)
    // and return `mod foo { ... }`. The procedure should be called
    // recursively.

    let src = fs::read_to_string(path).unwrap();
    let src_ascii: String = src.chars().filter(|&c| c.is_ascii()).collect();
    let ast = parse_file(&src_ascii).unwrap();

    let mut src_resolved = "".to_owned();
    for item in ast.items.iter() {
        let text = item.span().source_text().unwrap();
        if let syn::Item::Mod(item_mod) = item {
            if item_mod.content.is_none() {
                assert!(item_mod.semi.is_some());
                let mut text = text;
                let semi = text.pop();
                assert_eq!(semi, Some(';'));
                src_resolved += &text;

                let mod_path = resolve_mod_source(path, item_mod);
                src_resolved += "{";
                src_resolved += &resolve_nested_mod(&mod_path);
                src_resolved += "}";
                continue;
            }

            // FIXME: we miss `mod inline { #[path = "other.rs"] mod inner; }`.
        }
        src_resolved += &text;
    }
    src_resolved
}

pub fn resolve_mod_source(
    main_path: &Path,
    item_mod: &syn::ItemMod,
) -> PathBuf {
    let path_override = item_mod.attrs.iter().find_map(|attr| {
        if let syn::AttrStyle::Inner(_) = attr.style {
            return None;
        }
        if let syn::Meta::NameValue(mnv) = &attr.meta {
            let ident = mnv.path.get_ident()?.to_string();
            if ident != "path" {
                return None;
            }
            if let syn::Expr::Lit(expr_lit) = &mnv.value {
                if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                    return Some(lit_str.value());
                }
            }
        }
        None
    });

    let dir = main_path.parent().unwrap();
    let main = PathBuf::from(main_path.file_name().unwrap());
    let main_stem = main.file_stem().unwrap();
    if let Some(path) = path_override {
        dir.join(path)
    } else {
        let basename = format!("{}.rs", item_mod.ident.to_string());
        if main_stem == "mod" || main_stem == "lib" {
            dir.join(basename)
        } else {
            dir.join(main_stem).join(basename)
        }
    }
}

#[cfg(test)]
mod tests_mod_path {
    use std::path::PathBuf;

    use syn::parse_file;

    use super::*;

    #[test]
    fn sanity_check() {
        const SUITE: &[((&str, &str), &str)] = &[
            (("src/a/mod.rs", r#"#[path = "foo.rs"] mod c;"#), "src/a/foo.rs"),
            (("src/a/b.rs", r#"#[path = "foo.rs"] mod c;"#), "src/a/foo.rs"),
            (("src/a/mod.rs", "mod c;"), "src/a/c.rs"),
            (("src/a/b.rs", "mod c;"), "src/a/b/c.rs"),
        ];

        for ((main_path, file), mod_path) in SUITE {
            let file = parse_file(file).unwrap();
            let item = match &file.items[0] {
                syn::Item::Mod(i) => i,
                _ => unreachable!(),
            };
            let actual = resolve_mod_source(main_path.as_ref(), item);
            assert_eq!(actual, PathBuf::from(mod_path));
        }
    }
}
