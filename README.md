# BlockSim

Rust + WebAssembly によるブロック図モデリング・PID 制御実験環境です。連続／離散 PID の出力制限、条件付き積分による anti-windup、ベクトル対応 Saturation ブロックを実装しています。ブラウザ GUI でモデルを設定し、目標値を下げた後の復帰を有効／無効で比較できます。

<p align="center">
  <strong>MODEL · SIMULATE · UNDERSTAND</strong><br>
  <sub>Rust × WebAssembly — ブロックをつなぎ、信号の動きを確かめる。</sub>
</p>

<p align="center">
  <a href="docs/media/hero.gif"><img src="docs/media/hero.gif" width="960" alt="BlockSim hero：PID ブロック図、連続・離散混在、ベクトル、サブシステムを実画面で紹介"></a>
</p>

<p align="center">
  <strong>ブロック図</strong> · <strong>連続／離散</strong> · <strong>ベクトル</strong> · <strong>サブシステム</strong> · <strong>anti-windup</strong>
</p>


任意のブロック図を作成し、サブシステム・ベクトル信号・同期した連続／離散処理を組み合わせて実行できます。


## 動く BlockSim

GIF をクリックすると大きく表示できます。すべて実際の GUI 操作と WASM 計算から録画しています。録画では見やすいレイアウト・見出し・波形の描画演出を加えています。波形の描画演出は録画用で、リアルタイムのシミュレーション再生機能ではありません。

<table>
  <tr>
    <td width="50%" align="center">
      <strong>01 — Build your own signal flow</strong><br>
      <a href="docs/media/build.gif"><img src="docs/media/build.gif" width="460" alt="Constant・Gain・Scope を追加して接続し、ポートをクリックして配線し、入力2×ゲイン3の出力6を計算"></a><br>
      <sub>追加 → ポートをクリックして接続 → 実行。</sub>
    </td>
    <td width="50%" align="center">
      <strong>02 — Compose with subsystems</strong><br>
      <a href="docs/media/subsystems.gif"><img src="docs/media/subsystems.gif" width="460" alt="PID とプラントをサブシステム化し、内部モデルを開き、グループ解除で元のブロック図へ戻す"></a><br>
      <sub>接続を保ったまま階層化・グループ解除。</sub>
    </td>
  </tr>
  <tr>
    <td width="50%" align="center">
      <strong>03 — Saturate. Recover.</strong><br>
      <a href="docs/media/anti-windup.gif"><img src="docs/media/anti-windup.gif" width="460" alt="anti-windup 有効・無効の波形と数値を比較。整定時間26.85秒から7.01秒に改善"></a><br>
      <sub>同じモデルで復帰比較。整定 26.85 s → 7.01 s。</sub>
    </td>
    <td width="50%" align="center">
      <strong>04 — Every component. Every limit.</strong><br>
      <a href="docs/media/vectors.gif"><img src="docs/media/vectors.gif" width="460" alt="3成分のベクトルを Gain・Saturation・Selector で処理し、Scope の表示系列とカーソルを切り替える"></a><br>
      <sub>各成分を独立に制限。必要な系列を選んで確認。</sub>
    </td>
  </tr>
  <tr>
    <td width="50%" align="center">
      <strong>05 — Two clocks. One model.</strong><br>
      <a href="docs/media/hybrid.gif"><img src="docs/media/hybrid.gif" width="460" alt="50ms の離散 PID と連続プラント、100ms の Delay 観測を同じモデルで実行"></a><br>
      <sub>50 ms PID × 連続 RK4 × 100 ms 観測。</sub>
    </td>
    <td width="50%" align="center">
      <strong>06 — Edit without regret</strong><br>
      <a href="docs/media/history.gif"><img src="docs/media/history.gif" width="460" alt="内部 Gain を変更し、Undo・Redo でモデルと波形を戻し、モデルを保存する"></a><br>
      <sub>ゲイン変更、Undo／Redo、保存。試行錯誤を軽く。</sub>
    </td>
  </tr>
</table>

<p align="center">
  <strong>07 — Copy. Zoom. Keep your ideas.</strong><br>
  <a href="docs/media/workspace.gif"><img src="docs/media/workspace.gif" width="960" alt="サブシステムを含むモデルの接続と設定をコピー・貼り付けし、全体表示で見渡して元と同じ数値結果を計算・保存"></a><br>
  <sub>コピー／貼り付け → 全体表示 → 同じ波形を確認 → 保存。</sub>
</p>

<p align="center">
  <strong>08 — From clutter to clarity</strong><br>
  <a href="docs/media/layout.gif"><img src="docs/media/layout.gif" width="960" alt="重なった複製モデルを自動配置で整理し、Undo・Redo、実 WASM 計算と保存を確認"></a><br>
  <sub>自動配置 → Undo／Redo → 同じ数値結果を確認。</sub>
</p>

<p align="center">
  <strong>09 — Find the block. Follow the signal.</strong><br>
  <a href="docs/media/search.gif"><img src="docs/media/search.gif" width="960" alt="同じ ID の内部 Gain を階層パスで区別して検索し、コピー側だけを編集して出力3と5を比較・保存"></a><br>
  <sub>階層をまたいで検索 → 内部へ移動 → 設定を編集。</sub>
</p>

<p align="center">
  <strong>10 — Find the problem. Fix the flow.</strong><br>
  <a href="docs/media/diagnostics.gif"><img src="docs/media/diagnostics.gif" width="960" alt="未接続の Scope 入力をモデル診断で見つけ、対象ポートへ移動して接続し、実 WASM の出力6を確認・保存"></a><br>
  <sub>問題を一覧で確認 → 対象ポートへ移動 → 修正して再実行。</sub>
</p>

<p align="center">
  <strong>11 — Keep a baseline. See the change.</strong><br>
  <a href="docs/media/comparison.gif"><img src="docs/media/comparison.gif" width="960" alt="ゲイン3の波形を基準として保持し、ゲイン5の波形を重ね、t=0.5秒の出力3と5・差2を確認する"></a><br>
  <sub>変更前を保持 → 波形を重ねる → 差分と CSV を確認。</sub>
</p>

<p align="center">
  <strong>12 — Save the run. Reuse the result.</strong><br>
  <a href="docs/media/archive.gif"><img src="docs/media/archive.gif" width="960" alt="設定と全201点の計算結果をファイルに保存し、WASM の再計算と一致を確認して読み込み、現行の出力5と基準3を比較する"></a><br>
  <sub>設定と結果を保存 → 再計算で確認 → 比較の基準へ。</sub>
</p>

[GIF の生成方法とサイズ](docs/media/README.md) · [ブロック図の仕様](docs/graph.md) · [数値検証](docs/validation.md)

<p align="center">
  <strong>13 — Adapt the step. Keep the samples.</strong><br>
  <a href="docs/media/adaptive.gif"><img src="docs/media/adaptive.gif" width="960" alt="同じ1001記録点で RK4 と RK45 を切り替え、一次遅れの微分評価4000回から609回への削減と保存を確認"></a><br>
  <sub>記録時計を保持 → 内部刻みを適応 → 誤差と評価回数を確認。</sub>
</p>

<p align="center">
  <strong>14 — Sweep. Measure. Choose.</strong><br>
  <a href="docs/media/sweep.gif"><img src="docs/media/sweep.gif" width="960" alt="anti-windup を0と1で掃引し、5〜40秒の復帰 IAE 約10.49から0.65への改善を比較、基準保持・設定適用・CSV出力を確認"></a><br>
  <sub>設定を掃引 → IAE／ISE を比較 → 結果を基準やモデルに使う。</sub>
</p>

## さらに使いやすく

- **コピー・複製**：選択内の接続と内部設定を保持。Ctrl/Cmd+A・C・V・D、Undo／Redo に対応し、設定は元モデルから独立します。
- **ブロック検索**：名前・ID・種類で階層内も検索。結果からブロックへ移動して設定を開けます。Ctrl/Cmd+K に対応します。
- **モデル診断**：未接続・幅不一致・代数ループなどを一覧表示。内部ブロックや親の入力へ移動し、修正すると診断が更新されます。
- **結果の比較**：実行結果と設定を基準として保持。変更後の波形・カーソル値・最大差・RMSE を比較し、全対応成分の差分 CSV を書き出せます。
- **実行結果ファイル**：現行／基準の設定と全 Scope の結果を JSON に保存。後から読み込み、WASM の再計算と全記録点の一致を確認して基準へ戻せます。
- **自動配置**：表示中の階層を信号の流れに沿って整理。独立したモデルを分け、閉ループ・Undo／Redo に対応します。
- **ズームと全体表示**：2〜200% 表示。縮小・拡大中もモデル座標で移動でき、全体表示にはフィードバック線も含めます。
- **未保存変更の保護**：モデル置き換えとページ離脱で確認し、Undo で保存時点へ戻ると確認を解除します。
- **ポートから配線**：出力→入力をクリック。Escape で中止し、接続済み入力や幅不一致はその場で拒否します。右の接続フォームも使えます。
- **線を選んで削除**：矢印付きの線をクリック／Enter で選択し、削除ボタンまたは Delete で削除。フィードバック線はブロックの下を通して表示します。
- **グループ解除**：サブシステムの境界をつなぎ直して展開。設定・ベクトル幅・分岐・フィードバックを保持し、Undo／Redo に対応します。
- **モデルを追加読み込み**：読み込み方法を「既存モデルに追加」にすると、元のモデルを残して取り込めます。ID の衝突を回避し、既存の時計・終了時刻を保持します。

## 開発環境

必要なもの：Linux、Bash、C コンパイラ、curl、Node.js 24（`.node-version`）、npm、Rust 1.90.0、`wasm32-unknown-unknown`、wasm-bindgen CLI 0.2.100。ブラウザテストには Chromium が必要です。追加サービス・認証情報は不要です。

```bash
cd /workspace/BlockSim
./scripts/setup.sh
source scripts/rust-env.sh
./scripts/check.sh
./scripts/start.sh
```

`setup.sh` は Rust がなければ導入し、固定バージョンのツールと lockfile から依存関係を準備して、WASM と GUI をビルドします。管理する Rust と npm キャッシュはチェックアウトの外側（クラウドでは `/workspace/.blocksim-tools`）に置きます。既存のシステム Rust があれば利用します。`BLOCKSIM_TOOL_ROOT` で場所を変更できます。Rust のコマンドを単独で実行するときは `rust-env.sh` を source してください。

`web/src/wasm/` は生成物です。Rust を変更したら `./scripts/build-wasm.sh` を再実行してください。`npm run build` は生成済み WASM を使います。`web/dist/` に静的配信用ファイルを出力します。起動後は `node web/scripts/smoke.mjs` で WASM 計算と表示を確認できます。

ブラウザテストは `/usr/bin/chromium` を利用します。別のパスは `CHROMIUM_PATH` で指定できます。システム Chromium がない場合は `cd web && npx playwright install --with-deps chromium` で Playwright のブラウザを導入できます。`--with-deps` の OS パッケージ導入には管理者権限が必要です。

クラウドのタスクは既に分離されています。既存チェックアウトを使い、明示的に依頼されない限り Git worktree は作成しません。公開後や次のタスクではプロセスの再起動が必要です。

## ブロック図エディタ

画面上部の「ブロック図エディタ」でモデルを組めます。Constant、Step、Sine、Gain、Sum、Product、Saturation、Integrator、FirstOrder、Delay、PID、Mux、Selector、Scope、Subsystem を配置・接続できます。ブロックのチェックで複数選択しサブシステム化、内部を編集できます。Undo / Redo、モデル保存・復元、Scope の複数系列とカーソル、CSV 出力にも対応します。作成途中の保存は未接続を許可し、実行時に不足・幅不一致・代数ループを検証します。

連続系の結合 RK4／可変刻み RK45 と整数倍の複数離散周期を混在させ、計算は中止可能な Web Worker で行います。詳細・保存形式・サンプルは [ブロック図仕様](docs/graph.md) を参照してください。

```bash
source scripts/rust-env.sh
cargo run --locked -p blocksim-core --example graph -- samples/graphs/mixed-pid.json
```

「シミュレーション設定」で RK45 の許容誤差・最小／最大内部刻みを保存できます。「パラメータ掃引」は1パラメータを最大32通りで計算し、評価窓の最小／最大・終点・IAE／ISE、微分評価回数を比較します。設定ファイルの保存／復元、全ケースの CSV、結果の基準保持とモデルへの適用に対応します。記録時計と離散時計は保持します。詳しくは [数値計算と掃引](docs/numerics.md) を参照してください。

## PID Lab の操作

- PID の下限／上限は空欄で制限なし。片側だけの制限や同じ上下限も設定できます。制限は連続／離散の両方に適用されます。
- `anti-windup` チェックで実行対象を選択し、「シミュレーション実行」で比較します。有効／無効の両方を同じモデルで計算し、選択側を太線で示します。
- 「復帰比較サンプル」で 5 秒後に目標値を 3 から 0.5 に下げるモデルを読み込みます。出力範囲は `[0, 1]` です。
- Saturation ブロックの JSON 配列は要素ごとに制限します。スカラーは `[2]` のように1要素で入力できます。空配列も対応します。PID 制限とは独立したブロックの試験欄です。
- 「ブラウザに保存」でモデルと Saturation の設定を localStorage に保存し、再読み込み時に自動復元します。「保存設定を復元」で明示的な復元もできます。
- JSON の書き出し／読み込みで設定を移動できます。[サンプル](samples/pid-recovery.json) のモデル JSON も直接読み込めます。実行中の PID 状態ではなく、設定を保存します。読み込みで積分状態はゼロから再実行します。
- 不正な制限・数値・未対応バージョンはエラー表示します。不正な保存ファイルを自動上書きしません。

## 数値検証

```bash
source scripts/rust-env.sh
cargo run --locked -p blocksim-core --example recovery -- samples/pid-recovery.json
```

40 秒の観測、刻み幅 0.01 秒、一次遅れプラント `G(s)=1/(s+1)`、`Kp=2, Ki=1, Kd=0`。以下の時間は 5 秒の目標変更を観測した時点からの秒数です。

| PID | AW | 復帰 IAE | 整定時間 (誤差 ±0.02) | 上限解除 | 最大 \|積分項\| |
|---|---|---:|---:|---:|---:|
| 連続 | 有効 | 0.650079 | 7.01 | 0.00 | 0.499999 |
| 連続 | 無効 | 10.490943 | 26.85 | 18.01 | 10.993262 |
| 離散 | 有効 | 0.650078 | 7.01 | 0.00 | 0.499999 |
| 離散 | 無効 | 10.493422 | 26.86 | 18.01 | 10.998237 |

このサンプルでは IAE は約94%、整定時間は約74%改善しました。特定モデル・観測期間での結果であり、すべての PID チューニングでの改善を保証するものではありません。25 秒では無効側が未整定だったため、比較サンプルは 40 秒まで観測します。

制限なしでは AW の有効／無効が完全に一致するテストと、標準的な無制限 PID の解析解／漸化式に一致するテストを用意しています。元リポジトリに既存モデルはなかったため、過去バージョンの実測との比較は行っていません。新しい制限フィールドを省略した JSON は無制限、AW は無効として読み込みます。

詳細は [PID 数値仕様](docs/pid.md)、実行した検証は [検証記録](docs/validation.md) を参照してください。

## 構成

- `crates/blocksim-core`：Rust PID、Saturation、閉ループ実験、グラフと混在ソルバ、メトリクス。
- `crates/blocksim-wasm`：PID とグラフ JSON の検証／正規化、実行、比較、ベクトル制限の WASM API。
- `web`：TypeScript / Vite GUI、モデル編集・保存・復元、グラフ、Worker、Vitest / Playwright。
- `samples/pid-recovery.json`：再現可能な復帰比較サンプル。

CI は Rust のテスト・fmt・Clippy、WASM ビルド、GUI 型チェックとビルド、設定単体テスト、実際の WASM を使う Chromium GUI テストを実行します。
