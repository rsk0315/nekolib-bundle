use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::{Path, PathBuf},
    string::ToString,
};

use syn::{parse_file, Item};

use crate::{
    metadata::Metadata,
    polish::polish_library,
    resolve::resolve_nested_mod,
    source::{dfs_use_tree, Source},
};

pub const LIBRARY_NAME: &'static str = "nekolib";
pub const LIB_PATH_DEFAULT: &'static str = "~/git/rsk0315/nekolib/nekolib-doc";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum UseIdent {
    MacroExport(String),
    PubItem(String),
}

impl ToString for UseIdent {
    fn to_string(&self) -> String {
        match self {
            UseIdent::MacroExport(s) | UseIdent::PubItem(s) => s.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Crate {
    category_name: String,
    crate_name: String,
}

impl Crate {
    fn new(category_name: String, crate_name: String) -> Self {
        Self { category_name, crate_name }
    }
    fn from_path(path: &Path) -> Self {
        let crate_name = path.file_name().unwrap().to_str().unwrap().to_owned();
        let parent = path.parent().unwrap();
        let category_name =
            parent.file_name().unwrap().to_str().unwrap().to_owned();
        Self { category_name, crate_name }
    }
}

impl std::fmt::Display for Crate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.category_name, self.crate_name)
    }
}

pub struct Library {
    // path: PathBuf,
    ident_crate: BTreeMap<Vec<String>, Crate>,
    crate_path: BTreeMap<Crate, PathBuf>,
    deps_1: BTreeMap<Crate, Vec<Crate>>,
    deps_oo: BTreeMap<Crate, BTreeSet<Crate>>,
    macro_exports: BTreeMap<Crate, Vec<String>>,
}

impl Library {
    pub fn traverse(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        // [foo::foo2] -> [foo::foo1, ...] <=> foo::foo2 depends on foo::foo1
        let mut deps_1 = BTreeMap::new();

        // [foo1_fn] -> foo::foo1 <=> foo::foo1 exports foo1_fn
        let mut ident_crate = BTreeMap::new();

        // [foo::foo1] -> nekolib-src/foo/foo1/src/lib.rs
        let mut crate_path = BTreeMap::new();

        let mut macro_exports = BTreeMap::new();

        // path: nekolib-doc
        for (k0, v0) in dependency_paths(path.join("Cargo.toml")) {
            // v0: nekolib-src/foo_category
            for (k1, v1) in dependency_paths(v0.join("Cargo.toml")) {
                // v1: nekolib-src/foo_category/foo_crate
                let foo = Crate::new(k0.clone(), k1.clone());
                for (_k2, v2) in dependency_paths(v1.join("Cargo.toml")) {
                    // v2: nekolib-src/bar_category/bar_crate
                    let bar = Crate::from_path(&v2);

                    // foo_crate depends on bar_crate
                    deps_1.entry(foo.clone()).or_insert(vec![]).push(bar);
                }

                let mut mx = vec![];
                // nekolib-src/foo_category/foo_crate/src/lib.rs
                for item in export_items(v1.join("src/lib.rs")) {
                    ident_crate.insert(
                        vec![k0.clone(), item.to_string()],
                        foo.clone(),
                    );
                    ident_crate.insert(
                        vec![k0.clone(), k1.clone(), item.to_string()],
                        foo.clone(),
                    );
                    if let UseIdent::MacroExport(s) = &item {
                        mx.push(s.to_owned());
                    }
                }
                macro_exports.insert(foo.clone(), mx);
                crate_path.insert(foo, v1.join("src/lib.rs"));
            }
        }

        let deps_oo = transitive(&deps_1);

        Ok(Self {
            ident_crate,
            crate_path,
            deps_1,
            deps_oo,
            macro_exports,
        })
    }

    pub fn bundle(&self, source: &Source, metadata: &Metadata) -> String {
        // {[foo] => [foo1, ...], ...}
        let required = self.required(&source.get_uses());

        // original file
        let mut res = source.get_code().to_owned();

        if !required.is_empty() {
            res += &format!(
                r#"
/// This module is bundled automatically.
/// See <https://rsk0315.github.io/nekolib/nekolib_doc/index.html> for documentation.
/// Commit: {}
#[allow(unused)]
"#,
                metadata.get_commit()
            );
            res += &format!("pub mod {LIBRARY_NAME} {{");
            for (cat, v) in required {
                res += &format!("pub mod {cat} {{\n");
                for (cr, path) in v {
                    res += &format!("    pub mod {cr} {{\n");

                    let key = Crate::new(cat.clone(), cr.clone());
                    if let Some(deps_1) = self.deps_1.get(&key) {
                        for dep in deps_1 {
                            res += &format!(
                                "        use crate::{LIBRARY_NAME}::{}::{};",
                                dep.category_name, dep.crate_name,
                            )
                        }
                    }

                    res += &bundle_file(&path, &cat, &cr);

                    if let Some(mx) = self.macro_exports.get(&key) {
                        if !mx.is_empty() {
                            let mx = mx.join(", ");
                            res +=
                                &format!("        pub(crate) use {{{mx}}};\n");
                        }
                    }
                    res += &format!("    }}\n");
                    res += &format!("    #[allow(unused_imports)]\n");
                    res += &format!("    pub use {cr}::*;\n");
                }
                res += &format!("}}\n");
            }
            res += &format!("}}\n");
        }

        res
    }

    fn required(
        &self,
        uses: &[Vec<String>],
    ) -> BTreeMap<String, Vec<(String, PathBuf)>> {
        let mut bundled = vec![];
        for u in uses {
            let cr = self.ident_crate[u].clone();
            bundled.push(cr.clone());
            if let Some(deps) = self.deps_oo.get(&cr) {
                bundled.extend(deps.iter().cloned());
            }
        }

        bundled.sort_unstable();
        bundled.dedup();

        let mut res = BTreeMap::new();
        for b in &bundled {
            res.entry(b.category_name.to_owned())
                .or_insert(vec![])
                .push((b.crate_name.to_owned(), self.crate_path[b].clone()));
        }

        res
    }
}

fn dependency_paths(
    cargo_toml_path: impl AsRef<Path>,
) -> BTreeMap<String, PathBuf> {
    let manifest = cargo_toml::Manifest::from_path(&cargo_toml_path).unwrap();
    let crate_dir = cargo_toml_path.as_ref().parent().unwrap();
    manifest
        .dependencies
        .into_iter()
        .filter_map(|(name, dep)| {
            dep.detail().and_then(|d| d.path.clone()).map(|path| {
                let name = name.replace("-", "_");
                let path = crate_dir.join(path).canonicalize().unwrap();
                (name, path)
            })
        })
        .collect()
}

fn export_items(src_lib_path: impl AsRef<Path>) -> Vec<UseIdent> {
    let src = std::fs::read_to_string(src_lib_path).unwrap();
    let ast = parse_file(&src).unwrap();

    let mut res = vec![];

    for item in ast.items {
        let (vis, ident) = match item {
            Item::Const(item) => (item.vis, item.ident),
            Item::Enum(item) => (item.vis, item.ident),
            Item::Fn(item) => (item.vis, item.sig.ident),
            Item::Mod(item) => (item.vis, item.ident),
            Item::Static(item) => (item.vis, item.ident),
            Item::Struct(item) => (item.vis, item.ident),
            Item::Trait(item) => (item.vis, item.ident),
            Item::TraitAlias(item) => (item.vis, item.ident),
            Item::Type(item) => (item.vis, item.ident),
            Item::Union(item) => (item.vis, item.ident),
            Item::Use(item) => {
                let mut cur = vec![];
                let tmp = dfs_use_tree(&item.tree, &mut cur);
                if is_pub(item.vis) {
                    for u in tmp {
                        let ident = u.last().unwrap().to_string();
                        res.push(UseIdent::PubItem(ident));
                    }
                }
                continue;
            }

            Item::Macro(item) => {
                if is_exported(&item.attrs) {
                    if let Some(ident) = item.ident {
                        res.push(UseIdent::MacroExport(ident.to_string()));
                    }
                }
                continue;
            }

            _ => continue,
        };

        if is_pub(vis) {
            res.push(UseIdent::PubItem(ident.to_string()));
        }
    }

    res
}

fn is_pub(vis: syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn is_exported(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .get_ident()
            .map(std::string::ToString::to_string)
            .map(|ident| ident == "macro_export")
            .unwrap_or(false)
    })
}

fn transitive(
    deps1: &BTreeMap<Crate, Vec<Crate>>,
) -> BTreeMap<Crate, BTreeSet<Crate>> {
    let mut res = BTreeMap::<_, BTreeSet<_>>::new();
    for key in tsort(&deps1).rev() {
        if let Some(deps1) = deps1.get(&key).cloned() {
            let mut tmp = BTreeSet::new();
            for dep in deps1 {
                tmp.insert(dep.clone());
                if let Some(dep_tr) = res.get(&dep) {
                    tmp.extend(dep_tr.clone());
                }
            }
            res.insert(key, tmp);
        }
    }
    res
}

fn tsort(
    deps1: &BTreeMap<Crate, Vec<Crate>>,
) -> impl Iterator<Item = Crate> + DoubleEndedIterator {
    let mut indeg = {
        let mut indeg = BTreeMap::new();
        for (_, v) in deps1 {
            for nk in v {
                *indeg.entry(nk.clone()).or_insert(0) += 1;
            }
        }
        indeg
    };

    let mut res: Vec<_> =
        deps1.keys().cloned().filter(|k| !indeg.contains_key(k)).collect();
    let mut q: VecDeque<_> = res.iter().cloned().collect();

    while let Some(k) = q.pop_front() {
        if let Some(v) = deps1.get(&k) {
            for nk in v.clone() {
                *indeg.get_mut(&nk).unwrap() -= 1;
                if indeg[&nk] == 0 {
                    res.push(nk.clone());
                    q.push_back(nk);
                }
            }
        }
    }

    res.into_iter()
}

fn bundle_file(path: &Path, cat: &str, cr: &str) -> String {
    let expanded = resolve_nested_mod(path);
    polish_library(&expanded, cat, cr)
}
