# CLAUDE.md

仓库 = Rust 端口 + 浏览器 GUI + 跟 Java 上游对拍。

## 仓库结构

- `crates/m2p-core/` — 算法库 (`Tables`, `Solver`, `tools::*`)
- `crates/m2p-cli/` — `m2p` 二进制(solve/scramble/random/bench/daemon)+ `m2p_shootout`(对拍专用)
- `crates/m2p-wasm/` — wasm-bindgen 包装,出 `Min2Phase` class
- `pkg/` — wasm-pack 产物 + 单页 GUI(`index.html` + `app.js`)。**已 commit**,clone 后 server 起来就能跑
- `benches/solve.rs` — criterion 微基准
- `fixtures/java_*.tsv` — Java 端跑出来的 ground truth,2000 cube 是默认对拍输入
- Java 上游在 `D:\cube\min2phase\` — 只读,不改

## 常用命令

```pwsh
cargo build --release --workspace
cargo test -p m2p-core --lib --release           # 35 个,~3s
cargo run -p m2p-cli --release -- bench 1000     # 默认走 m2p bin
wasm-pack build crates/m2p-wasm --release --target web --out-dir ../../pkg
```

## 性能基线(2000-cube shootout,3-run median)

Rust ~20% 快 solve / 2.3× 快 init,长度直方图跟 Java **bit-perfect**(15:1 16:1 17:3 18:20 19:119 20:514 21:1342)。单次 run 噪声大(±20%),数据要看 3 次中位数。

## 不要做

- **不要给 `Min2Phase::solve()` / `m2p solve` 加 `INVERSE_SOLUTION`**。默认正向解(apply 到乱状态→还原)才符合用户直觉。`m2p_shootout.rs` 用 INVERSE_SOLUTION 是因为它要 `from_scramble(sol)==cube` 做 round-trip 验证,不要动。
- **不改 Java 上游**。要新增 fixture / shootout 的 java 代码就只动 `D:\cube\min2phase\test\` 下的新文件。
- **wasm 默认 `--target web`**,不要换 `no-modules`(GUI 已经写成 ES module 形态)。`file://` 不工作是浏览器安全限制,所有 WASM 项目同等,无解,只能 server。
- **不改 `fixtures/java_*.tsv`** —— 那是测试 baseline,改了等于失效。

## 环境踩坑

- **Java 源码扁平在 `src/*.java`**,虽然 package 是 `cs.min2phase`。javac 不用 cs/min2phase/ 子目录路径,直接 `javac -d test/classes test/Shootout.java src/*.java`。
- **PowerShell 不展开 `*.java`**,javac 也不展开。要 `Get-ChildItem src/*.java | %{ $_.FullName }` 拿到列表再 splat 给 javac。
- **`cargo run -p m2p-cli --release -- <subcmd>`** 之所以能省 `--bin m2p` 是因为 `Cargo.toml` 里写了 `default-run = "m2p"`,别删。
- **GUI 必须走 server**。开发时 `python -m http.server -d pkg 8000`,浏览器开 `http://localhost:8000/`。
- **wasm-pack 每次 build 会重新生成 `pkg/.gitignore`(内容 `*`)**,build 完手动删掉,否则 `git add pkg/` 啥都进不来。

## 测试纪律

- 改 `m2p-core` 的算法层 → 跑 `cargo test -p m2p-core --lib --release` 必须全过(其中 `solves_java_fixture_100` 是关键)。
- 改 `tools::apply_moves` / `from_scramble` 的 parser → 跑 `apply_moves_*` 4 个测试。
- 改性能相关代码 → 跑 `cargo bench -p m2p-core`,看 `solve_random/max_depth_21` 区间(应稳定在 550-700 µs)。
- 改完 GUI → playwright 跑一遍随机+solve+applyMoves round-trip,不要只看截图。

## 不要主动 commit

改完不主动 `git add` / `git commit`,等用户明示再提交。commit message 一律英文。
