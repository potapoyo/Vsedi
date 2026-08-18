# ADR 0014: M4の安全な復元方式

- 状態: 保留（Slint移植後に再評価）
- 日付: 2026-08-13

> 2026-08-18: M4の実装計画をいったん破棄したため、このADRは現在の実装基準ではなく、将来M4を再計画する際の参考記録として保持する。

## 背景

M4では、ユーザーが保存履歴から過去の状態へ戻した後も、復元直前の状態を失わずに戻せる必要がある。通常UIではGitのbranch、reset、stash、detached HEADを操作させず、既存履歴も書き換えない。

## 決定

### 復元は現在のbranchへ新しいcommitとして記録する

- `HEAD`、branch ref、既存commitを過去へ移動・削除・書換えしない。
- 対象revisionのtreeをindexとworktreeへ反映し、そのtreeを親が現在の`HEAD`である新しい「復元commit」として記録する。
- 復元対象はGitが返した完全なobject IDに限定し、実行直前にcommit objectと到達可能性を再検証する。
- `reset --hard`、detached HEAD、強制branch切替、stashは通常フローで使用しない。

これにより履歴は線形のまま保持され、過去へ戻した事実と、その後に復元前へ戻した事実を履歴から確認できる。

### 復元前スナップショット

- worktreeがcleanなら、復元直前の`HEAD`を復旧先として記録し、追加commitは作らない。
- 未保存変更がある場合は、復元開始前にrepository全体の変更を一つの「復元前スナップショットcommit」として保存する。
- スナップショットはM3の保存と同じく、preview tokenの再検証後に`git add -A`と`git commit`で作成する。
- ignored fileはGit管理対象外のまま維持し、復元でも変更しない。
- snapshot作成に失敗した場合は復元を開始しない。`git add`後など変更可能性がある失敗は`mayHaveMutated = true`で返す。
- 既存staged変更、conflict、merge / rebase / cherry-pick等の未完了操作、Git lock、対応外のrepository構成では停止する。

スナップショットcommitは隠しrefや一時領域ではなく通常履歴へ残す。Gitのgarbage collectionやアプリ設定消失に依存せず、Vsedi外からも復旧点を確認できることを優先する。

### PreviewとTOCTOU対策

- previewはrepository全体を対象とし、追加・変更・削除・rename・binary・project外の変更を表示する。
- 対象revisionのtreeと現在のworktreeから、復元で変化するpathを算出する。未追跡fileもスナップショット後に追跡対象となるためpreviewへ含める。
- previewには対象commit ID、preview時点の`HEAD`、worktree fingerprint、変更file summaryから生成したtokenを含める。
- 実行直前にproject / repository境界、`HEAD`、worktree、blocking state、対象revisionを再取得し、previewと異なる場合はrepositoryを変更せず停止する。
- file数と表示用diff byte数に上限を設ける。上限超過時も復元対象file数と省略理由を表示し、黙って一部だけ復元しない。

### 復元の実行と検証

- snapshotが必要なら先に作成し、そのcommit IDを復旧先として保持する。cleanなら実行開始時の`HEAD`を復旧先とする。
- 対象revisionのtreeをrepository全体のindex / worktreeへ反映し、復元commitを作成する。
- commit作成後に、復元commitのtree IDが対象revisionのtree IDと一致すること、worktreeがcleanであること、`HEAD`が作成したcommitを指すことを検証する。
- 検証完了後に、復元commit ID、復元対象commit ID、復元前へ戻るためのcommit IDを結果として返す。
- 復元前へ戻る操作も、保持したcommit IDを対象とする新しいpreviewと復元commitで行う。特別な履歴書換えは行わない。
- snapshot後またはtree反映開始後の失敗は、原則として`mayHaveMutated = true`とし、確認できた`HEAD`、snapshot commit ID、失敗段階を返せるモデルにする。

### Unity起動中

- platform adapterで、対象projectを開いているUnity processをbest-effortで検知する。
- 検知できた場合は復元を開始せず、Unityを閉じて再確認するよう案内する。
- 検知不能は安全の保証として扱わず、確認画面では常にUnityを閉じる注意を表示する。

## 対象外

- ignored fileのバックアップまたは復元
- staged / unstaged状態の区別の復元
- submodule、linked worktree、sparse checkoutの復元
- merge commitの作成、自動merge / rebase、conflict解消
- branch作成・切替、履歴削除、commitのamend
- Unity processの強制終了

## 結果

- 復元操作によりcommit数は増えるが、現在状態と復元操作の履歴を失わない。
- 未保存変更も通常commitとして残るため、利用者がアプリ設定を失ってもGit履歴から復旧できる。
- 復元の途中失敗は完全なtransactionにはならないため、段階別エラー、保守的な`mayHaveMutated`、統合testが必要になる。
