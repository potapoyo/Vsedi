# ADR 0009: アプリケーションエラーモデル

- 状態: 一部置換（Git LFS診断部分をADR 0012で置換）
- 日付: 2026-08-10

## 背景

Vsedi は Git、ファイルシステム、Unity / VRChat プロジェクト、設定ファイル、復元処理など複数の外部状態を扱う。

OS の生エラー、Git の stderr、外部ツールの終了コードを Frontend が直接解釈すると、UI が実装詳細に依存し、Windows / macOS や Git version の違いで挙動が不安定になる。

また、Vsedi は安全性を優先するため、失敗時に「何が起きたか」だけでなく「途中で repository / filesystem が変更された可能性があるか」をユーザーへ説明できる必要がある。

## 決定

Rust 側に共通の構造化エラー型 `AppError` と、安定した `ErrorCode` enum を定義する。

概念上の共通フィールドは次のとおりとする。

```text
AppError
- code: ErrorCode
- message: String
- technicalDetail: Option<String>
- operation: Option<String>
- mayHaveMutated: bool
```

Rust の `ErrorCode` は Frontend へ `SCREAMING_SNAKE_CASE` の安定コードとして serialize する。

例:

```text
ENV_GIT_NOT_FOUND
ENV_GIT_VERSION_FAILED
PROJECT_NOT_FOUND
PROJECT_PERMISSION_DENIED
PROJECT_INVALID_UNITY
PROJECT_UNSUPPORTED_KIND
GIT_REPOSITORY_NOT_FOUND
GIT_COMMAND_FAILED
SETTINGS_READ_FAILED
SETTINGS_WRITE_FAILED
SETTINGS_INVALID_JSON
SETTINGS_UNSUPPORTED_SCHEMA
BACKUP_INVALID_FORMAT
BACKUP_UNSUPPORTED_VERSION
RESTORE_PRECHECK_FAILED
RESTORE_SNAPSHOT_FAILED
RESTORE_FAILED
FILESYSTEM_READ_FAILED
FILESYSTEM_WRITE_FAILED
PERMISSION_DENIED
INTERNAL_ERROR
```

実装時には必要なコードのみ追加し、利用されないコードを先回りして大量定義しない。

## 命名規則

原則として次の形式を使う。

```text
<DOMAIN>_<CAUSE>
```

例:

- `ENV_GIT_NOT_FOUND`
- `PROJECT_INVALID_UNITY`
- `SETTINGS_UNSUPPORTED_SCHEMA`
- `RESTORE_SNAPSHOT_FAILED`

広すぎる `*_COMMAND_FAILED` や `INTERNAL_ERROR` は、より具体的なコードへ分類できない場合の fallback として扱う。

## 診断結果とエラーの分離

「未導入」「対象外」「問題なしだが機能が利用できない」といった状態は、可能な限り通常の診断結果として表現し、例外にしない。

例:

- Git が見つからない → 環境診断 command の通常レスポンス内で `NotInstalled` として返せる場合は診断状態として返す

これにより、Frontend は「ユーザーが次に行動できる診断状態」と「処理が成立しなかったエラー」を区別できる。

## Frontend の責務

Frontend は `error.code` を使って UI 分岐する。

`message` はユーザーへ表示可能な概要として扱う。

`technicalDetail` は必要な場合だけ詳細表示や診断ログへ利用し、通常 UI の主要メッセージにはしない。

Frontend は Git stderr、OS error message、外部ツールの文言を解析して条件分岐しない。

## Git / OS エラーの扱い

Git stderr や OS error は Rust 側で解釈し、可能な範囲で Vsedi の安定した `ErrorCode` へ変換する。

例:

```text
fatal: not a git repository
```

を Frontend が文字列比較するのではなく、Rust 側で `GIT_REPOSITORY_NOT_FOUND` に変換する。

生の stderr / OS error は診断に必要な場合だけ `technicalDetail` や sanitized log に保持する。

## mutation 状態

`mayHaveMutated` は安全性上の重要フィールドとする。

- `false`: 失敗前に永続的な変更が発生していないと判断できる
- `true`: 一部の filesystem / repository mutation が発生した可能性がある

mutation を伴う処理では、失敗時にこの値を保守的に設定する。

Frontend は `mayHaveMutated = true` の場合、単純な「失敗しました」だけで終わらせず、現在状態の確認や復旧導線を表示できるようにする。

## 型共有

`AppError` と Frontend へ公開する `ErrorCode` は ADR 0008 に従い、Rust を正本として `serde + ts-rs` から TypeScript 型を生成する。

Frontend 側で同じ error code union を手作業で二重定義しない。

## 影響

良い点:

- Frontend が Git / OS の実装詳細から分離される
- Windows / macOS 間で一貫したエラー UX を作りやすい
- エラーコードをログ、テスト、サポート情報に利用できる
- mutation の可能性をユーザーへ明確に伝えられる
- Rust / TypeScript の型共有と自然に統合できる

注意点:

- 外部ツールの失敗をどの `ErrorCode` に変換するかを継続的に整備する必要がある
- `technicalDetail` へ秘密情報が入らないよう redaction が必要
- エラーコードの意味は一度公開後に安易に変更しない

## 再検討する条件

- error chain / source 情報を機械可読で Frontend へ公開する必要が生じた場合
- telemetry / crash reporting を導入し、追加の correlation ID 等が必要になった場合
- localization のため message を Rust ではなく Frontend で完全生成する方針へ変更する場合
