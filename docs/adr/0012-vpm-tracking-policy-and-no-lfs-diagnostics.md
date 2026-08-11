# ADR 0012: VPM追跡方針の設定化とGit LFS診断の廃止

- 状態: 採用
- 日付: 2026-08-11

## 背景

Git LFS はすべてのUnity / VRChat projectで必要とは限らず、利用しないユーザーに未導入警告や `.gitattributes` 修正要求を表示すると、問題のないprojectを「要修正」と誤認させる。

また、VRChat公式のVPM source-control方針はpackage本体を除外してResolverを保持する構成だが、復元可能な状態を完全にGitへ記録するため、VPM package本体も追跡したい運用がある。

Unity project以外の関連ファイルを同じrepositoryで管理する場合、Git repository rootがUnity projectの親folderになることも正常な構成である。

## 決定

- VsediはGit LFSの導入有無を診断しない
- Git LFS ruleの有無、LFS対象候補、大容量ファイル候補を修正条件にしない
- system Gitを使用する設計は維持し、既存repositoryがGit LFSを利用していてもVsediが専用設定を管理しない
- Git repository rootがUnity project外にある状態はエラーや警告ではなく情報として表示する
- VPM package本体をGit管理から除外するか、Git管理に含めるかを設定で選択可能にする
- 初期値は公式デフォルトに合わせて「VPM packageを除外する」とする
- 選択した方針と `.gitignore` / Git追跡状態が矛盾する場合だけ警告する

初期実装ではVPM追跡方針をアプリ全体の設定とする。projectごとの設定が必要になった場合は、M3のproject登録モデルで拡張する。

## 設定migration

`settings.json` のschema versionを2へ更新し、schema 1からのmigrationでは `vpmTrackingPolicy: "EXCLUDE_PACKAGES"` を追加する。migration前の元ファイル保全はADR 0007に従う。

## 影響

- M1で実装した `git lfs version` 診断とGit LFSカードを削除する
- M2の `.gitattributes` / 大容量ファイル診断を削除する
- VPM package診断は単一の公式ルールではなく、ユーザーが選択した追跡方針を正本とする
- ADR 0009のGit LFS診断例と、ADR 0011のGit LFS診断責務を本ADRで置き換える
