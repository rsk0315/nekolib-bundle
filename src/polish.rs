use quote::quote;
use syn::{parse_file, visit_mut::VisitMut};

pub fn polish_library(src: &str) -> String {
    let mut ast = parse_file(src).unwrap();

    remove_doc_attrs(&mut ast.attrs);
    remove_test_items(&mut ast.items);
    remove_doc_comments(&mut ast);

    (quote! { #ast }).to_string()
}

fn remove_doc_attrs(attrs: &mut Vec<syn::Attribute>) {
    attrs.retain(|attr| !attr.meta.path().is_ident("doc"));
}

fn remove_test_items(items: &mut Vec<syn::Item>) {
    items.retain(|item| {
        let attrs = match item {
            syn::Item::Const(item) => &item.attrs,
            syn::Item::Enum(item) => &item.attrs,
            syn::Item::ExternCrate(item) => &item.attrs,
            syn::Item::Fn(item) => &item.attrs,
            syn::Item::ForeignMod(item) => &item.attrs,
            syn::Item::Impl(item) => &item.attrs,
            syn::Item::Macro(item) => &item.attrs,
            syn::Item::Mod(item) => &item.attrs,
            syn::Item::Static(item) => &item.attrs,
            syn::Item::Struct(item) => &item.attrs,
            syn::Item::Trait(item) => &item.attrs,
            syn::Item::TraitAlias(item) => &item.attrs,
            syn::Item::Type(item) => &item.attrs,
            syn::Item::Union(item) => &item.attrs,
            syn::Item::Use(item) => &item.attrs,
            _ => unimplemented!(),
        };
        attrs.iter().all(|attr| !is_test_attr(&attr))
    });
}

struct RemoveDocComments;

impl VisitMut for RemoveDocComments {
    fn visit_item_mut(&mut self, node: &mut syn::Item) {
        eprintln!("visited: {}", quote! { #node });

        let attrs = match node {
            syn::Item::Const(item) => &mut item.attrs,
            syn::Item::Enum(item) => &mut item.attrs,
            syn::Item::ExternCrate(item) => &mut item.attrs,
            syn::Item::Fn(item) => &mut item.attrs,
            syn::Item::ForeignMod(item) => &mut item.attrs,
            syn::Item::Impl(item) => {
                for item in &mut item.items {
                    self.visit_impl_item_mut(item);
                }
                &mut item.attrs
            }
            syn::Item::Macro(item) => &mut item.attrs,
            syn::Item::Mod(item) => {
                if let Some((_, contents)) = &mut item.content {
                    for content in contents {
                        self.visit_item_mut(content);
                    }
                }
                &mut item.attrs
            }
            syn::Item::Static(item) => &mut item.attrs,
            syn::Item::Struct(item) => &mut item.attrs,
            syn::Item::Trait(item) => &mut item.attrs,
            syn::Item::TraitAlias(item) => &mut item.attrs,
            syn::Item::Type(item) => &mut item.attrs,
            syn::Item::Union(item) => &mut item.attrs,
            syn::Item::Use(item) => &mut item.attrs,
            _ => unimplemented!(),
        };

        remove_doc_attrs(attrs);
    }

    fn visit_impl_item_mut(&mut self, node: &mut syn::ImplItem) {
        eprintln!("visited: {}", quote! { #node });

        let attrs = match node {
            syn::ImplItem::Const(item) => &mut item.attrs,
            syn::ImplItem::Fn(item) => &mut item.attrs,
            syn::ImplItem::Macro(item) => &mut item.attrs,
            syn::ImplItem::Type(item) => &mut item.attrs,
            _ => unimplemented!(),
        };

        remove_doc_attrs(attrs);
    }
}

fn remove_doc_comments(ast: &mut syn::File) {
    RemoveDocComments.visit_file_mut(ast);
}

fn is_test_attr(attr: &syn::Attribute) -> bool {
    match &attr.meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) if list.path.is_ident("cfg") => {
            list.path.is_ident("cfg") && list.tokens.to_string() == "test"
        }
        _ => false,
    }
}

#[test]
fn removal() {
    use proc_macro2::TokenStream;
    use quote::quote;

    let src = r#"
//! doc

/// doc
#[allow(unused)]
fn one(x: i32) -> i32 { 1 }

/// doc struct
struct Foo;

impl Foo {
    /// doc fn
    fn foo(&self) {}
}

/// doc mod
mod libs {
    /// doc struct
    struct Bar;

    impl Bar {
        /// doc fn
        fn bar(&self) {}
    }

    /// doc inner mod
    mod internal {
        /// doc fn
        fn baz() -> i32 { 1 }
    }
}

#[test]
fn test() { assert_eq!(one(0), 1); }

#[cfg(test)]
mod tests {
    #[test]
    fn test() { assert!(true); }
}
"#;

    let actual = polish_library(src);
    let expected = r#"
#[allow(unused)]
fn one(x: i32) -> i32 { 1 }

struct Foo;

impl Foo {
    fn foo(&self) {}
}

mod libs {
    struct Bar;

    impl Bar {
        fn bar(&self) {}
    }

    mod internal {
        fn baz() -> i32 { 1 }
    }
}
"#;

    let actual = parse_file(&actual).unwrap();
    let expected = parse_file(&expected).unwrap();

    let actual = TokenStream::from(quote! { #actual });
    let expected = TokenStream::from(quote! { #expected });

    eprintln!("{actual}");
    eprintln!("{expected}");
    assert_eq!(actual.to_string(), expected.to_string());
}
