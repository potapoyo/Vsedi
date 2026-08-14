# アーキテクチャ

## 概要

Vsedi は Windows / macOS 向けの Tauri v2 デスクトップアプリケーションである。

Web フロントエンドは状態表示とユーザー操作の受付を担当し、ファイルシステムへのアクセス、Git プロセスの実行、プロジェクト検証などの権限を伴う処理は Rust 側が担当する。

フロントエンド技術構成は ADR 0006 に従い、React + TypeScript + Vite + pnpm + Tailwind CSS + shadcn/ui を採用する。

Rust / TypeScript 間の共有データ型は ADR 0008 に従い、Rust を正本として `serde + ts-rs` から TypeScript 型を生成する。

メインウィンドウの画面遷移、repository workspace、全体設定とrepository設定の境界は [`application-navigation-and-settings.md`](application-navigation-and-settings.md) に従う。

M1 の実装は `src-tauri/src` に次の境界を持つ。`commands` は Tauri の薄い入口、`services` はユースケース、`git` / `platform` / `settings` は具体的な外部状態との接続を担当する。

```text
src-tauri/src/
├─ commands/       # inspect_environment / inspect_project / settings / logging
├─ services/       # diagnostics / projects
├─ git/            # 固定された Git 診断と repository 判定
├─ platform/       # app data / log path、PATH executable 探索、OS folder open
├─ models/         # Rust 正本の共有 DTO
├─ errors/         # AppError / ErrorCode
├─ logging/        # 日次ログ、redaction、診断ログ export
└─ settings/       # settings.json の検証、migration、退避、Tauri Store 保存
```

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
- worktree statusを手動で再取得し、変更ファイルを階層表示する
- 変更一覧とrepositoryのファイルツリーを切り替えて表示する。全体表示はGit管理対象と無視されていない未管理ファイルをRust側から取得する
- 保存履歴を細い列表示にし、選択したcommitのファイルを現在の変更と同じ階層ツリーで表示する
- 変更・保存詳細のツリー列はマウス操作で幅を調整し、必要に応じて横スクロールできる
- 現在の作業のサマリーは既定で折りたたみ、project診断や保存停止などの異常状態を検知したカードだけ自動展開する
- repository画面では共通ページヘッダーを省略して作業領域を上端へ配置し、サイドバーの選択中Projectと関連メニューを枠でまとめる
- 保存状態を未保存・保存済み・保存停止中に分類し、未保存や停止中は注意を引く色で表示する
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

M1 で実装する command contract は次のとおりである。

| Command | 入力 | 戻り値 | 用途 |
| --- | --- | --- | --- |
| `inspect_environment` | なし | `EnvironmentDiagnostic` | OS / architecture / Git 診断 |
| `inspect_project` | `path: string` | `ProjectDiagnostic` | Unity project と repository の診断。VPM方針は設定から解決 |
| `load_settings` | なし | `SettingsLoadResult` | 設定読込と stale path の状態表示 |
| `save_settings` | `AppSettings` | `void` | 許可された内部設定の保存 |
| `export_diagnostic_log` | `destination: string` | `void` | redaction 済み診断ログの書出し |
| `open_log_directory` | なし | `void` | OS 標準ファイルマネージャーでログ領域を開く |

Frontend に `run_shell` / `run_git` のような汎用 command は公開しない。folder picker は Tauri dialog plugin の directory picker を使い、選択後の検証は Rust の `inspect_project` が行う。

## Application Service

### ProjectService

登録済みプロジェクトの識別、path の正規化、プロジェクトのライフサイクルを担当する。

### DiagnosticsService

Git、Unity、VRChat/VPM、ignore設定の診断結果を統合し、ユーザー向けの診断情報へ変換する。

M2 の `inspect_project` は読み取り専用で、次の情報を1つの `ProjectDiagnostic` に統合する。

- `Assets` と `ProjectSettings/ProjectVersion.txt` による Unity project validation
- Unity editor version / revision
- `Packages/manifest.json` と `Packages/vpm-manifest.json` の package metadata
- `com.vrchat.avatars` / `com.vrchat.worlds` に基づく Avatar / World 判定
- Git repository root と選択 project root の境界一致
- root `.gitignore` とVPM用 `Packages/.gitignore` の状態
- 設定で選択したVPM package追跡方針との一致

Avatar / World 判定は package ID を根拠にする。両方が存在する場合は非対応エラーとして診断を停止する。VRChat package はあるが種別 package がない場合は推測で正常扱いせず、要確認とする。ProjectVersion、manifest、ignore設定などの読み取り不能は警告として記録し、読み取れる範囲の診断を継続する。

Git repository rootがUnity project外にある構成は関連ファイルを同じrepositoryで管理できる正常な構成として、`INFO`の診断理由を返す。

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
- 保存処理では stdout / stderr を逐次読み取り、構造化した Tauri event としてUIへ転送する
- parser は fixture を用いたテストを行う
- secret をログへコピーしない

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
- 管理対象の project path、最終更新日時、アプリ内タグ
- secret ではない UI 設定
- window state

初期実装では Tauri Store を使用し、OS 標準のアプリデータ領域へ `settings.json` として保存する。

`settings.json` は整数の `schemaVersion` を持ち、初期値は `1` とする。Explorer / Finder から通常のファイルとしてコピーでき、対応するファイルを所定の場所へ手動配置した場合も Vsedi が読み込めることを非常時の低レベル復旧手段として保証する。

### 持ち運び可能な環境バックアップ

PC 初期化・買い替え後の再構成に利用できる、バージョン付き JSON をユーザーが明示的に export / import できるようにする。

- ファイル名は `vsedi-environment.vsedi.json` とする
- `formatVersion` を必須とし、初期値は `1` とする
- 絶対 path は正本にしない
- remote URL、Unity version、branch 等の復元に必要な非秘密情報を保持する
- password / token / SSH private key 等の秘密情報は含めない

リポジトリの正しい状態は Git / project files に置き、アプリケーション側の状態を authoritative な情報として二重管理しない。

`settings.json` は整数の `schemaVersion` を必須とし、OS 標準の app data directory に Tauri Store で保存する。読込前に JSON と schema を検証し、破損 JSON や migration 対象は元ファイルを `.bak.<timestamp>` として退避する。未対応の新しい schema はエラーを返し、元ファイルを変更しない。管理対象の project path は存在しなくても設定から削除せず、`SettingsLoadResult.recentProjects[].exists` で再指定が必要な状態を表現する。schema 4の単一カテゴリはschema 6で複数タグへ移行し、repositoryごとのVPM追跡方針overrideもschema 6で保持する。schema 7では既存のignore templateを保持したままmacOS / WindowsのOS生成ファイルruleを不足分だけ追加する。repository設定はapp data側だけへ保存し、repository内へ設定fileを書き込まない。

## Rust / TypeScript 型境界

ADR 0008 に従い、Rust の型定義を正本とする。

- `serde` で実際の serialization 形式を定義する
- `ts-rs` で Frontend 用 TypeScript 型を生成する
- Frontend 側で同じ response / request model を手作業で二重定義しない
- backup manifest の公開型は `formatVersion` と合わせて互換性を管理する
- Rust 内部型と公開 DTO は必要に応じて分離する

## エラーモデル

ADR 0009 に従い、Rust 側に共通の `AppError` と安定した `ErrorCode` enum を持つ。

`AppError` は少なくとも次を含む。

- `code`: Frontend の分岐に使う安定した application error code
- `message`: ユーザーへ安全に表示できる概要
- `technicalDetail`: 必要に応じた技術的詳細
- `operation`: どの処理段階で失敗したか
- `mayHaveMutated`: repository / filesystem の変更が発生した可能性があるか

`ErrorCode` は `<DOMAIN>_<CAUSE>` を基本とする `SCREAMING_SNAKE_CASE` で serialize する。Frontend は Git stderr や OS error message を文字列解析して条件分岐しない。

Git stderr / OS error は Rust 側で可能な範囲で安定コードへ変換し、生の詳細は必要な場合だけ sanitized な `technicalDetail` / log に保持する。

Git未導入など、ユーザーが次の行動を取れる「正常な未検出」はエラーではなく診断結果として返す。実行そのものが予期せず失敗した場合だけ構造化エラーにする。

`AppError` / `ErrorCode` は ADR 0008 の方針に従い `serde + ts-rs` から TypeScript 型を生成する。

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

M1 の capability は `core:default`、native directory picker 用の `dialog:default`、内部設定用の `store:default` に限定している。Frontend は任意の filesystem API や shell API を持たず、ログフォルダを開く処理も Rust 側でアプリ自身の log directory に固定する。

## M1 実装の検証境界

Rust の unit test と `ts-rs` 型再生成は `src-tauri` の Cargo toolchain が必要である。Windows と Apple Silicon macOS の native 起動・Tauri bundle は各 OS の Rust / Tauri prerequisites が揃った環境で実行する。リポジトリには `pnpm generate-types` と `pnpm check-generated-types` を用意し、生成型は Git 管理する。

## 現時点で未確定の設計判断

次の項目は各 milestone で具体化する。

- restore / safety snapshot の具体的な方式
- GitHub 固有 OAuth
- updater infrastructure
- `ts-rs` 生成物を Git に含めるか、build/test 時に生成するか

フロントエンド技術構成、設定/環境バックアップ方針、Rust / TypeScript 型共有方針、アプリケーションエラーモデルは ADR 0006〜0009 で確定済み。

## 参考資料

- Tauri v2 Shell: https://v2.tauri.app/plugin/shell/
- Git credentials: https://git-scm.com/docs/gitcredentials.html
- VRChat VPM source control: https://vcc.docs.vrchat.com/vpm/source-control/
