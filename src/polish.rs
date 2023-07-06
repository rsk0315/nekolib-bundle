use quote::quote;
use syn::{parse_file, spanned::Spanned, visit_mut::VisitMut};

pub fn polish_library(src: &str) -> String {
    let src_ascii: String = src.chars().filter(|&c| c.is_ascii()).collect();
    let mut ast = parse_file(&src_ascii).unwrap();

    remove_attrs_by_ident(&mut ast.attrs, "doc");
    remove_doc_comments(&mut ast);
    remove_test_items(&mut ast.items);
    remove_macro_exports(&mut ast);

    restore_macro_sources(&ast)
}

fn restore_macro_sources(ast: &syn::File) -> String {
    let src_tk = (quote! { #ast }).to_string();
    let ast_tk = parse_file(&src_tk).unwrap();

    let mut res = "".to_owned();
    for (item, item_tk) in ast.items.iter().zip(&ast_tk.items) {
        if let syn::Item::Macro(item) = item {
            if !res.is_empty() && !res.ends_with("\n") {
                res += "\n";
            }
            res += &item
                .span()
                .source_text()
                .unwrap()
                .replace("\n", &format!("{:<13}", '\n'));
            res += "\n";
        } else {
            res += &item_tk.span().source_text().unwrap();
        }
    }

    res
}

fn remove_attrs_by_ident(attrs: &mut Vec<syn::Attribute>, ident: &str) {
    attrs.retain(|attr| !attr.meta.path().is_ident(ident));
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

        remove_attrs_by_ident(attrs, "doc");
    }

    fn visit_impl_item_mut(&mut self, node: &mut syn::ImplItem) {
        let attrs = match node {
            syn::ImplItem::Const(item) => &mut item.attrs,
            syn::ImplItem::Fn(item) => &mut item.attrs,
            syn::ImplItem::Macro(item) => &mut item.attrs,
            syn::ImplItem::Type(item) => &mut item.attrs,
            _ => unimplemented!(),
        };

        remove_attrs_by_ident(attrs, "doc");
    }
}

fn remove_doc_comments(ast: &mut syn::File) {
    RemoveDocComments.visit_file_mut(ast);
}

struct RemoveMacroExports;

impl VisitMut for RemoveMacroExports {
    fn visit_item_macro_mut(&mut self, node: &mut syn::ItemMacro) {
        remove_attrs_by_ident(&mut node.attrs, "macro_export");
    }
}

fn remove_macro_exports(ast: &mut syn::File) {
    RemoveMacroExports.visit_file_mut(ast);
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

#[test]
fn macro_export() {
    use proc_macro2::TokenStream;
    use quote::quote;

    let src = r#"
#[macro_export]
macro_rules! foo {
    () => {}
}
"#;

    let actual = polish_library(src);
    let expected = "macro_rules! foo { () => {} }";

    let actual = parse_file(&actual).unwrap();
    let expected = parse_file(&expected).unwrap();

    let actual = TokenStream::from(quote! { #actual });
    let expected = TokenStream::from(quote! { #expected });

    eprintln!("{actual}");
    eprintln!("{expected}");
    assert_eq!(actual.to_string(), expected.to_string());
}

#[test]
fn macro_fmt() {
    use syn::{spanned::Spanned, visit::Visit};

    let original = r##"
macro_rules! foo {
    ( $($ty:ty)* ) => { $(
        impl Foo for $ty {
            fn foo(self) -> $ty {
                0 as $ty
            }
        }
    )* }
}

fn bar() {}

macro_rules! baz {
    () => { r#"..
..
.."#
    }
}
"##;

    struct VisitItemMacro {
        spans: Vec<proc_macro2::Span>,
    }

    impl<'ast> Visit<'ast> for VisitItemMacro {
        fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
            eprintln!("{:?}", node.ident);
            eprintln!("{:?}", node.mac.span());
            self.spans.push(node.mac.span());
        }
    }

    let file = syn::parse_file(&original).unwrap();
    let mut vim = VisitItemMacro { spans: vec![] };
    vim.visit_file(&file);
    for span in vim.spans {
        eprintln!("{span:?}; {:?}..{:?}", span.start(), span.end());
        eprintln!("{}", span.source_text().unwrap());
    }

    let ts = original.parse::<proc_macro2::TokenStream>().unwrap();
    let splitted = ts.to_string();
    println!("{splitted}");

    let file = syn::parse_file(&splitted).unwrap();
    let mut vim = VisitItemMacro { spans: vec![] };
    vim.visit_file(&file);
    for span in vim.spans {
        eprintln!("{span:?}; {:?}..{:?}", span.start(), span.end());
        eprintln!("{}", span.source_text().unwrap());
    }
}

/*
macro_rules ! foo { ($ ($ ty : ty) *) => { $ (impl Foo for $ ty { fn foo (self) -> $ ty { 0 as $ ty } }) * } } fn bar () { } macro_rules ! baz { () => { r#"..
..
.."# } }
 */

/*
macro_rules! foo {
    ( $($ty:ty)* ) => { $(
        impl Foo for $ty {
            fn foo(self) -> $ty {
                0 as $ty
            }
        }
    )* }
} fn bar () { } macro_rules! baz {
    () => { r#"..
..
.."#
    }
}
*/
