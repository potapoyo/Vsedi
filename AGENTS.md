# AGENTS.md

## Codex専用のGitコミット識別ルール

このルールは、Codexがこのリポジトリでコミットを作成するときだけ適用する。通常のユーザーが作成するコミットのAuthor、Committer、Git設定には影響を与えない。

### Codex用の一時的なGit identity

Codexがコミットを作成するときは、次のidentityをそのコマンドの実行時だけ指定する。

- Author / Committer name: `Codex`
- Author / Committer email: `codex@users.noreply.github.com`

例:

```sh
git -c user.name='Codex' -c user.email='codex@users.noreply.github.com' commit -m '変更内容'
```

`git -c` はその1回のGit実行にだけ有効なため、repo-local、global、system のGit設定を変更しない。コミットを生成する `merge`、`cherry-pick`、`revert` などをCodexが実行する場合も、同じ一時指定を付ける。

### 禁止事項と確認

- `git config user.name` / `git config user.email` を、`--local`、`--global`、`--system` のいずれでも、Codex用identityを恒久設定する目的で実行しない
- Codex用identityをGit alias、hook、環境変数、リポジトリ設定へ恒久的に登録しない
- ユーザーが作成したコミットを、Codexのidentityへ書き換えたり、無断で amend / rebase したりしない
- コミット後に次でAuthorとCommitterの両方を確認する

```sh
git log -1 --format='Author=%an <%ae>%nCommitter=%cn <%ce>%nCommit=%H'
```

identityを明示できないツールでCodexがコミットを作成する必要がある場合は、そのツールによるコミットを避け、上記の一時指定を付けたローカルGit CLIを使う。

### GitHubの認証主体との違い

GitのAuthor / Committerはコミットオブジェクトに保存される文字列メタデータであり、この運用では `Codex` と表示する。一方、GitHub上で表示されるpush、PR、Issue、コメントなどの操作ユーザーは、GitHubへ接続しているアカウントまたはtokenの認証主体で決まる。このルールだけでは、GitHubの認証主体を `Codex` に変更できない。

したがって、Codexが作成したコミットは `Author=Codex` / `Committer=Codex` として識別できても、そのコミットをpushしたユーザー、PRを作成したユーザー、Issueを操作したユーザーは、引き続き認証中のGitHubアカウントとして記録される。また、このidentityは表示上の識別であり、暗号学的な署名やGitHubアカウントによる本人確認を意味しない。
