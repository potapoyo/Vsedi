# M3 実装計画 — ローカル保存

## 目的

M3では、診断済みのUnity / VRChat projectについて、Gitの用語を通常UIへ露出しすぎずに、現在の変更をローカルcommitとして保存し、保存履歴と変更内容を確認できる状態を作る。

完了時のユーザー体験は次の一連の操作で確認する。

1. Unity projectを選択する
2. 未初期化なら、作成されるrepositoryとignore ruleの変更を確認して初期化する
3. 新規・変更・削除されたfileを確認する
4. 保存メモを入力して「作業を保存」を実行する
5. 保存されたcommit IDと時刻を確認する
6. 履歴からcommitを選び、変更fileと表示可能なdiffを確認する

この縦切りがApple Silicon macOSとWindowsで動作した時点をInternal Alpha候補とする。

## M2からの開始条件

2026-08-12時点の`main`では、M2のUnity / VRChat / VPM診断、Git境界診断、設定保持、ログ表示とexportが実装済みである。自動検証はRust 21テスト、clippy、TypeScript production build、生成型差分チェックに合格している。

M3の状態変更機能を実装する前に、最新commitで次の2点だけを実機で再確認する。

- 「ログフォルダ」でmacOSのFinderが実際のlog directoryを開く
- 親folderがGit rootのprojectで、`GIT_ROOT_OUTSIDE_PROJECT`のpathを主要UIへ表示しない

これらはM3のGit保存ロジックを妨げる項目ではないが、Internal Alpha判定まで未確認のまま残さない。

## 対象範囲

### 含める

- Git repositoryの初期化previewと実行
- 既存`.gitignore`を置換しないUnity / VPM ignore ruleの提案と追記
- repository全体の変更状態の読み取り
- 新規・変更・削除・rename・conflict・binaryの区別
- file単位のdiff読み取り
- 保存メモの入力、`add`、`commit`
- 保存直前の再検証と保存結果の表示
- commit履歴の一覧
- commit詳細、変更file、表示可能なdiff
- Rustの正本型からTypeScript bindingを生成
- Windows / macOSのnative smoke testとbundle確認

### 含めない

- remote、push、pull、clone
- branchの作成・切替
- merge、rebase、conflict解消
- file単位の保存対象選択
- amend、履歴書換え、commit削除
- 過去状態への復元（M4）
- Git LFS診断

## 安全方針

- Frontendへ任意のGit commandやshell commandを公開しない。用途別Tauri commandだけを追加する。
- すべての操作は、M2で正規化・検証したproject rootと検出済みrepository rootから開始する。
- 親folderがrepository rootの場合は正常構成として扱う。保存対象はrepository全体とし、project外の変更もpreviewへ必ず表示する。
- 保存対象はUIの表示内容と一致させる。preview後、保存直前にstatusを再取得し、内容が変わっていた場合は保存せず再確認を求める。
- conflict、未完了のmerge / rebase、Git lock、読み取り不能fileがある場合は保存を開始しない。
- Vsedi実行前からstaging済みの変更がある場合は、初期版では保存を拒否する。既存indexを自動でreset、unstage、上書きしない。
- `git add`後にcommitが失敗した場合は、worktreeを戻さず、indexが変化した可能性を`mayHaveMutated`と画面で明示する。
- 空の変更、空白だけの保存メモ、同時保存、二重clickを拒否する。
- Gitのstdout / stderrやdiffを通常のapplication logへ記録しない。credential helperを起動する操作はM3に含めない。
- pathは引数文字列へ連結せず、常にprocessの独立した引数として渡す。revisionはGitが返した完全なobject IDだけを受け付ける。

## 先に確定する仕様

実装着手時に短いADRまたはarchitecture追記として、次を固定する。

1. **index方針**: 通常UIではstagingを隠し、既存staged変更があれば安全のため停止する。
2. **保存範囲**: repository root配下の全変更を1回の保存対象とする。親repositoryの場合もproject外変更を隠さない。
3. **初期化方針**: `.git`がない場合だけ`git init`を許可し、既存repositoryや親repository内で重複初期化しない。
4. **ignore rule方針**: 既存内容と改行形式を保持し、不足ruleだけをpreview後に追記する。VPM ruleは現在の`VpmTrackingPolicy`に従う。
5. **diff上限**: 巨大text / binaryによるUI停止を防ぐため、file数、1 fileあたりのbyte数、全体byte数に上限を設け、打切り理由を型で返す。

## Rust / TypeScriptモデル案

Rustを正本として、少なくとも次の概念を共有型にする。実装名は既存命名に合わせて調整してよい。

- `RepositoryState`: repository root、初期化要否、HEAD有無、branch表示名、保存可否、blocking reason
- `ChangedFile`: 表示path、旧path、change kind、staged / unstaged状態、binary判定、project外判定
- `WorktreeSnapshot`: status token、変更一覧、conflict有無、既存staged変更有無
- `FileDiff`: path、text / binary / unavailable、patch、truncate状態
- `SaveRequest`: project path、status token、保存メモ
- `SaveResult`: full commit ID、short commit ID、保存メモ、author time、file件数
- `HistoryEntry`: full / short commit ID、保存メモ、author time
- `CommitDetail`: commit metadata、parent IDs、変更file一覧

保存時のerror codeは環境診断用codeと分け、少なくとも「repository不正」「状態変化」「conflict」「既存staged変更」「変更なし」「保存メモ不正」「add失敗」「commit失敗」「履歴読取失敗」をユーザーが区別できるようにする。

## GitAdapter実装方針

- statusはlocale非依存の`git status --porcelain=v2 -z --untracked-files=all`をfixtureで解析する。
- object IDやrepository情報は、機械可読な`rev-parse` / `symbolic-ref`を用途別関数で読む。
- diffは`--no-ext-diff`とNUL区切りのraw / numstat情報を組み合わせ、binaryをtextとして無理に表示しない。
- 保存は状態再検証後に`git add -A`、続けて明示した保存メモで`git commit`を実行する。成功後にHEADとstatusを再読込して結果を検証する。
- 履歴は区切り文字を固定した`git log`から解析し、画面表示用文字列のlocaleに依存しない。
- commit詳細は選択済みの完全なobject IDに限定し、worktreeを変更しないcommandだけを使う。
- processのexit code、stdout、stderrを分離し、errorへはsanitizedした必要最小限のdetailだけを渡す。

## 実装フェーズ

### Phase 1 — 契約と読み取り基盤

- index / 保存範囲 / ignore / diff上限を文書化する
- repository、status、changed-file、diff、history用のRust型とerror codeを追加する
- GitAdapterをsubcommand別に分割し、status parserのfixtureテストを追加する
- repository state、worktree status、file diffの読み取り専用commandを追加する
- TypeScript bindingを再生成し、Frontendに変更一覧を表示する

完了条件: 既存repositoryで新規・変更・削除・rename・binary・conflictを安全に区別して表示できる。

### Phase 2 — 初期化

- 未初期化、project自身がroot、親repositoryの3構成を区別する
- `.gitignore`変更previewを作成する
- previewと状態が一致するときだけ`git init`とignore追記を実行する
- 途中失敗時に「どこまで変更された可能性があるか」を返す

完了条件: 未初期化projectを、既存fileを失わず、重複repositoryを作らずに管理可能へ移行できる。

### Phase 3 — 作業を保存

- 保存メモUI、保存前summary、実行中状態、二重実行防止を追加する
- status tokenでTOCTOUを検出し、変化時は再previewを要求する
- blocking stateを確認してから`add` / `commit`を実行する
- 成功時にcommit ID、時刻、file件数を表示し、変更一覧を再読込する
- add成功後 / commit失敗のfixtureとユーザー向け復旧表示を実装する

完了条件: 変更のあるprojectで「作業を保存」を1回実行し、HEADに期待したcommitが作られる。変更なし、conflict、既存staged変更ではrepositoryを変更しない。

### Phase 4 — 保存履歴と詳細

- 件数制限付きの履歴一覧を追加する
- commit選択でmetadataと変更fileを表示する
- text diff、binary表示、truncate表示を追加する
- 初回commit、rename、削除、merge commitを含むfixtureでparserを検証する

完了条件: 保存直後のcommitが履歴先頭に現れ、選択すると保存メモ、日時、commit ID、変更fileと表示可能なdiffを確認できる。

### Phase 5 — 統合検証とInternal Alpha判定

- 一時repositoryを使うRust統合テストで、init→status→save→history→detailを通す
- project rootと親repositoryの両構成をテストする
- file名の空白、日本語、rename、削除、binary、大量diff、empty repositoryをテストする
- Apple Silicon macOSとWindowsでnative GUI smoke testを行う
- macOS DMGとWindows installerを生成し、起動確認する
- README、architecture、roadmap、Diaryを実装結果へ更新する

完了条件: ロードマップの「Unity projectを登録し『作業を保存』を押すと、その時点が履歴に残る」を両OSで再現でき、既知のデータ損失経路がない。

## テストマトリクス

| 領域 | 最低限のケース |
| --- | --- |
| status parser | untracked、modified、deleted、rename、type change、conflict、空白・日本語path、NUL区切り |
| repository境界 | 未初期化、project=root、親repository、壊れた`.git`、別worktree |
| ignore | fileなし、既存ruleあり、不足rule、CRLF / LF、末尾改行なし、読取不能 |
| save | 初回commit、通常commit、変更なし、空memo、状態変化、既存staged、conflict、add失敗、commit失敗 |
| history | 初回commit、複数commit、rename、削除、merge commit、binary、履歴なし |
| UI | loading、empty、blocking error、成功、再読込、二重click、長いpath / memo |
| platform | Apple Silicon macOS `.app` / DMG、Windows native app / installer、system Git差異 |

## 完了条件

- `cargo test`、`cargo clippy --all-targets -- -D warnings`、TypeScript typecheck、Vite production build、生成型差分チェックが成功する
- Git parserは実行環境のlocaleに依存せず、fixtureで主要状態を再現できる
- Frontendから任意のGit / shell commandを実行できない
- 保存前に対象変更が表示され、表示後に状態が変わった場合はcommitしない
- 失敗時にworktreeを自動で巻き戻さず、変更可能性と次の行動を説明できる
- 保存成功後にcommit IDと時刻が表示され、同じcommitを履歴と詳細画面から確認できる
- macOSとWindowsでinit→save→history→detailのnative smoke testに合格する
- Internal Alphaの既知制約をREADMEまたはrelease noteへ記録する

## 推奨する作業分割

1. M3-1: index / 保存範囲 / ignore / diff上限の仕様確定
2. M3-2: Git status parserと共有型
3. M3-3: repository初期化とignore preview
4. M3-4: 変更一覧とfile diff UI
5. M3-5: add / commitと保存結果
6. M3-6: history一覧とcommit詳細
7. M3-7: 両OS統合検証、配布build、Internal Alpha判定

各作業は読み取り機能、状態変更機能、UI、テストを同じ差分に詰め込みすぎず、parserと安全条件を先にreviewできる単位にする。
