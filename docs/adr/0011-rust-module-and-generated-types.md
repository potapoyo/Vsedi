# ADR 0011: Rust モジュール構成と生成 TypeScript 型の管理

- 状態: 採用
- 日付: 2026-08-10

## 背景

M1 では Tauri command、Git 環境診断、設定、エラー、ログ等の Rust 実装が始まる。

Tauri command に実処理を直接書き込むと、UI 境界と domain logic が混ざり、後続の Unity / VRChat 診断、保存、履歴、復元機能が追加された際に保守しづらくなる。

また ADR 0008 により Rust を型定義の正本とし `serde + ts-rs` で TypeScript 型を生成することは確定しているが、生成物を Git に含めるかどうかは未確定だった。

## 決定

### M1 の Rust モジュール構成

M1 では次を基本構成とする。

```text
src-tauri/src/
├─ commands/
│  ├─ environment.rs
│  ├─ projects.rs
│  └─ mod.rs
├─ services/
│  ├─ diagnostics.rs
│  ├─ settings.rs
│  └─ mod.rs
├─ git/
│  ├─ command.rs
│  ├─ diagnostics.rs
│  └─ mod.rs
├─ platform/
│  ├─ paths.rs
│  ├─ process.rs
│  └─ mod.rs
├─ models/
│  ├─ diagnostics.rs
│  ├─ settings.rs
│  └─ mod.rs
├─ errors/
│  └─ mod.rs
├─ logging/
│  └─ mod.rs
├─ settings/
│  ├─ store.rs
│  ├─ migration.rs
│  └─ mod.rs
├─ lib.rs
└─ main.rs
```

M2 以降で必要になった時点で `unity/`、`vrchat/` 等を追加する。将来用の空モジュールを M1 で大量に作らない。

### 責務

- `commands/`: Tauri command の薄い境界。入力検証と application service 呼び出しを中心とし、Git / filesystem の実処理を書かない
- `services/`: Vsedi 上のユースケースを組み立てる
- `git/`: system Git CLI の安全な実行、出力 parsing、Git診断
- `platform/`: OS 固有 path / process 等の処理を隔離する
- `models/`: Frontend と共有する DTO や domain 上の構造化データ
- `errors/`: ADR 0009 の `AppError` / `ErrorCode`
- `logging/`: ADR 0010 のログ初期化、sanitize / redact、診断ログ関連
- `settings/`: ADR 0007 の `settings.json` 読み書き、validation、migration、退避処理

基本的な依存方向は次のようにする。

```text
Frontend
  ↓
commands
  ↓
services
  ↓
git / platform / settings
  ↓
models / errors
```

`commands` から任意 shell / Git command を直接受け取る汎用 API は作らない。

### `ts-rs` 生成物

`ts-rs` で生成した Frontend 用 TypeScript 型は **Git 管理する**。

理由:

- PR diff で Rust 側の型変更が Frontend 契約へどう影響したか確認できる
- Frontend の checkout 直後から型ファイルが存在し、生成ツール実行前でも参照できる
- release / CI / ローカル環境間で生成漏れを検出しやすい

生成物を手編集しない。Rust 型を変更した場合は専用の生成手順を実行して生成物を更新する。

### CI

CI では次を検証する。

1. Rust の型から TypeScript 型を再生成する
2. 再生成後に Git diff が発生しないことを確認する
3. 差分がある場合は CI を失敗させ、生成物の更新漏れとして扱う

これにより、Git に含まれる生成済み TypeScript 型と Rust の正本が常に一致していることを検証する。

生成を通常 build の暗黙 side effect のみに依存させず、明示的に実行・検証できる command / test を用意する。

## 影響

良い点:

- Tauri 固有境界と Git / filesystem 実処理を分離できる
- M2 以降の機能追加時も責務が明確になる
- Rust / TypeScript の型 drift を PR / CI で検出できる
- Frontend 開発時に生成型が常に checkout 済みの状態になる

注意点:

- Rust 型変更時には生成済み TypeScript ファイルも差分に含まれる
- CI 用の安定した型生成手順が必要になる
- `ts-rs` の version 更新時に生成差分が大量に発生する可能性がある

## 再検討する条件

- `ts-rs` より適した安定した型 / command 生成基盤へ移行する場合
- 生成物が極端に増え、Git 管理のコストが明確に問題となる場合
- Rust workspace / crate 分割により現在のモジュール構成が不自然になった場合
