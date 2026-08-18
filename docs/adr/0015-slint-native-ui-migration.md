# ADR 0015: Slint ネイティブUIへの移植

- 状態: 採用（移植中）
- 日付: 2026-08-18

## 背景

M3までのVsediはTauri v2 + React + TypeScriptで実装されている。一方、製品の対象はWindowsとApple Silicon macOSのデスクトップアプリであり、UIの実行時にWebViewとJavaScriptのcommand wrapperを必要としない構成を試作する価値がある。

移植中もM3の動作を基準として扱い、Git、Unity/VRChat診断、保存、履歴、設定、安全エラーの挙動をUI技術の変更で失わない必要がある。

## 決定

Slintを最終候補のネイティブUIとして試作・移植する。

- `.slint` は表示、入力、レイアウト、アクセシビリティ、UI callbackを担当する
- Git CLI、filesystem、project診断、保存、履歴、設定migration、エラー分類はRust側を正本とする
- UI frameworkから呼び出す操作は `src-tauri/src/application.rs` のapplication facadeを境界にする
- Tauri commandは移植完了まで互換実装として保持し、Slint画面と同じserviceを呼び出す
- SlintのUIスレッドでGitやfilesystemの長時間処理を実行せず、結果をevent loopへ戻す
- M3の機能同等性、Windows/macOS native build、UIテスト、キーボード操作、アクセシビリティを確認してからTauri/Reactを削除する

最終的な構成は次を目標とする。

```text
Slint UI
    |
    v
Application facade / presenters
    |
    v
Rust services
    |-- Git / project diagnostics
    |-- save / history / diff
    |-- settings / logging
    `-- platform adapters
```

## 移植段階

1. Tauri/ReactのActionsを一時停止し、M3の基準を固定する
2. application facadeを追加し、UIから独立したRust操作境界を作る
3. Slintの最小window、environment診断、project診断を確認する
4. Home、project、設定、worktree、diff、保存、履歴、ログの順に移植する
5. Rust presenter testとSlint native smoke testを追加する
6. ユーザーの実projectで確認する
7. ActionsをSlintのworkspace test/buildへ置換し、自動実行を戻す
8. 受け入れ条件を満たした後にTauri/React資産を削除し、mainへマージする

## 受け入れ条件

- M3の保存・履歴・diff・診断・設定の主要操作がSlint版で実行できる
- 操作前preview、stale検出、競合検出、既存staged変更の停止が維持される
- Windows x86_64とApple Silicon macOSでdebug起動とrelease bundleが成功する
- UI操作から任意のshell/Git commandを実行できない
- 自動テストがRust service、presenter、Slint callbackの失敗を検出できる
- キーボード操作と主要要素のアクセシビリティ識別子を確認できる
- 失敗時にReact/Tauri版へ戻せる、または移植を中止する判断材料が残っている

## 影響

良い点:

- RustサービスをUI技術から分離できる
- WebView依存を減らし、native desktop UIとしての検証対象を明確にできる
- Slintのproperty/callbackとRust DTOの境界を一つにできる

注意点:

- SlintのUI markupと既存React画面の機能同等性を確認する必要がある
- Windows/macOSのnative backend、renderer、アクセシビリティを個別に検証する必要がある
- 移植完了まではTauri/ReactとSlintのbuild経路が一時的に共存する

## 再検討する条件

- WindowsまたはApple Silicon macOSで安定したnative build/runtimeを実現できない
- 主要画面のキーボード操作またはアクセシビリティを満たせない
- Rust service boundaryを保ったまま必要なUI表現を実装できない
- M3の安全性・機能同等性を維持できない
