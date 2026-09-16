# abs_buff

ABStraction of BUFFered IO.

This crate provides cancellation-safe traits for both buffered or unbuffered io devices.

## 公开工具

- `ReadySegm<S, E>`：一个「立即就绪」的 future，产出 `SomeOf<S, E>`。实现
  `TrInput` / `TrOutput` 时可直接用它作为 `ReadAsync<'f>` / `WriteAsync<'f>`
  的具体类型。

## 测试

```sh
cargo test
```

测试依赖同级的 `abs_buff-testkit`（共享测试设备 + 泛型断言函数），它在
`[dev-dependencies]` 里以 **path** 引用；由于 `abs_buff-testkit` 又以 git 依赖
引用 `abs_buff` 自身，`Cargo.toml` 里有一段 `[patch]` 把它指回本仓库，保证测试
用的是本地这份 `abs_buff`：

```toml
[dev-dependencies]
abs_buff-testkit = { path = "../abs_buff-testkit" }

[patch."https://gitee.com/lino_snsalias/abs_buff.git"]
abs_buff = { path = "." }
```

因此跑测试时需要 `../abs_buff-testkit` 存在。

另有两条集成测试（`tests/segm_.rs`、`tests/pipelining_.rs`）专门承载依赖
testkit 的用例：`cargo test` 会为单元测试**另外**编译一份 crate，它与 testkit
链接的那份不是同一个 crate 实例，跨实例的 trait 实现无法匹配，所以这类用例
只能放在集成测试里。
