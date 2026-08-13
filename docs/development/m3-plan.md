# M3 再計画 — ローカル保存と Internal Alpha

Status: In Progress

Replanned: 2026-08-12

Last updated: 2026-08-13

Reference: Issue #18

## 目的

M3では、診断済みのUnity / VRChat projectを登録し、Gitの用語を通常UIへ露出しすぎずに、現在の変更をローカルcommitとして保存し、保存履歴と変更内容を確認できる状態を作る。

完了時の中心導線は次のとおりとする。

1. ホームで管理Projectを追加または選択する
2. 未初期化なら、作成されるrepositoryとignore ruleの変更を確認して初期化する
3. 「現在の作業」で新規・変更・削除されたfileを確認する
4. 保存メモを入力して「作業を保存」を実行する
5. 「保存履歴」でcommit、変更file、表示可能なdiffを確認する
6. リポジトリ設定と全体設定を、それぞれの影響範囲に応じて変更する

この導線と配布物をApple Silicon macOSとWindowsで確認できた時点をInternal Alpha候補とする。

## 再計画の前提

2026-08-12に追加したWindows native UI workflowは、release executableのbuildとWebView2 / Edge WebDriverの準備までは成功したが、WebDriverの`POST /session`で`DevToolsActivePort file doesn't exist`となり、テスト本体を1件も開始できなかった。実行対象をcapabilityとserviceの両方へ明示した後も再現したため、native UI CIの調査は一旦中断している。

この失敗はM3のRust / React実装に対するテスト失敗ではなく、Windows runner上でnative WebDriver sessionを作成できないテスト基盤の問題として扱う。ただし、native testが成功したことにもならないため、Windows native確認はInternal Alphaの未完了条件として残す。

検証経路は混同せず、次の4層に分ける。

| 層 | 役割 | 現在の状態 |
| --- | --- | --- |
| 通常CI | typecheck、frontend build、Rust test、Clippy、生成型整合性 | 利用可能 |
| Playwright smoke | IPCをmockした主要画面とnavigationの確認 | Windows / macOSのfixtureを分離した4ケースをローカルで成功。PR / 手動workflowを実装済み、Actions実行記録が未完了 |
| Native UI test | WebView、Rust IPC、native windowを通る自動確認 | embedded WebDriver方式へ更新し、macOS実機で最小ケースが成功。Windows / macOS Actions実行が未完了 |
| 配布物smoke | `.app` / DMG、Windows app / installerの実機確認 | macOS現行UIの`.app`は合格、DMG確認が未完了。Windowsは再ビルドとexe起動まで成功、GUI操作が未完了 |

## 現在地

| 領域 | 状態 | 補足 |
| --- | --- | --- |
| repository境界・Git診断 | 完了 | project自身のrepositoryと親repositoryを区別する |
| 初期化preview | 完了 | 既存ignoreを置換せず、不足ruleだけを提示・追記する |
| status・worktree diff | 完了 | rename、binary、conflict、project外変更を扱う |
| 作業保存 | 完了 | TOCTOU、既存staged、conflict、二重実行を防止する |
| 保存履歴・commit詳細 | 完了 | text / binary / unavailable / truncatedを区別する |
| application log | 完了 | 5段階のlevelと保持期間内の全ログ表示を実装済み |
| 画面分割 | 完了 | ホーム、現在の作業、保存履歴、リポジトリ設定、全体設定 |
| 管理Project・タグ・検索 | 実装済み | schema 6 migration、最終更新順、複数タグ設定・タグ絞り込み・名前/path/tag検索を実装済み |
| stale Project管理 | 実装済み・配布物での再確認待ち | 場所の再指定、一覧からの削除、重複登録防止を実装 |
| repository固有設定 | 実装済み・配布物での詳細操作確認待ち | VPM overrideをsettings schema 5からschema 6へ引き継ぎ、診断・初期化で実効値を使用 |
| ignore template・差分適用 | 実装済み・配布物での詳細操作確認待ち | 全体設定で編集し、repository設定で不足ruleをpreviewして追加 |
| UI smoke更新 | 実装・ローカル完了 | 現行UIの4ケースをWindows / macOS fixtureで各4件成功。PR / 手動workflowのActions実行は未記録 |
| Windows native検証 | embedded方式のActions実行待ち・手動GUI未完了 | 旧external方式の`DevToolsActivePort`依存を廃止。新方式のWindows runner結果と配布物GUI操作が未完了 |
| macOS最終native検証 | native自動テストと`.app`合格・DMG未完了 | embedded WebDriverで起動、全体設定、実OS / System Git表示を確認済み。現行UI DMGの生成・インストールが残る |

## 他PC向け引き継ぎ時点の残タスク

最新の引き継ぎ基準は、リモートブランチ`codex/m3-local-save`の`839360a`（Windows GUI smoke test build result）とする。作業開始時は次で同期する。

```sh
git pull --ff-only origin codex/m3-local-save
```

優先順は次のとおり。

1. **Windows配布物GUI smoke** — 再生成したMSI / NSISの少なくとも一方（可能なら両方）をインストールし、環境診断、Project追加、初期化preview、現在の作業の保存、履歴・diff、タグ・設定、再起動後の復元を確認する。Computer Use helperの`EPERM`を解消できない場合は、Windows上の手動操作で代替し、スクリーンショットとログを記録する。
2. **macOS現行UI DMG** — `bundle_dmg.sh`のエラーを解消または切り分け、現行UIのDMG生成・マウント・起動を確認する。`.app`の現行UI smokeは合格済み。
3. **UI workflowの実行記録** — Playwrightとembedded native UIの両workflowをWindows / Apple Silicon macOSで実行し、テスト結果、スクリーンショット、driver / app logを確認する。失敗しても通常CIの必須checkにはしない。
4. **M3完了判定と文書更新** — 両OSの配布物で`init → save → history → detail`を確認した後、README、roadmap、既知制約、M3計画の状態を更新し、Internal Alpha候補を判定する。

Windows native UI CIは旧external driver方式を廃止し、アプリ内のdebug限定embedded WebDriver方式へ更新した。配布物の手動GUI smokeとは独立して結果を記録し、Actionsで未検証の項目を成功扱いしない。

## 対象範囲

### M3に含める

- Git repositoryの初期化previewと実行
- ユーザーが編集可能なUnity / VPM ignore templateと、不足ruleのpreview・追記
- repository全体の変更状態とfile diffの読み取り
- 保存メモ、`add`、`commit`、保存直前の再検証
- commit履歴、commit詳細、変更file、表示可能なdiff
- 管理Project一覧、最終更新順、複数タグ設定・タグ絞り込み・Project検索
- ホーム、現在の作業、保存履歴、リポジトリ設定、全体設定の分離
- 全体のVPM既定値とrepository固有override
- Rustの正本型からTypeScript bindingを生成する仕組み
- 通常CI、Playwright smoke、両OSのnative / bundle確認

### M3に含めない

- remote、push、pull、clone
- branchの作成・切替
- merge、rebase、conflict解消
- file単位の保存対象選択
- amend、履歴書換え、commit削除
- 過去状態への復元（M4）
- Git LFS診断
- UI testを毎commitで自動実行する運用

## 維持する安全条件

- Frontendへ任意のGit commandやshell commandを公開しない。用途別Tauri commandだけを使用する。
- 操作は正規化・検証済みのproject rootとrepository rootから開始する。
- 親folderがrepository rootの場合もrepository全体を保存対象とし、project外の変更をpreviewへ表示する。
- preview後、保存または初期化の直前に状態を再取得し、変化していれば処理を停止する。
- conflict、未完了のmerge / rebase、Git lock、読み取り不能file、既存staged変更がある場合は保存しない。
- `git add`後にcommitが失敗してもworktreeを巻き戻さず、indexが変化した可能性を表示する。
- 既存`.gitignore`を置換せず、元の改行形式を保って不足ruleだけを追記する。
- 空の変更、空白だけの保存メモ、同時保存、二重clickを拒否する。
- Gitのstdout / stderrやdiffを通常のapplication logへ記録しない。
- revisionはGitが返した完全なobject IDだけを受け付ける。

## 残工程

### Phase A — 現在の製品差分を確定する（実装・ローカル検証済み）

- 管理Project一覧、最終更新順、タグ編集・解除・絞り込み・検索を実装・review済み
- schema 1〜5からschema 6へのmigrationと、単一カテゴリからタグへの変換、破損・未来schemaの拒否を実装・テスト済み
- stale pathの表示、再指定、削除、重複登録防止を実装済み。配布物での詳細操作確認は残る
- 通常CI相当のRust test、Clippy、typecheck、production build、生成型差分チェックを複数回成功済み

完了条件: 既存設定を失わずに全Projectを一覧でき、タグが再起動後も保持される。

### Phase B — 設定画面とignore安全性を確認する

VPM tracking policyのrepository override、schema migration、ignore template編集・差分適用は実装済み。Mac現行UIで設定画面の表示は確認済みだが、Windows配布物を含む詳細操作の再確認が残っている。

- repository固有のVPM追跡方針を「全体設定に従う / 除外する / 含める」で保存する
- 実効値と、全体既定値・repository overrideのどちらが採用されたかを表示する
- 全体設定でignore templateを編集し、リポジトリ設定から現在のignoreとの差分previewと不足ruleの明示適用を行えるようにする
- repository設定はapp dataの`settings.json`へ保存し、設定変更だけでrepositoryをdirtyにしない
- schema migration、canonical path、stale repository設定をテストする

完了条件: 全体設定とrepository設定の責務がUIと保存形式の両方で一致し、設定画面を開いただけではrepositoryを変更しない。

### Phase C — UI smoke testを現行画面へ追従させる（実装・ローカル完了、Actions実行待ち）

- `origin/main`のUI test基盤をM3 branchへ取り込み済み
- JSONテストケースを、ホーム、Project選択、現在の作業、保存履歴、両設定画面、タグ編集・絞り込み・Project検索へ更新済み
- Rust IPCをmockし、成功・empty・blocking errorを決定的に再現する4ケースをローカルで成功済み
- WindowsとApple Silicon macOSで異なる実行環境fixtureを使い、ローカルで各4ケースを成功済み
- `pull_request`と`workflow_dispatch`でWindows / macOS matrixを実行し、結果とPlaywright artifactを保持するworkflowへ更新済み
- GitHub Actions上で両OSの実行結果とartifactを確認する

完了条件: 両OSのPlaywright smokeが現行UIの主要navigationと表示を通過する。

### Phase D — 製品機能の回帰検証を仕上げる

- 一時repositoryを使うRust統合テストでinit→status→save→history→detailを通す（基本flowは追加済み。全matrixの記録を仕上げる）
- project rootと親repositoryの両構成を確認する
- 空白・日本語path、rename、削除、binary、大量diff、empty repositoryを確認する
- 設定migration、管理Project順、タグ、Project検索、repository overrideを含める
- typecheck、production build、Rust test、Clippy、生成型差分チェックを成功させる

完了条件: M3の状態変更と設定migrationに既知のデータ損失経路がなく、通常CIが成功する。

### Phase E — Native UI CIを再開する（実装・macOSローカル完了、Actions実行待ち）

- `@wdio/tauri-service`のembedded providerを採用し、WindowsのEdgeDriverとmacOSの外部driverへの依存を廃止した
- `native-ui-test` Cargo feature、テスト専用Tauri設定、frontend pluginをdebug test buildだけへ分離した。通常buildではWDIOコード、権限、WebDriver serverを含めない
- Windows / macOS matrixでdebug appをbuildし、起動、ホーム、全体設定、実行環境、System Gitを確認する手動workflowを実装した
- 失敗時のスクリーンショットとWDIO / backend log、成功時の実行環境スクリーンショットをartifactへ保存する
- Apple Silicon macOS実機でnative UI smoke 1件が成功した
- GitHub Actions上でWindows / macOS jobを実行し、両方の結果とartifactを確認する

完了条件: Actions上の両OSでnative test本体が少なくとも1件成功し、失敗時にもdriver / app logとスクリーンショットを回収できる。安定するまでは通常CIの必須checkにしない。

### Phase F — 両OSの配布物smokeとInternal Alpha判定

- Apple Silicon macOSでは現行UIの`.app`でinit→save→history→detailを確認済み。現行UI DMGの生成・マウント・起動を確認する
- Windowsでは再ビルド、MSI / NSIS生成、`vsedi.exe`の直接起動まで確認済み。installerのインストールと同じ中心導線を確認する
- native UI自動化で未検証のOS統合は手動チェックリストで補い、結果を日記へ記録する
- README、architecture、roadmap、既知制約を実装結果へ更新する

完了条件: 「Unity projectを登録し『作業を保存』を押すと、その時点が履歴に残る」を両OSの配布物で再現できる。

## テストマトリクス

| 領域 | 最低限のケース |
| --- | --- |
| status parser | untracked、modified、deleted、rename、type change、conflict、空白・日本語path、NUL区切り |
| repository境界 | 未初期化、project=root、親repository、壊れた`.git`、別worktree |
| ignore | fileなし、既存ruleあり、不足rule、CRLF / LF、末尾改行なし、読取不能、編集済みtemplate |
| save | 初回commit、通常commit、変更なし、空memo、状態変化、既存staged、conflict、add失敗、commit失敗 |
| history | 初回commit、複数commit、rename、削除、merge commit、binary、履歴なし |
| settings | schema 1〜6、未来schema、破損JSON、タグ、全体既定値、repository override、stale path |
| UI | loading、empty、blocking error、成功、再読込、二重click、長いpath / memo、タグ絞り込み、Project検索 |
| platform | Playwright Windows / macOS、Apple Silicon `.app` / DMG、Windows native app / installer |

## M3完了条件

- 通常CIの全項目が成功する
- Frontendから任意のGit / shell commandを実行できない
- 保存前に対象変更が表示され、表示後に状態が変わった場合はcommitしない
- 失敗時にworktreeを自動で巻き戻さず、変更可能性と次の行動を説明できる
- 保存成功後にcommit IDと時刻が表示され、同じcommitを履歴と詳細画面から確認できる
- 管理Project、タグ、全体設定、repository設定が再起動後も安全に復元される
- Windows / macOSのPlaywright smokeが現行画面で成功する
- macOSとWindowsの配布物でinit→save→history→detailのnative smokeに合格する
- native UI CIが未安定の場合、その事実を既知制約として残し、手動native smokeでInternal Alpha判定を補完する

## CIを除外した実行順序

1. Phase A: 管理Project・タグ・検索差分を確定
2. Phase B: repository固有設定を完成
3. Phase D: 製品機能のローカル回帰検証を完成
4. Phase F: Macのロック解除後に現行UIの配布物smokeを実施
5. Windowsが利用可能になった段階で配布物smokeを実施
6. README、Diary、既知制約を更新してInternal Alpha判定

Phase C（Playwright）とPhase E（native UI CI）はこの作業列から除外し、製品実装・ローカル検証・手動配布物確認が完了した後に別工程として再開する。

Windows native CIの調査はPhase A〜Dを妨げない。driver調査が長期化した場合も、製品実装・通常CI・Playwright・手動native smokeの結果を個別に記録し、未確認項目を成功扱いしない。
