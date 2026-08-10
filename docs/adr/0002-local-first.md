# ADR 0002: ローカルファーストとする

- 状態: 採用
- 日付: 2026-08-10

## 背景

主なユーザーニーズは、VRChat の Unity 作業を保存し、失敗したときに戻せることにある。GitHub 等のホスティングサービスを必須にすると、その価値を得る前にアカウント、認証、ネットワーク、プライバシーの問題が増える。

VRChat 公式の SDK 更新ガイドでも、GitHub 等へ repository をアップロードしなくても version control は有用であると案内されている。

## 決定

Vsedi はローカルファースト（Local First）とする。

製品の中心機能はローカル Git repository だけで成立しなければならない。

- repository の初期化
- 変更内容の確認
- 作業の保存（commit）
- 履歴の確認
- 復元内容のプレビュー
- 安全な復元

リモートサービスは、Vsedi Core 完成後に追加する任意のバックアップ／同期機能とする。

## 影響

利点:

- 初回利用フローを簡単にできる
- 非公開・購入アセットを最初から外部へアップロードする必要がない
- network / authentication 障害があっても中心機能を利用できる
- Vsedi の価値が特定の hosting provider に依存しない

欠点:

- ローカルだけの repository はディスク故障へのバックアップにはならない
- onboarding で「ローカル保存」と「リモートバックアップ」の違いを明確に説明する必要がある
- remote setup は別の明示的なフローになる

## UI 要件

ローカル commit が外部へアップロードされたように誤解させる表現をしない。

推奨用語:

- commit: `作業を保存`
- push: `リモートへバックアップ`

## 参考資料

- https://creators.vrchat.com/sdk/updating-the-sdk/
