# nekolib-bundle

## 使用方法

ビルドしておく。

```
% cargo build --release
```

```
% /path/to/bundler [-l /path/to/library] main.rs
```

## 仕様

`main.rs` のトップレベルにある `use nekolib::{..}` を元にして、依存しているファイルを bundle する。

bundle の際、下記のものは除去される。

- comment および doc comment 
- `#[test]` および `#[cfg(test)]` のつく item
    - トップレベル以外は現状は放置される

ただし、`macro_rules!` の [*MacroRulesDef*](https://doc.rust-lang.org/reference/macros-by-example.html) の部分に関しては除去を行わない。
これは、トークン列に分解して処理した後のコードを rustfmt で整形する際、マクロ内が散らかってしまうことに対処するため、その部分は元コードをそのまま利用していることによる。
