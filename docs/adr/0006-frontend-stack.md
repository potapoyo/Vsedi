# ADR 0006: フロントエンド技術構成

- 状態: 採用
- 日付: 2026-08-10

## 背景

Vsedi は Windows / Apple Silicon macOS の両方で動作する Tauri v2 デスクトップアプリケーションである。

UI はプロジェクト一覧、診断結果、変更ファイル一覧、保存履歴、設定、確認ダイアログ、初回チュートリアルなど、比較的典型的なデスクトップアプリの画面を中心とする。

フロントエンド技術そのものを製品上の差別化要素にはせず、保守性、情報量、導入容易性、Tauri との相性を優先する。

## 決定

M1 以降のフロントエンド構成を次のようにする。

- **React**
- **TypeScript**
- **Vite**
- **pnpm**
- **Tailwind CSS**
- **shadcn/ui**

状態管理は、M1 では React 標準の state を基本とする。

Rust 側から取得する診断・履歴などの非同期データ管理が複雑になった段階で **TanStack Query** を導入する。Redux などの大規模なグローバル状態管理ライブラリは、実際に必要性が生じるまでは導入しない。

Rust / TypeScript 間の共有データ型は ADR 0008 に従い、Rust を正本として `serde + ts-rs` から生成する。

## UI 方針

Web サイトのような装飾性より、デスクトップアプリとしての分かりやすさを優先する。

- Windows / macOS で基本的に共通の UI 構造を使う
- OS ごとに極端に異なる見た目にはしない
- 左側ナビゲーションとメインコンテンツを基本レイアウト候補とする
- 危険操作は色だけに頼らず文言と確認 UI で明示する
- アイコンだけで操作の意味を伝えない
- アニメーションは控えめにする
- Git 用語より「作業を保存」「保存履歴」など製品用語を優先する

shadcn/ui のコンポーネントはプロジェクト側に取り込んで利用し、Vsedi の UX に合わせて必要な調整を行う。

## Tauri / Rust との境界

フロントエンドは Git command を直接構築・実行しない。

例えば UI は `saveWork(projectId, message)` のようなアプリケーション操作を要求し、実際の `git status` / `git add` / `git commit` 等の組み立てと実行は Rust 側の Service / Adapter が担当する。

この方針は `docs/design/architecture.md`、ADR 0001、ADR 0008 に従う。

## 理由

- Tauri v2 と組み合わせやすい一般的な構成である
- React / TypeScript は情報量が多く、将来の保守や参加者増加にも対応しやすい
- Vite はデスクトップ UI の開発環境として構成が単純で高速
- pnpm は依存管理を明確かつ効率的に行える
- Tailwind CSS と shadcn/ui は、独自 UI を作りつつ基本コンポーネント実装の負担を減らせる
- Redux 等を初期段階から入れず、必要以上に構成を複雑にしない

## 影響

良い点:

- 一般的で情報が多い技術構成になる
- UI コンポーネントを素早く構築できる
- Vsedi 固有の見た目や挙動へ調整しやすい
- TypeScript により Tauri command 境界の型安全性を高めやすい

注意点:

- React / Tailwind / shadcn/ui の依存更新を継続的に管理する必要がある
- shadcn/ui は完成済みライブラリをそのまま参照する方式ではなく、取り込んだコンポーネントを自分たちで保守する必要がある
- 状態管理が複雑化した場合は追加の設計判断が必要になる

## 再検討する条件

- React が Tauri 上で重大な性能・互換性問題を起こす場合
- 状態管理が React 標準 state では維持できなくなった場合
- UI コンポーネント層の保守コストが過大になった場合

## 参考資料

- Tauri Create Project: https://v2.tauri.app/start/create-project/
- shadcn/ui Vite: https://ui.shadcn.com/docs/installation/vite
