# Vsedi Product Vision

## Purpose

Vsedi は、VRChat のアバター／ワールド制作者が Unity プロジェクトを安全に保存・確認・復元するためのデスクトップアプリである。

一般的な Git GUI を再実装することではなく、Git の仕組みを利用して VRChat 制作に必要な「セーブポイント」を分かりやすく提供する。

## Target user

主な対象は次のような制作者。

- Unity と VRChat Creator Companion / VPM は使える
- Git の branch / rebase / reset などは詳しくない
- アバター改変やワールド制作で「壊す前に戻れる状態」を残したい
- BOOTH 等の購入アセットを扱うため、公開範囲には慎重でありたい
- 必要なら GitHub 等へバックアップしたいが、GitHub は必須にしたくない

## Core job

ユーザーが次の流れを安心して行えること。

1. Unity プロジェクトを Vsedi に登録する
2. プロジェクトの Git / Unity / VRChat 状態を診断する
3. 作業の節目で「作業を保存」する
4. 保存履歴と変更内容を確認する
5. 問題が起きたら安全スナップショットを作成してから過去の状態へ戻す
6. 必要に応じてリモートへバックアップする

## Positioning

Vsedi は「VRChat 向け Git クライアント」ではあるが、UI 上の主語は Git ではなく制作作業とする。

例:

- commit → 作業を保存
- history → 保存履歴
- restore / checkout → この状態に戻す
- push → リモートへバックアップ
- pull / fast-forward → リモートから同期

必要な場面では Git 用語も補足表示し、仕組みそのものを完全には隠さない。

## Success criteria for the first usable version

次が一連で動けば Vsedi Core が成立したとみなす。

> Unity プロジェクトを登録し、最初の保存を作り、変更後に履歴を確認し、現在状態を失わず安全に過去へ戻せる。

リモート同期はこの価値が成立した後に追加する。

## References

- VRChat Creator Companion: Using Source Control with the VPM
  - https://vcc.docs.vrchat.com/vpm/source-control/
- VRChat Creation: Updating the SDK
  - https://creators.vrchat.com/sdk/updating-the-sdk/
