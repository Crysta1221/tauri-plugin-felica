# Wiki の運用・同期ガイド

本リポジトリ内の `wiki/` ディレクトリに配置されている Markdown ファイル群は、そのまま GitHub Wiki に反映できるように構成されています。

このページでは、ローカルの `wiki/` フォルダの内容を GitHub Wiki にデプロイ・同期する手順を説明します。

---

## 前提知識：GitHub Wiki の仕組み

GitHub の Wiki は、メインのリポジトリとは別に、専用の Git リポジトリとして管理されています。

```text
メインリポジトリ:  https://github.com/<owner>/<repo>.git
Wiki 用リポジトリ: https://github.com/<owner>/<repo>.wiki.git
```

そのため、本リポジトリの `wiki/` フォルダ内のファイル群を `wiki.git` に push することで、GitHub Wiki のページが更新されます。

---

## 初回準備（GitHub 側での Wiki 有効化）

1. GitHub のリポジトリページを開き、**Settings** > **General** > **Features** に進みます。
2. **Wikis** にチェックが入っていることを確認します。
3. リポジトリ上部タブの **Wiki** を開き、「**Create the first page**」ボタンを押して最初のページ（内容はデフォルトのままで可）を保存します。
   > [!IMPORTANT]
   > 初回ページを GitHub 上で作成するまで、`<repo>.wiki.git` は生成されません。必ず一度保存を行ってください。

---

## 方法 1: 手動でプッシュする場合

Git コマンドを使って直接 Wiki リポジトリへプッシュする方法です。

```bash
# 1. 一時ディレクトリに Wiki リポジトリをクローン
git clone https://github.com/Crysta1221/tauri-plugin-felica.wiki.git wiki-repo

# 2. wiki/ フォルダの内容をコピー
# (Windows PowerShell の場合)
Copy-Item -Path .\wiki\* -Destination .\wiki-repo\ -Recurse -Force

# 3. 変更をコミットしてプッシュ
cd wiki-repo
git add .
git commit -m "docs: update wiki documentation"
git push origin master

# 4. 一時ディレクトリを削除
cd ..
Remove-Item -Recurse -Force .\wiki-repo
```

---

## 方法 2: GitHub Actions で自動同期する場合（推奨）

本リポジトリには、`main` ブランチにプッシュされた際に `wiki/` ディレクトリの変更を自動で GitHub Wiki に反映する GitHub Actions ワークフロー（`.github/workflows/wiki.yml`）を用意しています。

### 自動同期ワークフローの設定

1. リポジトリの **Settings** > **Actions** > **General** > **Workflow permissions** を開き、「**Read and write permissions**」に設定して保存します。
2. `main` ブランチの `wiki/**` ファイルに変更をコミット＆プッシュすると、GitHub Actions が自動的に Wiki リポジトリへ反映します。
