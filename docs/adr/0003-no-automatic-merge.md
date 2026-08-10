# ADR 0003: MVP では分岐した履歴を自動マージしない

- 状態: 採用
- 日付: 2026-08-10

## 背景

初心者向けクライアントが remote 同期時の衝突に対して merge / rebase を自動選択すると、データ損失や理解しにくい worktree 状態を生みやすい。

Unity プロジェクトには scene、prefab、metadata、binary asset も含まれるため、一般的な text conflict の処理だけでは不十分な場合がある。

## 決定

MVP のリモート同期では、fast-forward 可能で安全な場合にのみ incoming history を自動反映する。

local と remote の履歴が分岐している場合、Vsedi は次のように動作する。

1. remote history を fetch する
2. 履歴の分岐を検出する
3. worktree や history を変更する前に停止する
4. local / remote の双方に固有の変更があることを表示する
5. Vsedi がそれらを自動統合しないことを説明する

MVP では初心者に merge と rebase の選択を求めない。

## 影響

利点:

- 同期時の挙動を予測しやすい
- conflict を大量に含む状態を黙って作らない
- まず信頼できる検出と説明に実装を集中できる

欠点:

- 履歴が分岐したユーザーは、外部 Git tool または将来の Vsedi conflict workflow が必要になる
- 初期版では複数人での共同作業シナリオが制限される

## 再検討する条件

ローカル保存・復元・基本的なリモートバックアップが安定し、実利用から必要性が確認された後に、専用の conflict resolution 設計を追加する。
