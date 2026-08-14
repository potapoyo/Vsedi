# 別マシン向け現状引き継ぎ

更新日: 2026-08-14

## 現在地

- 対象ブランチ: `codex/m3-local-save`
- M3（ローカル保存・履歴確認）は実装・検証完了
- 保存履歴は20件単位で読み込み、古い履歴を「さらに読み込む」で追加取得できる
- 履歴・保存詳細・設定などの長い画面は、サイドバーを固定したメイン領域でスクロールできる
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
- 保存履歴の20件単位ページング、古い履歴の追加読み込み、長い画面のスクロール
- Windows Gitの履歴出力解析修正（root commit・末尾改行を含む）
- 管理Project、タグ、Project検索、stale path管理
- 全体設定、repository固有設定、ignore template
- Windows配布物の手動GUI smoke
- macOS DMG / app と Windows MSI / NSIS の配布ビルド
- Windows表示用パスの整形（`\\?\`除去、`/`表示）

## 検証コマンド

```sh
CI=true pnpm typecheck
CI=true pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
CI=true pnpm check-generated-types
pnpm exec tauri build
```

配布物は次に生成されます。

- `src-tauri/target/release/bundle/nsis/Vsedi_0.1.0_x64-setup.exe`
- `src-tauri/target/release/bundle/msi/Vsedi_0.1.0_x64_en-US.msi`

## 既知の注意点

- アプリの設定・最近のProject一覧は各マシンのアプリデータに保存され、Git管理対象ではありません。別マシンではProjectを再登録してください。
- リモート操作、過去状態への復元、履歴書換えは未実装です。これらはM4以降の対象です。
- 現環境ではRustテスト48件、Playwright UIテスト11件がすべて成功しています。過去に`HOME`未設定環境ではログテスト1件が失敗したため、別マシンで再現した場合は`HOME`を設定してから再実行してください。
- 配布物は未署名です。

## 次の開発候補

1. M4: 履歴からの安全な復元 preview
2. 復元前スナップショットと復元後の検証
3. Unity起動中の復元警告
4. M5: リモートバックアップ・安全な同期
