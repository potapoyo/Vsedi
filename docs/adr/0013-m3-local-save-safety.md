# ADR 0013: M3ローカル保存のindex・preview安全方針

- 状態: 採用
- 日付: 2026-08-12

## 決定

- 保存対象は検出した repository root 配下の全変更とする。Unity project が親 repository 内にある場合も、project 外の変更を隠さない。
- 通常 UI に staging を公開しない。Vsedi 実行前から staged の変更がある場合は、index を変更せず保存を停止する。
- `git status --porcelain=v2 -z --untracked-files=all` の出力と変更ファイルの内容 fingerprint を preview の正本とし、その checksum を保存 request に含める。同じ path の内容だけが変わった場合も検出できるようにする。保存直前に同じ状態を再読込し、異なれば commit しない。
- conflict のある worktree、空の変更、空白だけの保存メモを拒否する。
- `git add -A` 成功後に commit が失敗した場合、worktree や index を自動復元しない。エラーの `mayHaveMutated` を true とし、ユーザーへステージ済みの可能性を通知する。
- 初期化は repository 外の場合だけ許可する。`.gitignore` の候補は `settings.json` の `ignoreTemplates` を正本とし、既存内容・改行形式を維持して不足ルールだけを preview 後に追記する。初期化後の追記失敗も変更可能性として通知する。

## 結果

M3 では Git の高度な操作を UI から提供しないまま、ユーザーが確認した変更内容だけを一つのローカル commit として保存できる。diff / history は後続の読み取り機能として追加する。
