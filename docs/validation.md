# 検証記録

2026-10-12（日本時間）、クラウド環境の `/workspace/BlockSim` で実行。

元リポジトリはソース・コミット・未コミット変更がなく、`main` の fetch は `fatal: couldn't find remote ref main`。ユーザーの新規実装指示に従って作成しました。既存版との比較は未実施です。

## 編集操作の改善（2026-10-12）

Rust 26件、TypeScript 21件、実 WASM の Chromium テスト14件、計61件が成功。型チェック・Vite 本番ビルド・Clippy `-D warnings` も成功しました。失敗・無視・スキップは最終実行で0件です。

追加した確認：ポートのクリック／キーボード配線、Escape キャンセル、既存入力の保護、幅チェック、線の選択・Delete・Undo、サブシステム解除と設定保持、追加読み込みの ID 衝突回避・時計保持・異なる時計の拒否、保存／復元、複数ポートと異なるベクトル幅。

サブシステム解除の前後で、連続 PID サンプルの全4001記録点・2 Scope・時刻・数値結果が完全一致しました。異なる幅の複数入出力を持つモデルでも、ポート1を配線し直してから解除し、期待されるベクトル値を確認しました。

クリックのたびにノードを置き換えていたためダブルクリックが発火しない不具合を再現し、選択時の DOM を保持するよう修正しました。内部モデルをダブルクリックで開くブラウザ検証も成功しています。

README の配線・サブシステム GIF を新しい GUI の実操作で録画し直しました。録画用の演出・再生成方法は `docs/media/README.md` に記載しています。GitHub Actions のクラウド実行自体と新規クラウドタスクの復元は引き続き未検証です。

## ブロック図拡張の検証（2026-10-12）

最新コードで以下を実行しました。

| 検証 | 結果 |
|---|---|
| Rust core PID | 10件成功、従来の無制限・AW・復帰比較を維持 |
| Rust graph | 12件成功、結合 RK4・Step 区間分割・Delay 同期・ベクトル・階層・複数ポート・計算予算 |
| WASM JSON境界（ネイティブ） | 4件成功、グラフ API の追加を含む |
| TypeScript / Vitest | 13件成功、編集履歴・グループ化・保存・CSVを含む |
| Chromium / Playwright | 10件成功、実 WASM・Worker・編集・サブシステム内部・保存復元・中止・接続エラー |
| fmt / Clippy | 両方成功。Clippy は `-D warnings` |
| wasm32 release / wasm-bindgen | 成功 |
| TypeScript 型チェック / Vite 本番ビルド | 成功、Worker と WASM を静的配信用成果物に含む |
| Vite 本番 preview + `smoke.mjs` | PID・エディタ・Worker・ベクトル4系列を実ブラウザで確認 |
| Rust graph example | 混在サンプル 4001点・3 Scope、出力制限 `[0,1]`、最終プラント出力約0.4999995 |

合計49件（Rust 26、TypeScript 13、ブラウザ10）が成功。失敗・無視・スキップは最終実行で0件です。空の unit / doc-test ターゲットは件数に含めていません。GitHub Actions の実行自体、他ブラウザ・他OS・新規クラウドタスクへの復元は未検証です。

現在の通常サンドボックスでは Vite の待ち受けが `listen EPERM: operation not permitted 0.0.0.0:5173` で拒否されました。権限を持つ実行でローカルサーバーと Chromium テストを実行して成功しています。単にサーバー起動失敗を合格にしたものではありません。

グラフ連続 PID と既存 PID Lab のプラント出力は、AW有効／無効の両方で各点の差が `1e-8` 未満。一次遅れの RK4 は解析解への誤差 `1e-6` 未満、刻みを半減すると誤差が10分の1未満。刻み途中の Step + Integrator は厳密な面積との差 `1e-10` 未満。これらは固定したモデル・条件での検証で、任意モデルの安定性を保証しません。

## 初回 PID 実装時の実行済み記録

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
