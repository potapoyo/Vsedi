# ADR 0008: Rust / TypeScript 間の型共有

- 状態: 採用
- 日付: 2026-08-10

## 背景

Vsedi は Tauri v2 を利用し、権限を伴う処理や Git / filesystem 操作を Rust 側へ集約する。一方、UI は React + TypeScript で実装する。

Rust と TypeScript で同じデータ構造を手作業で二重定義すると、フィールド追加・enum 変更・nullability の違いなどによる不整合が発生しやすい。

特に Vsedi では、環境診断、プロジェクト診断、変更ファイル、履歴、復元プレビュー、環境バックアップ manifest など、多数の構造化データを Rust から Frontend へ渡す予定である。

## 決定

**Rust 側の型定義を正本（Source of Truth）とし、`serde` + `ts-rs` を利用して TypeScript 型を生成する。**

Rust の domain / command response 用 struct・enum は、必要に応じて次の trait を利用する。

- `serde::Serialize`
- `serde::Deserialize`
- `ts_rs::TS`

TypeScript 側では、自動生成された型を利用し、同じ構造を手作業で再定義しない。

## 対象となる型の例

- `EnvironmentDiagnostic`
- `ProjectDiagnostic`
- `GitEnvironment`
- `ChangedFile`
- `CommitInfo`
- `RevisionDetail`
- `RestorePreview`
- `BackupManifest`
- 構造化エラー型

## Tauri command との関係

Frontend からは汎用的な shell / Git command を呼ばず、アプリケーション上の意図を表す Tauri command を利用する。

例:

- `inspect_environment`
- `inspect_project`
- `save_work`
- `get_history`
- `preview_restore`

TypeScript 側では、生成された response / request 型を使って `invoke<T>()` をラップする。

command 名や引数・戻り値まで完全自動生成する仕組みは M1 の必須条件にはしない。

## tauri-specta について

`tauri-specta` のような command binding 自動生成ライブラリは有力な候補だが、初期基盤では追加の依存・生成方式を増やさず、`serde + ts-rs` を採用する。

将来、Tauri command 数が増え、手動 wrapper の保守負担が大きくなった場合は再評価する。

## 生成物

生成された TypeScript 型は、Frontend から import しやすい専用 directory に配置する。

例:

```text
src/
  generated/
    bindings/
```

生成物を Git に含めるか、build/test 時に必ず生成するかは M1 実装時に決める。ただし CI では Rust 型と TypeScript 型の不整合を検出できる構成にする。

## 命名と互換性

- JSON 上の field naming は明示的に統一する
- enum の serialization 方式を暗黙に変更しない
- backup format の公開構造は ADR 0007 の `formatVersion` と合わせて互換性を管理する
- Rust 内部だけで使う型と Frontend / backup file に公開する型を必要に応じて分離する

## 理由

- Rust 側の実装が権限境界と domain logic の正本である
- Rust / TypeScript の二重定義を減らせる
- serde による実際の JSON 形式と TypeScript 型を近づけられる
- 比較的単純な導入で、M1 の基盤を過度に複雑化しない

## 影響

良い点:

- Frontend / Rust 間の型ずれを減らせる
- Rust の変更を TypeScript 側へ反映しやすい
- backup manifest や診断結果などの構造化データを一貫して扱える

注意点:

- Tauri command wrapper 自体は別途実装する必要がある
- serde 属性と TypeScript 生成結果の整合性をテストする必要がある
- `ts-rs` の更新時には生成差分を確認する必要がある

## 再検討する条件

- command binding の手動管理が大きな負担になった場合
- より安定した Tauri 向け end-to-end binding generator が必要になった場合
- OpenAPI / JSON Schema 等、別の schema を正本とする必要が生じた場合
