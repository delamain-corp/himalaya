# Claude 開発ガイドライン

## リポジトリについて

このリポジトリは [pimalaya/himalaya](https://github.com/pimalaya/himalaya) の**Forkリポジトリ**です。

Himalayaは、Rustで書かれたCLIベースのメールクライアントです。IMAP、Maildir、Notmuchなど複数のバックエンドをサポートしています。

## 重要な注意事項

### Issue・PRの作成ルール

- **IssueやPRはこのForkリポジトリ（delamain-corp/himalaya）内で完結させてください**
- **Fork元リポジトリ（pimalaya/himalaya）へのIssue作成やPR送信は禁止です**
- PRのベースブランチは必ずこのリポジトリ内のブランチ（例: `master`）を指定してください

### ブランチ運用

- デフォルトブランチ: `master`
- 機能追加ブランチ: `feature/<作業名>`
- バグ修正ブランチ: `fix/<作業名>`

## 開発環境

### 必要なツール

- Rust（toolchainバージョンは `rust-toolchain.toml` を参照）
- Cargo

### ビルド

```bash
cargo build --release
```

### テスト

```bash
cargo test
```

### 機能フラグ

このプロジェクトはCargoのfeature flagsを使用しています。主な機能:

- `imap` - IMAPバックエンド
- `maildir` - Maildirバックエンド
- `notmuch` - Notmuchバックエンド
- `smtp` - SMTPバックエンド
- `keyring` - システムキーリング連携
- `oauth2` - OAuth 2.0認証
- `pgp-*` - PGP暗号化機能

## コミットメッセージ

Conventional Commits形式を使用してください:

```
feat: 新機能の追加
fix: バグ修正
docs: ドキュメントの変更
refactor: リファクタリング
test: テストの追加・修正
chore: その他の変更
```

## 設定ファイル

- `config.sample.toml` - 設定ファイルのサンプル
- `Cargo.toml` - プロジェクト設定とデフォルト機能の定義
