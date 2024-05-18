use syn::parse_file;

pub struct Source {
    content: String,
}

impl Source {
    pub fn new(content: String) -> Self { Self { content } }

    pub fn get_code(&self) -> &str { &self.content }

    pub fn get_uses(&self) -> Vec<Vec<String>> {
        let ast = parse_file(&self.content).unwrap();
        let mut res: Vec<_> =
            ast.items
                .into_iter()
                .filter_map(|i| {
                    if let syn::Item::Use(u) = i { Some(u) } else { None }
                })
                .flat_map(|u| {
                    let mut cur = vec![];
                    if u.leading_colon.is_some() {
                        cur.push("".to_owned());
                    }
                    dfs_use_tree(&u.tree, &mut cur)
                })
                .collect();

        res.sort_unstable();
        res.dedup();
        res.into_iter()
            .filter_map(|u| (u[0] == "nekolib").then(|| u[1..].to_vec()))
            .collect()
    }
}

pub fn dfs_use_tree(
    u: &syn::UseTree,
    cur: &mut Vec<String>,
) -> Vec<Vec<String>> {
    fn dfs(
        u: &syn::UseTree,
        cur: &mut Vec<String>,
        res: &mut Vec<Vec<String>>,
    ) {
        match u {
            syn::UseTree::Path(ref path) => {
                cur.push(path.ident.to_string());
                dfs(&path.tree, cur, res);
                cur.pop();
            }
            syn::UseTree::Name(ref name) => {
                cur.push(name.ident.to_string());
                res.push(cur.clone());
                cur.pop();
            }
            syn::UseTree::Rename(ref rename) => {
                cur.push(rename.rename.to_string());
                res.push(cur.clone());
                cur.pop();
            }
            syn::UseTree::Glob(_) => {
                cur.push("*".to_owned());
                res.push(cur.clone());
                cur.pop();
            }
            syn::UseTree::Group(ref group) => {
                for item in &group.items {
                    dfs(item, cur, res);
                }
            }
        }
    }

    let mut res = vec![];
    dfs(u, cur, &mut res);
    res
}
