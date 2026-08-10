# アーキテクチャ

## 概要

Vsedi は Windows / macOS 向けの Tauri v2 デスクトップアプリケーションである。

Web フロントエンドは状態表示とユーザー操作の受付を担当し、ファイルシステムへのアクセス、Git プロセスの実行、プロジェクト検証などの権限を伴う処理は Rust 側が担当する。

フロントエンド技術構成は ADR 0006 に従い、React + TypeScript + Vite + pnpm + Tailwind CSS + shadcn/ui を採用する。

Rust / TypeScript 間の共有データ型は ADR 0008 に従い、Rust を正本として `serde + ts-rs` から TypeScript 型を生成する。

```text
Frontend UI
    |
    | 型付けされた Tauri command / event
    v
Tauri Command Boundary
    |
    v
Application Services
    |-- ProjectService
    |-- DiagnosticsService
    |-- SaveService
    |-- HistoryService
    |-- RestoreService
    |-- BackupService
    `-- SyncService          (Vsedi Core 完成後)
    |
    v
Domain / Adapters
    |-- GitAdapter
    |-- UnityProjectAnalyzer
    |-- VrchatProjectAnalyzer
    |-- LfsAnalyzer
    |-- FileSafety
    |-- SettingsStore
    `-- PlatformAdapter
```

## フロントエンドの責務

フロントエンドが行ってよいこと:

- プロジェクト一覧、診断結果、変更、履歴、プレビューを表示する
- ユーザーの操作意図と確認を受け取る
- あらかじめ定義されたアプリケーション操作を要求する
- Rust から返された構造化済みの進捗・エラーを表示する

フロントエンドが行ってはいけないこと:

- 任意の shell command を実行する
- 実行用の生の Git command を組み立てる
- プロジェクト内の任意ファイルを直接変更する
- Git の password / token をアプリ設定として受け取ったり永続化したりする

## Rust コマンド境界

Tauri command は Git の構文ではなく、アプリケーション上の意図を表現する。

推奨する command の例:

- `inspect_environment()`
- `inspect_project(path)`
- `initialize_project(path, plan)`
- `get_worktree_status(project_id)`
- `save_work(project_id, message)`
- `get_history(project_id, cursor)`
- `get_revision_detail(project_id, revision)`
- `preview_restore(project_id, revision)`
- `restore_revision(project_id, revision, confirmation)`
- `export_environment_backup(destination)`
- `inspect_environment_backup(path)`
- `restore_environment(path, destination_plan)`

次のような汎用 command は避ける:

- `run_git(args)`
- `run_shell(command)`

汎用的な抜け道を用意すると、セキュリティ境界がフロントエンド側へ移動し、service layer を設ける意味が失われる。

## Application Service

### ProjectService

登録済みプロジェクトの識別、path の正規化、プロジェクトのライフサイクルを担当する。

### DiagnosticsService

Git、Unity、VRChat/VPM、LFS、ignore 設定、大容量ファイルの診断結果を統合し、ユーザー向けの診断情報へ変換する。

Git LFS は独立した executable を探索せず、Vsedi が利用する Git に対して `git lfs version` を実行して利用可否と version を診断する。これにより、実際の Git 実行環境から見える LFS 状態を診断結果の正本とする。

### SaveService

製品上の「作業を保存」という操作を、検証済みの Git index / commit 操作へ変換する。

### HistoryService

リポジトリを変更せずに commit history と revision の詳細を読み取る。

### RestoreService

復元プレビュー、安全スナップショット作成、復元実行、検証、復旧用メタデータを担当する。

### BackupService

ADR 0007 に従い、持ち運び可能な環境バックアップの export / import、formatVersion 検証、migration、復元計画の作成を担当する。

秘密情報は環境バックアップへ含めない。

### SyncService

Vsedi Core 完成後に追加する。fetch / push / fast-forward 同期と履歴分岐検出を担当する。ADR 0003 に従い、履歴が分岐している場合は停止する。

## GitAdapter

初期実装では ADR 0001 に従い、システムにインストールされた Git CLI を利用する。

要件:

- executable と arguments を分離して渡す
- working directory を明示する
- 機械可読形式が利用できる場合、人間向けで locale に依存する出力を避ける
- exit code、stdout、stderr を分離して取得する
- parser は fixture を用いたテストを行う
- secret をログへコピーしない
- Git LFS の診断は検出済み Git に `lfs version` を引数として渡して行い、`git-lfs` を別 executable として探索しない

実装時には、適切な場所で NUL 区切りの status format など、安定した機械可読出力を持つ Git の plumbing / porcelain format を選択する。

## プロジェクト識別と path

プロジェクトに対する操作は、登録済みかつ正規化された project root を起点とする。

状態変更前には次を行う。

1. 登録済み project root を解決する
2. OS ごとの規則に従って対象 path を canonicalize / normalize する
3. 操作対象が意図した repository の範囲内であることを確認する
4. 明示的に対応していない repository / worktree 境界を検出した場合は拒否する

symlink、junction、nested repository、worktree は、対応済みとして扱う前にテストを用意する。

## アプリケーション状態と環境バックアップ

設定は ADR 0007 に従い、用途を分離する。

### アプリ内部設定

現在の PC 上でのみ必要な Vsedi 固有情報をローカルファイルへ保存する。

例:

- onboarding 完了状態
- 最近利用した project path
- secret ではない UI 設定
- window state

初期実装では Tauri Store を基本候補とする。

### 持ち運び可能な環境バックアップ

PC 初期化・買い替え後の再構成に利用できる、バージョン付き JSON をユーザーが明示的に export / import できるようにする。

- `formatVersion` を必須とする
- 絶対 path は正本にしない
- remote URL、Unity version、branch 等の復元に必要な非秘密情報を保持する
- password / token / SSH private key 等の秘密情報は含めない

リポジトリの正しい状態は Git / project files に置き、アプリケーション側の状態を authoritative な情報として二重管理しない。

## Rust / TypeScript 型境界

ADR 0008 に従い、Rust の型定義を正本とする。

- `serde` で実際の serialization 形式を定義する
- `ts-rs` で Frontend 用 TypeScript 型を生成する
- Frontend 側で同じ response / request model を手作業で二重定義しない
- backup manifest の公開型は `formatVersion` と合わせて互換性を管理する
- Rust 内部型と公開 DTO は必要に応じて分離する

## エラーモデル

Rust 側の操作は、少なくとも次を含む構造化エラーを返す。

- 安定した application error code
- ユーザーへ安全に表示できる概要
- 必要に応じた技術的詳細
- どの処理段階で失敗したか
- repository の変更が発生した可能性があるか

生の stderr は診断に有用な場合があるが、そのままユーザー向けメッセージとして扱わず、共有用ログへ出力する場合も redact の確認を行う。

## OS 固有処理の境界

特に次の Windows / macOS 固有処理は adapter の背後へ分離する。

- Unity の検出・起動
- Finder / Explorer を開く
- executable の探索
- process の検査
- path の扱い

Git の中心ロジックと domain logic は可能な限り OS 非依存にする。

## Tauri capabilities

Tauri の permission は必要最小限にする。JavaScript に広範な shell 実行能力を与えるより、Rust 側で process execution を管理する。

一般的な process / filesystem access を許可する capability の追加は、明示的なセキュリティレビュー対象とする。

## 現時点で未確定の設計判断

次の項目は各 milestone で具体化する。

- restore / safety snapshot の具体的な方式
- GitHub 固有 OAuth
- updater infrastructure
- Tauri Store の具体的な schema / file naming
- `ts-rs` 生成物を Git に含めるか、build/test 時に生成するか

フロントエンド技術構成、設定/環境バックアップ方針、Rust / TypeScript 型共有方針は ADR 0006〜0008 で確定済み。

## 参考資料

- Tauri v2 Shell: https://v2.tauri.app/plugin/shell/
- Git credentials: https://git-scm.com/docs/gitcredentials.html
- VRChat VPM source control: https://vcc.docs.vrchat.com/vpm/source-control/
