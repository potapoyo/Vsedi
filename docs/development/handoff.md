# 別マシン向け現状引き継ぎ

更新日: 2026-08-13

## 現在地

- 対象ブランチ: `codex/m3-local-save`
- M3（ローカル保存・履歴確認）は実装・検証完了
- Windows / Apple Silicon macOS の配布物確認まで完了
- 次の開発対象は M4（安全な復元）

この文書は、別マシンで現在の状態を再現するための入口です。作業開始時は、必ずリモートのブランチ先端を取得してください。

## 別マシンでの開始手順

```sh
git fetch origin
git switch codex/m3-local-save
git pull --ff-only origin codex/m3-local-save
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
- Windows Gitの履歴出力解析修正（root commit・末尾改行を含む）
- 管理Project、タグ、Project検索、stale path管理
- 全体設定、repository固有設定、ignore template
- Windows配布物の手動GUI smoke
- macOS DMG / app と Windows MSI / NSIS の配布ビルド
- Windows表示用パスの整形（`\\?\`除去、`/`表示）

## 検証コマンド

```sh
pnpm typecheck
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml services::
pnpm exec tauri build
```

配布物は次に生成されます。

- `src-tauri/target/release/bundle/nsis/Vsedi_0.1.0_x64-setup.exe`
- `src-tauri/target/release/bundle/msi/Vsedi_0.1.0_x64_en-US.msi`

## 既知の注意点

- アプリの設定・最近のProject一覧は各マシンのアプリデータに保存され、Git管理対象ではありません。別マシンではProjectを再登録してください。
- リモート操作、過去状態への復元、履歴書換えは未実装です。これらはM4以降の対象です。
- 以前の全Rustテストでは、`HOME`未設定環境に依存するログテスト1件が失敗しました。`services::` の24テストと履歴関連4テストは成功しています。
- 配布物は未署名です。

## 次の開発候補

1. M4: 履歴からの安全な復元 preview
2. 復元前スナップショットと復元後の検証
3. Unity起動中の復元警告
4. M5: リモートバックアップ・安全な同期
