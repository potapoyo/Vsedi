# Vsedi

Vsedi は、VRChat 向け Unity プロジェクトの作業を安全に保存・確認・復元するためのデスクトップアプリです。

内部では Git を使用しますが、Git に詳しくない VRChat 制作者でも「作業を保存」「保存履歴を見る」「過去の状態に戻す」「リモートへバックアップする」といった操作を扱えることを目指します。

さらに、PC の故障・初期化・買い替え後に、Unity Editor、VCC または ALCOM、Git 等の必要環境、Vsedi の持ち運び可能な環境バックアップ、アクセス可能なリモートリポジトリを使って制作環境を再構成できる復元モードを目標とします。

## 方針

- Windows / Apple Silicon macOS 対応
- Tauri v2 を利用
- React + TypeScript + Vite + pnpm + Tailwind CSS + shadcn/ui を採用
- Rust / TypeScript 間の共有型は `serde + ts-rs` で Rust を正本として管理
- ローカルファースト（Local First）: GitHub 等のリモートがなくても価値が成立する
- 安全性を機能性より優先（Safety over Power）: 高機能な Git GUI より事故防止を優先する
- Unity / VRChat / VPM の構成を理解して安全性を高める
- アプリ内部設定と持ち運び可能な環境バックアップを分離する
- 環境バックアップへ password / token / SSH private key 等の秘密情報を含めない
- Windows / macOS 向けインストーラーバイナリの配布を必須とし、初期配布は未署名とする
- 仕様・設計思想・重要な判断は `docs/` に Markdown で残す

## 現在の段階

Tauri v2 / React の基盤、M2のproject診断、M3のローカル保存（repository初期化preview、変更確認、作業保存、履歴・commit詳細、表示可能なfile diff）に加え、管理Project一覧、複数タグ、Project検索、repository単位の設定と作業画面を実装中です。リモート操作・復元・履歴書換えはまだ提供しません。実装ロードマップは [`docs/development/roadmap.md`](docs/development/roadmap.md)、詳細計画は [`docs/development/m3-plan.md`](docs/development/m3-plan.md) を参照してください。

### `.gitignore` 初期ルールのカスタマイズ

Vsedi が新規 repository を初期化するときに提案する `.gitignore` ルールは、全体設定の `ignore template` から編集でき、アプリデータフォルダ内の `settings.json` の `ignoreTemplates` で管理します。`unityRules` は project root の `.gitignore`、`vpmExcludeRules` は VPM package を除外する設定時の `Packages/.gitignore` に対する候補です。

既存repositoryの「リポジトリ設定」では、現在のignoreとの差分を確認してから不足ruleだけを明示的に追加できます。既存 `.gitignore` は置換・削除されず、改行形式も保持します。設定変更だけではrepository内のファイルを変更しません。

既定の `unityRules` は、GitHub の [Unity テンプレート](https://github.com/github/gitignore/blob/main/Unity.gitignore)へ、Vsedi利用者向けの `Library/metadata` と `Library/assetDatabase3` の保持ルールを統合したものです。

### ログレベルとログ表示

メイン画面の「ログ設定」から `ERROR` / `WARN` / `INFO` / `DEBUG` / `TRACE` を選択できます。変更は即時適用され、`settings.json` の `logLevel` に保存されます。ログ表示ウィンドウは、30日保持の対象になっているサニタイズ済みログを現在はすべて表示します。

## ドキュメント

- [`docs/product/vision.md`](docs/product/vision.md) — 製品ビジョンと対象ユーザー
- [`docs/design/principles.md`](docs/design/principles.md) — 設計原則
- [`docs/design/safety-model.md`](docs/design/safety-model.md) — 復元・破壊的操作の安全モデル
- [`docs/design/architecture.md`](docs/design/architecture.md) — アーキテクチャと権限境界
- [`docs/development/requirements.md`](docs/development/requirements.md) — MVP / 製品要件
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — 開発フェーズ
- [`docs/adr/`](docs/adr/) — アーキテクチャ判断記録（ADR）

## 開発コマンド

```sh
pnpm install
pnpm tauri dev
pnpm tauri build
pnpm generate-types
pnpm check-generated-types
```

`pnpm tauri dev` / `pnpm tauri build` は Rust、Tauri の OS 依存ライブラリ、対象 OS の native toolchain が必要です。公式対応環境は Windows と Apple Silicon macOS です。

### Windows の開発環境

Windows では、次のネイティブ依存を先に用意します。

1. Visual Studio Installer で **Desktop development with C++** workload と Windows SDK をインストールする。
2. Microsoft Edge WebView2 Runtime がインストール済みであることを確認する。Windows 10 version 1803 以降と Windows 11 では通常インストール済みですが、無い場合は Evergreen Bootstrapper をインストールする。
3. MSI installer も生成する場合は、Windows の Optional features で **VBSCRIPT** を有効にする。NSIS のみを生成する場合は不要です。

PowerShell 7 と mise は `winget` で導入できます。PowerShell 7 を起動して、初回だけ次を実行します。

```powershell
winget install --id Microsoft.PowerShell --source winget
winget install jdx.mise
```

PowerShell 7 のプロファイル（`$PROFILE`）へ次の行を一度追加し、mise を自動有効化します。

```powershell
(&mise activate pwsh) | Out-String | Invoke-Expression
```

PowerShell 7 を再起動し、Vsedi のリポジトリでプロジェクト固定のツールチェーンと依存関係を導入します。

```powershell
mise trust
mise install
corepack enable
pnpm install --frozen-lockfile
```

`.mise.toml` により、Node.js `22.23.1`、Rust `1.97.1`、`rustfmt`、`clippy` がプロジェクト単位で選択されます。導入後は次で確認できます。

```powershell
node --version
rustc --version
cargo --version
pnpm --version
```

#### Windows の GUI スモークテスト

```powershell
pnpm tauri dev
```

1. 「Vsedi」というネイティブウィンドウが開くことを確認する。
2. 「実行環境」が対応対象になり、Windows の OS / architecture が表示されることを確認する。
3. 「System Git」に Git のバージョンが表示されることを確認する。
4. 「再診断」を押し、赤いエラー表示が出ないことを確認する。
5. 「フォルダを選択」から Unity project のルートを選び、Unity project と Unity バージョンが表示されることを確認する。
6. 終了するときは、ターミナルで `Ctrl+C` を押す。

#### Windows の native build

```powershell
pnpm tauri build
```

installer の生成結果は `src-tauri/target/release/bundle/` 配下で確認します。MSI の生成で `light.exe` などのエラーが出る場合は、上記の VBSCRIPT を有効にしてから再実行してください。

参考: [Tauri の Windows 前提条件](https://v2.tauri.app/ja/start/prerequisites/)、[mise の Windows インストール](https://mise.jdx.dev/installing-mise.html)。

### macOS の開発環境

リポジトリには `mise` 用の `.mise.toml` を含めています。Node.js と Rust はリポジトリ単位で固定されるため、別のバージョンを試すときも他のプロジェクトへ影響しません。初回だけ次を実行します。

```sh
brew install mise
mise trust
mise install
pnpm install --frozen-lockfile
```

現在の固定値は Node.js `22.23.1`、Rust `1.97.1` です。変更するときは、プロジェクトのディレクトリで次のように実行して `.mise.toml` を更新します。

```sh
mise use --pin node@22.23.1
mise use --pin rust@1.97.1
```

macOS のデスクトップGUIを起動して確認する場合は、次を実行します。

```sh
pnpm tauri dev
```

#### macOS の DMG build

macOSではDMGのFinder装飾にApple Events権限が必要になる場合があります。配布用DMGを権限なしで再現可能に生成するには、Finder装飾を省略して次を実行します。

```sh
pnpm tauri:build:macos
```

生成されたDMGは `src-tauri/target/release/bundle/dmg/` で確認します。

#### 人が行うGUI確認

これは自動E2Eではなく、アプリがネイティブウィンドウとして起動し、主要な診断操作ができることを確認するための目視スモークテストです。

1. Vsedi のリポジトリで `pnpm tauri dev` を実行する。
2. 「Vsedi」というウィンドウが開くことを確認する。
3. 画面上部の「実行環境」が「対応対象」になり、Apple Silicon Mac では `macOS / arm64` と表示されることを確認する。
4. 「System Git」が「利用可能」になり、Git のバージョンが表示されることを確認する。
5. 「再診断」を押し、赤いエラー表示が出ないことを確認する。
6. 「フォルダを選択」を押して Unity project のルートフォルダを選ぶ。`Assets` と `ProjectSettings/ProjectVersion.txt` があるフォルダなら、Unity version と診断状態が表示される。
7. VRChat project では Avatar / World 種別と検出 package が表示されることを確認する。
8. `.gitignore`、VPM package rule、Git repository rootの診断理由が表示されることを確認する。
9. 「VPM packageのGit管理」を「除外する」「含める」で切り替え、選択した方針に応じて診断が更新されることを確認する。
10. 終了するときは、ターミナルで `Ctrl-C` を押す。

Unity project が手元にない場合は、手順 1〜5 までを実施すれば、ネイティブGUIと環境診断の確認ができます。

## 配布方針

- Windows: NSIS `.exe` または MSI `.msi` を少なくとも1種類提供
- macOS: Apple Silicon（arm64）向け `.dmg` を提供
- Intel Mac は正式対応対象外
- 初期配布は未署名とし、OS の警告や必要な起動手順を明確に案内する

## ライセンス

GNU General Public License v3.0。詳細は [`LICENSE`](LICENSE) を参照してください。
