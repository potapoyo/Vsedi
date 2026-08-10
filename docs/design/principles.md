# 設計原則

Vsedi の設計判断は、機能数よりも VRChat 制作者が安全に使えることを優先する。

## 1. 機能性より安全性を優先する（Safety over Power）

高機能な Git クライアントを目指さない。

- 危険な操作は初期版では提供しない
- 破壊的操作の前に必ず現在状態を保護する
- 自動解決より停止と説明を優先する

## 2. ローカルファースト（Local First）

リモートサービスを必須にしない。

- `git init`、commit、history、restore はローカルだけで完結する
- GitHub 等はバックアップ／共有先として後から追加できる
- 認証サービス障害があってもローカル保存機能は使える

VRChat 公式も、バージョン管理の利点を得るために GitHub 等へアップロードする必要はないと案内している。

## 3. 黙って破壊しない（Never Destroy Silently）

ユーザーが理解しないままデータを失う操作をしない。

- restore / checkout 相当操作の前に安全スナップショットを作る
- force push を MVP では提供しない
- `reset --hard` を通常 UI から提供しない
- conflict を勝手に解決しない
- `.gitignore` / `.gitattributes` は既存内容を無断で置換しない

## 4. Unity と VRChat を理解する

Git の一般論だけではなく Unity / VRChat / VPM の構造を理解する。

診断対象には少なくとも以下を含める。

- Unity プロジェクト構造
- `Assets/`
- `Packages/`
- `ProjectSettings/`
- `.meta` ファイル
- VPM manifest / Resolver
- VPM パッケージのソース管理除外状態
- Git LFS
- 大容量バイナリ

VRChat 公式の VPM ソース管理ガイドを基本ルールとして扱う。

## 5. 必要に応じて詳細を見せる（Progressive Disclosure）

最初は制作作業の言葉で見せ、必要な人には Git の詳細も見せる。

例:

- 「作業を保存」＋補足「Git ではコミットと呼びます」
- 「リモートへバックアップ」＋詳細表示で push / remote を確認可能

## 6. 危険な操作は Rust 側が管理する

Frontend から任意の Git / shell command を直接実行させない。

- Git 操作は Rust 側の明示的な application service を通す
- command と arguments は構造化して実行する
- shell 文字列連結を避ける
- Tauri capabilities は必要最小限にする
- 対象 path が登録済み project の範囲内か検証する

Tauri v2 の shell plugin は、危険な command scope をデフォルトで許可せず capabilities で明示する設計になっているため、この方針に合わせる。

## 7. ドキュメントも製品の一部とする

仕様・思想・重要な変更理由はリポジトリ内に残す。

- 製品思想: `docs/product/`, `docs/design/`
- 変更可能な要件: `docs/development/`
- 大きな設計判断: `docs/adr/`

コードだけを正解にしない。

## 参考資料

- Tauri Shell plugin: https://v2.tauri.app/plugin/shell/
- VRChat VPM source control: https://vcc.docs.vrchat.com/vpm/source-control/
- VRChat SDK updates: https://creators.vrchat.com/sdk/updating-the-sdk/
