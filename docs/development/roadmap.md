# 開発ロードマップ

## M0 — 製品定義

目的: 実装前に製品の境界と安全原則を固定する。

成果物:

- 製品ビジョン
- 対象ユーザー / 用語
- 設計原則
- 安全モデル
- MVP 要件 / 対象外項目
- アーキテクチャ判断記録（ADR）

完了条件:

- 「Vsedi が何をするか」と「何をしないか」をリポジトリ内の Markdown だけで説明できる
- Git backend / local-first / automatic merge 方針が ADR で決まっている
- 正式対応 OS、Tauri v2、バイナリ配布方針が ADR で決まっている
- フロントエンド技術構成が ADR で決まっている
- 設定 / 環境バックアップ方針と Rust / TypeScript 型共有方針が ADR で決まっている

## M1 — Tauri 基盤（完了）

目的: Windows / Apple Silicon macOS で共通のアプリ基盤と Rust command boundary を作る。

採用済みフロントエンド構成:

- React
- TypeScript
- Vite
- pnpm
- Tailwind CSS
- shadcn/ui

タスク:

- Tauri v2 プロジェクト初期化
- React + TypeScript + Vite + pnpm の初期化
- Tailwind CSS / shadcn/ui の初期化
- `serde + ts-rs` による共有型生成基盤
- Rust ↔ frontend command boundary
- native folder picker
- Git executable detection
- Git version detection
- アプリ内部設定用ローカル store の初期基盤
- structured / sanitized logging
- Windows native build check
- Apple Silicon macOS native build check

完了条件:

- 両 OS でアプリが起動する
- 両 OS でネイティブビルドが成功する
- project folder を選べる
- Rust 側から Git の存在を安全に確認できる
- Rust の共有型から TypeScript 型を生成できる
- frontend から任意の Git / shell command を実行できない

実装済みの基盤:

- `src-tauri` に Tauri v2、Rust module 境界、共通 `AppError` / `ErrorCode` を追加
- React + TypeScript + Vite + pnpm、Tailwind CSS、shadcn/ui 形式の最小 UI を追加
- `inspect_environment` による system Git 診断を追加
- native directory picker と `inspect_project` による Unity / Git project 診断を追加
- Tauri Store を使う `settings.json` の schema 検証、migration 前退避、破損復旧、stale path 表示を追加
- 日次・30日保持の application logging、secret / remote URL redaction、診断ログ export を追加
- `ts-rs` 再生成コマンドと生成差分検証スクリプトを追加

未検証の環境依存項目:

- Windows native 起動 / bundle
- Apple Silicon macOS native 起動 / bundle

## M2 — Unity / VRChat プロジェクト検出

目的: Vsedi が管理対象 project を理解し、危険な状態を診断できる。

タスク:

- Unity project validation
- Unity version / project metadata reading
- VPM / VRChat project detection
- Avatar / World 判定の調査と可能な範囲での実装
- Avatar SDK と Worlds SDK が同居する project の拒否
- existing Git repository detection
- `.gitignore` diagnostics
- 選択可能なVPM package source-control diagnostics
- 読み取り不能な project 設定ファイルの警告

完了条件:

- 選択 project について「管理可能 / 要修正 / 非 Unity」を説明できる

## M3 — ローカル保存（完了）

目的: 最初に実用になる Vsedi を作る。

詳細な実装順序と安全条件は [`m3-plan.md`](m3-plan.md) を参照する。

タスク:

- Git repository initialization
- status parser
- changed-file model
- diff reading
- save memo UI
- add / commit
- history reading
- commit detail view

完了条件:

> Unity project を登録し「作業を保存」を押すと、その時点が履歴に残る。

この段階を Internal Alpha 候補とする。実装および Windows / Apple Silicon macOS 配布物の確認まで完了している。

## Slint移植 — 検討・試作

目的: 現在のTauri + React UIを、Rustサービス層を活かしながらSlintのネイティブUIへ移植できるか検証する。

方針:

- M3のTauri / React版を動作する基準として保持する。
- `src-tauri/src/services`、`git`、`models`、`errors`を優先的に再利用できる境界へ整理する。
- Home、保存履歴、ファイルツリー、スクロール、Rust command相当の非同期処理を小さなSlint試作で検証する。
- 全面移行の判断は、ビルド、Windows / Apple Silicon macOS、UIテスト、アクセシビリティを確認してから行う。

現在の進捗:

- `codex/slint-port` ブランチで自動Actionsを一時停止
- Rust側にUI非依存のapplication facadeを追加
- Slint 1.17.1の最小native windowを追加
- Slintから環境診断とproject診断を呼び出す経路を確認
- 既存Rustテスト49件が成功

未完了:

- M3画面全体の移植
- Windows / Apple Silicon macOSのSlint native起動・bundle確認
- Slint向けUI/presenterテスト
- ユーザーによる実Unity projectの受け入れ確認
- Slint用Actionsへの置換と自動実行の復帰

詳細な移植判断は [ADR 0015](../adr/0015-slint-native-ui-migration.md) を参照する。

## M4 — 保留（安全な復元）

安全な復元はSlint移植の方向性と基盤が固まった後に再計画する。旧M4計画ファイルは2026-08-18に破棄し、ADR 0014は将来の再検討用の参考記録として保留する。

## M5 — リモートバックアップ

目的: local-first の価値を保ったまま remote backup を追加する。

タスク:

- clone
- remote configuration
- fetch
- push
- fast-forward sync
- divergence detection
- Git credential helper integration
- authentication/error UX

明示的な制約:

- diverged history を自動 merge / rebase しない

## M6 — 環境バックアップと復元モード

目的: PC の故障・初期化・買い替え後でも、外部ツールとリモートリポジトリを利用して VRChat 制作環境を再構成できるようにする。

タスク:

- versioned `BackupManifest` 設計
- `formatVersion` migration framework
- environment backup export / import
- remote URL / branch / Unity version / VCC・ALCOM 参考情報の保存
- absolute path 非依存の復元設計
- Git / Unity / VCC / ALCOM の環境診断
- restore destination selection
- remote clone
- Unity / VPM compatibility diagnostics
- VCC / ALCOM から利用可能な状態かの診断
- restored project registration
- restore result / missing dependency report

明示的な制約:

- password / token / SSH private key / VRChat 認証情報を backup file に含めない
- 初期版では Unity / VCC / ALCOM の自動インストールを必須にしない

完了条件:

> 新しい PC に必要ツールを用意し、Vsedi の環境バックアップとアクセス可能なリモートリポジトリを渡すと、復元先を選択してプロジェクトを再構成し、不足項目を確認できる。

## M7 — VRChat 向け安全機能

目的: 一般 Git GUI ではなく VRChat 専用ツールとしての診断を強化する。

タスク:

- VPM ignore rules
- VRChat SDK / package accidental commit detection
- `.meta` consistency diagnostics
- SDK / package update checkpoint guidance
- Unity version update checkpoint guidance
- public repository / purchased asset warning design

## M8 — 初回チュートリアル

目的: Git を知らない新規ユーザーが最初の保存まで到達できる。

タスク:

- introduction
- project selection
- safety diagnostics walkthrough
- first save walkthrough
- local save vs remote backup explanation
- history walkthrough

完了条件:

- tutorial を通じて実際の最初の commit が作成される

## M9 — 公開ベータ品質と配布

目的: 必須要件である Windows / Apple Silicon macOS 向けバイナリ配布を、第三者が利用できる品質へ仕上げる。

タスク:

- Windows installer（NSIS `.exe` または MSI `.msi`）
- Apple Silicon macOS `.dmg`
- 各ネイティブ環境での release build
- GitHub Releases 等への成果物公開フロー
- 未署名配布時の Windows SmartScreen / macOS Gatekeeper 案内
- updater strategy
- CI builds
- automated tests
- crash/error reporting policy
- log export / redaction
- About / GPL notices
- release documentation

コード署名は初期リリースの必須条件にしない。ADR 0005 に従い、当面の公式配布物は未署名とする。

## 後回しにするもの

明示的に後回しにする。

- automatic merge / rebase
- rich branch management
- cherry-pick
- force push
- stash UI
- submodules / worktrees
- GitHub PR / Issue management
- UnityYAMLMerge automatic conflict resolution
- multi-user file locking
- trusted code signing / macOS notarization
- 外部ツール（Unity / VCC / ALCOM）の自動インストール
- credential を含む暗号化環境バックアップ

必要性が実利用から確認された時点で再評価する。
