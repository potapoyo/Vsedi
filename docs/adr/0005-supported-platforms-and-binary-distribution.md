# ADR 0005: 対応 OS とバイナリ配布

- 状態: 採用
- 日付: 2026-08-10

## 背景

Vsedi は VRChat 制作者が日常的に利用するデスクトップアプリであり、開発環境を用意できるユーザーだけを対象にはしない。

また、Windows と macOS の両方で利用できることが製品要件である。アプリ基盤には、両 OS 向けのネイティブデスクトップアプリとインストール用成果物を生成できる Tauri v2 を採用する。

macOS については、正式対応範囲を Apple Silicon に限定する。Intel Mac は正式対応対象外とする。

## 決定

Vsedi の正式対応環境と配布方式を次のようにする。

### Windows

- 正式対応 OS とする
- Tauri v2 でネイティブアプリをビルドする
- Windows のネイティブビルド環境で検証する
- 一般ユーザー向けに NSIS `.exe` または MSI `.msi` のインストーラーを少なくとも1種類提供する

### macOS

- 正式対応 OS とする
- **Apple Silicon（arm64）のみ**を正式対応する
- Intel Mac（x86_64）は正式対応しない
- Apple Silicon Mac のネイティブビルド環境で検証する
- 一般ユーザー向けに `.app` を含む `.dmg` を提供する

### 共通

- Tauri v2 をデスクトップアプリ基盤として使用する
- 一般ユーザーに Rust / Node.js / package manager 等の開発ツールチェーンを要求しない
- バイナリ配布は追加機能ではなく製品の必須要件とする
- Windows / macOS の成果物は各ネイティブ環境でビルド・検証する

## コード署名について

**当面の公式配布物は未署名とする。**

Windows / macOS ともに、コード署名や macOS Notarization は初期リリースの必須要件にしない。

### macOS

Apple Silicon 向け `.dmg` を配布するが、Developer ID による署名および Notarization は行わない。

そのため Gatekeeper による警告や追加操作が発生する可能性がある。配布ページとインストール手順では、その事実と安全な起動方法を明示する。

アドホック署名についても「ユーザーから見た信頼済みコード署名」と誤解されないよう扱い、初期リリースで利用するかどうかはビルド工程上の必要性に応じて判断する。

### Windows

Windows インストーラーも当面は未署名で配布する。

SmartScreen 等の警告が発生する可能性があるため、配布ページとインストール手順で明示する。

## 理由

- Windows / macOS の両対応は製品の前提であり、後から変更すると設計・テスト・配布工程への影響が大きい
- Tauri v2 は両 OS 向けのデスクトップアプリと配布成果物を生成できる
- 一般ユーザーに開発環境を要求すると、初心者向けという Vsedi の製品方針と矛盾する
- Apple Silicon のみに限定することで macOS のビルド・テスト範囲を明確にできる
- 初期段階では署名コストや証明書管理を導入せず、まず安定したバイナリ配布を成立させる

## 影響

良い点:

- 対応環境とリリース成果物が明確になる
- 各 OS のネイティブ環境で確実に検証できる
- 一般ユーザーがソースからビルドする必要がない
- macOS の対応範囲を Apple Silicon に絞れる
- 初期リリース工程が簡潔になる

注意点:

- macOS では Gatekeeper の警告が発生し得る
- Windows では SmartScreen の警告が発生し得る
- インストール手順で未署名であることを明確に説明する必要がある
- 将来、警告のない一般配布を重視する場合はコード署名を再検討する必要がある

## 再検討する条件

- ユーザーから Intel Mac 対応への十分な需要が確認された場合
- コード署名による警告低減を製品上重要と判断した場合
- App Store / Microsoft Store での配布を行う場合

## 参考資料

- Tauri Distribution: https://v2.tauri.app/distribute/
- Tauri macOS Code Signing: https://v2.tauri.app/distribute/sign/macos/
- Tauri Windows Code Signing: https://v2.tauri.app/distribute/sign/windows/
