# 別マシン向け現状引き継ぎ

更新日: 2026-08-18

## 現在地

- 対象ブランチ: `codex/slint-port`
- M3（ローカル保存・履歴確認）は実装・検証完了
- 保存履歴は20件単位で読み込み、古い履歴を「さらに読み込む」で追加取得できる
- 履歴・保存詳細・設定などの長い画面は、サイドバーを固定したメイン領域でスクロールできる
- Windows / Apple Silicon macOS の配布物確認まで完了
- 現行の比較基準はTauri / React版M3。Slint移植は、既存のカード型ホーム画面、M3のworktree確認・保存・履歴接続まで実装済み
- 移植中はReact/Tauri前提の自動Actionsを停止している
- M4（安全な復元）の旧計画は破棄し、移植方針が固まるまで保留

この文書は、別マシンで現在の状態を再現するための入口です。作業開始時は、必ずリモートのブランチ先端を取得してください。

## 別マシンでの開始手順

```sh
git fetch origin
git switch codex/slint-port
git pull --ff-only origin codex/slint-port
mise install
pnpm install --frozen-lockfile
```

`.mise.toml` が開発環境のバージョンを管理します。

- Node.js: 22.23.1
- Rust: 1.97.1（MSVC、rustfmt / clippy付き）
- pnpm: Corepack経由

## 完了済みの範囲

- Unity / VRChat project診断
- Git repository初期化 preview・適用
- worktree変更、file diff、保存メモ付きlocal commit
- 保存履歴、commit詳細、変更file、diff表示
- 保存履歴の20件単位ページング、古い履歴の追加読み込み、長い画面のスクロール
- Windows Gitの履歴出力解析修正（root commit・末尾改行を含む）
- 管理Project、タグ、Project検索、stale path管理
- 全体設定、repository固有設定、ignore template
- Windows配布物の手動GUI smoke
- macOS DMG / app と Windows MSI / NSIS の配布ビルド
- Windows表示用パスの整形（`\\?\`除去、`/`表示）
- Slint 1.17.1の最小native windowとRust application facade
- Slintからの環境/project診断、worktree確認、作業保存、保存履歴
- Slintのカード型ホーム（ヒーロー、状態カード、Project診断、作業・履歴カード、移植進捗）
- worktree変更の概要と最大5件のファイル一覧表示、native folder picker
- Slint testing backendによる主要操作コントロールの検出テスト

## 検証コマンド

```sh
CI=true pnpm typecheck
CI=true pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo check --manifest-path src-tauri/Cargo.toml --bin slint
cargo test --manifest-path src-tauri/Cargo.toml --bin slint
cargo build --manifest-path src-tauri/Cargo.toml --bin slint --release
CI=true pnpm check-generated-types
pnpm exec tauri build
```

Slint版の確認には次を使います。

```sh
pnpm slint:dev
pnpm slint:test
```

`pnpm slint:dev`ではproject pathを入力し、「Projectを診断」「変更を確認」「作業を保存」「履歴を読み込む」を確認できます。保存を試す場合は、既存Git repositoryの一時的な作業用projectを使い、保存前にworktreeのstatus tokenを取得してください。

配布物は次に生成されます。

- `src-tauri/target/release/bundle/nsis/Vsedi_0.1.0_x64-setup.exe`
- `src-tauri/target/release/bundle/msi/Vsedi_0.1.0_x64_en-US.msi`

## 既知の注意点

- アプリの設定・最近のProject一覧は各マシンのアプリデータに保存され、Git管理対象ではありません。別マシンではProjectを再登録してください。
- リモート操作、過去状態への復元、履歴書換えは未実装です。安全な復元はSlint移植後に再計画します。
- 現環境ではRustテスト49件、Slint UIテスト1件がすべて成功しています。過去に`HOME`未設定環境ではログテスト1件が失敗したため、別マシンで再現した場合は`HOME`を設定してから再実行してください。
- Slint画面は現在、全体/repository設定、file diff詳細、repository初期化preview、履歴詳細をまだ持ちません。これらはユーザーテスト後の次の移植単位です。
- 配布物は未署名です。

## 次の開発候補

1. ユーザーによるSlint native UIの実project確認
2. 設定、file diff、repository初期化preview、履歴詳細の移植
3. Windows / Apple Silicon macOS native起動・bundle確認
4. Slint用Actionsへ置換して自動実行を復帰
5. 移植完了後のM4安全な復元再計画
