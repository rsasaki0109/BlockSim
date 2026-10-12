# 検証記録

2026-10-12（日本時間）、クラウド環境の `/workspace/BlockSim` で実行。

元リポジトリはソース・コミット・未コミット変更がなく、`main` の fetch は `fatal: couldn't find remote ref main`。ユーザーの新規実装指示に従って作成しました。既存版との比較は未実施です。

## 実行済み

| 検証 | 結果 |
|---|---|
| `./scripts/setup.sh` | 成功。依存導入と WASM / GUI ビルドを繰り返して成功 |
| `cargo fmt --all --check` | 成功 |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | 成功 |
| `cargo test --locked --workspace` | Rust core 統合テスト10件、WASM JSON境界のネイティブテスト3件成功 |
| `./scripts/build-wasm.sh` | wasm32 の release ビルドと wasm-bindgen 生成成功 |
| `cd web && npm run build` | TypeScript 型チェックと Vite 本番ビルド成功 |
| `cd web && npm test` | 設定・入力検証の5件成功 |
| `cd web && npm run test:e2e` | Chromium の5件成功、生成 WASM バイナリを実際にロードして実行 |
| `./scripts/check.sh` | 上記チェック一式が終了コード0 |
| `./scripts/start.sh` と `node web/scripts/smoke.mjs` | 開発サーバー起動、WASM計算、IAE比較、グラフ3系列、ベクトル出力を確認 |
| `cargo run --locked -p blocksim-core --example recovery` | 両PIDの数値結果を出力、READMEの表と一致 |

Rust の空の unit / doc-test ターゲットは0件で、検証件数には含めていません。失敗・無視・スキップされたテストは最終チェックで0件です。

ブラウザテストで検証した内容：連続／離散、無制限時の AW 有効／無効の完全一致、出力 `[0,1]`、IAE の改善、GUI のモード・AW・上下限編集、ブラウザ保存・再読み込み・復元、JSON export / import、省略フィールドの無制限既定値、不正設定の拒否、Saturation の片側／等しい／未設定の制限。

初回の再セットアップでは Rust 配布サーバーが HTTP 503 を返しました。導入済みの固定バージョン・コンポーネント・ターゲットを再利用し、不足分だけを取得するよう修正しました。TLS / checksum の検証は無効化していません。

## 範囲と未実施

- GitHub Actions のクラウド実行自体は未実施です。CI と同じ `check.sh` をこの環境で実行しました。
- 他の OS、Firefox / WebKit、任意のプラント・ゲインの安定性は未検証です。
- 制限は制御出力に適用し、プラント出力を強制的に制限するものではありません。
- ブラウザ保存は設定のみ。実行中の状態を中断／再開する機能は未実装です。
- GitHub への push、クラウド環境の公開、新規タスクでの復元確認は未実施です。元リモートに参照がないため、ローカルコミットの新規タスクへの復元を保証しません。
- Simulink 比30%は目標とする主観的な開発評価です。客観的な達成率・互換性の証明は行っていません。
