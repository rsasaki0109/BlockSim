# BlockSim

Rust + WebAssembly によるブロック図モデリング・PID 制御実験環境です。連続／離散 PID の出力制限、条件付き積分による anti-windup、ベクトル対応 Saturation・Dead Zone ブロックを実装しています。ブラウザ GUI でモデルを設定し、目標値を下げた後の復帰を有効／無効で比較できます。

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

<p align="center">
  <strong>15 — One model. Two views.</strong><br>
  <a href="docs/media/frequency.gif"><img src="docs/media/frequency.gif" width="960" alt="同じ2次ローパスモデルを時間応答と Bode 線図で確認。角周波数1 rad/sの振幅−6.02 dB・位相−90度、全201点のCSVを実 WASM で計算"></a><br>
  <sub>伝達関数を設定 → 周波数応答を解析 → カーソルと全点 CSV を確認。</sub>
</p>

<p align="center">
  <strong>16 — Find the crossings. Read the margins.</strong><br>
  <a href="docs/media/margins.gif"><img src="docs/media/margins.gif" width="960" alt="明示した開ループのゲイン・位相交差を実 WASM で解析。ゲイン4から8で位相余裕11.42から−7.52度、ゲイン余裕3.52から−2.50 dBを確認し、CSVを保存"></a><br>
  <sub>開ループ設定を復元 → 全交差と余裕を確認 → 設定変更後の符号と CSV を確かめる。</sub>
</p>

<p align="center">
  <strong>17 — Follow the poles.</strong><br>
  <a href="docs/media/pole-zero.gif"><img src="docs/media/pole-zero.gif" width="960" alt="仮定した単位負帰還の極・零点を実 WASM で計算。ゲイン4・6・8で減衰する極、虚軸の境界、右半平面の極2個を区別し、全根のCSVを保存"></a><br>
  <sub>設定を復元 → 複素平面と Routh 判定を確認 → 境界と不安定を区別して保存。</sub>
</p>

<p align="center">
  <strong>18 — Current wires. Real matrices.</strong><br>
  <a href="docs/media/state-space.gif"><img src="docs/media/state-space.gif" width="960" alt="現在の負帰還接続から状態空間行列と固有値を実 WASM で抽出。ゲイン4・6・8で安定・境界・不安定を区別し、行列と全極のCSVを保存"></a><br>
  <sub>接続から A・B・C・D を抽出 → ゲイン変更と固有値を確認 → CSV を保存。</sub>
</p>

<p align="center">
  <strong>19 — One clock. A unit circle.</strong><br>
  <a href="docs/media/discrete-state-space.gif"><img src="docs/media/discrete-state-space.gif" width="960" alt="同一周期のDelay負帰還から離散状態空間行列と無次元の極を実WASMで抽出。極の半径0.8・1・1.2で安定・境界・不安定を区別し、周期と全行列のCSVを保存"></a><br>
  <sub>離散行列を抽出 → 単位円の内側・円上・外側を確認 → 周期と結果を保存。</sub>
</p>

<p align="center">
  <strong>20 — Samples. Frequencies. One model.</strong><br>
  <a href="docs/media/discrete-frequency.gif"><img src="docs/media/discrete-frequency.gif" width="960" alt="同一周期の離散負帰還と1サンプルDelayのBode応答を実WASMで計算。周期とNyquist周波数、振幅1・Nyquistで位相−180度、全201点CSVを確認"></a><br>
  <sub>単位円上で離散応答を計算 → 周期とNyquist上限を確認 → 全点を保存。</sub>
</p>

<p align="center">
  <strong>21 — Sampled loops. Measured margins.</strong><br>
  <a href="docs/media/discrete-margins.gif"><img src="docs/media/discrete-margins.gif" width="960" alt="同一周期の離散開ループのゲイン・位相余裕を実WASMで計算。ゲイン0.75・1・1.25で正・ほぼゼロ・負の余裕、Nyquist上限、周期付き全候補CSVを確認"></a><br>
  <sub>離散の開ループを明示 → 検出した交差と余裕を確認 → 周期・範囲と候補を保存。</sub>
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

連続線形モデルでは、選択した Scope 成分に関係する接続から [状態空間行列と内部安定性](docs/state-space.md) を解析できます。最大16状態の A・B・C・D、固有値と残差を表示し、設定の保存／復元、行列・極の CSV／結果 JSON に対応します。同じ周期のDelay・離散PIDでは [離散状態空間と単位円](docs/discrete-state-space.md)、[離散BodeとNyquist上限](docs/discrete-frequency.md)、[離散開ループの余裕](docs/discrete-margins.md) を選択できます。Scope に関係しない枝を含むモデル全体の安定性判定は対象外です。

<p align="center">
  <strong>22 — Coefficients in. Samples out.</strong><br>
  <a href="docs/media/discrete-transfer.gif"><img src="docs/media/discrete-transfer.gif" width="960" alt="離散伝達関数の直接項と1サンプル遅れのStep応答を比較し、実WASMの離散Bodeと周期付きCSVを確認"></a><br>
  <sub>離散IIR／FIRの係数・初期状態・周期を設定。サンプル時刻と保持を確認。</sub>
</p>

[離散伝達関数](docs/discrete-transfer.md) で離散フィルターの係数を直接指定できます。

<p align="center">
  <strong>23 — Convert. Inspect. Keep the frequency.</strong><br>
  <a href="docs/media/tustin.gif"><img src="docs/media/tustin.gif" width="960" alt="連続伝達関数をTustin法とプリワープで離散化し、変換係数を確認して適用、実WASMのStepと指定周波数のBodeを検証"></a><br>
  <sub>係数を確認 → 離散化 → 実行。プリワープで指定周波数を一致。</sub>
</p>

[Tustin離散化](docs/tustin.md) で連続伝達関数を離散ブロックへ変換できます。

<p align="center">
  <strong>24 — Hold the input. Match the samples.</strong><br>
  <a href="docs/media/zoh.gif"><img src="docs/media/zoh.gif" width="960" alt="連続ローパスをゼロ次ホールドで離散化し、入力を途中で下げた応答のサンプル時刻の一致を確認、保存と離散Bodeを実WASMで実行"></a><br>
  <sub>ZOHで離散化。保持入力に対するサンプル時刻の応答を合わせる。</sub>
</p>

[ZOH離散化](docs/zoh.md) で保持入力に対する応答を離散モデルへ変換できます。

<p align="center">
  <strong>25 — Convert. Compare. Measure.</strong><br>
  <a href="docs/media/conversion-comparison.gif"><img src="docs/media/conversion-comparison.gif" width="960" alt="選択した伝達関数の単位ステップを連続と離散で比較し、ZOHのサンプル時刻の一致とTustinの初回差を数値確認、設定と全点CSVを保存"></a><br>
  <sub>同じサンプル時刻で重ねる。最大差・RMS差を確認して、実行条件と全点を保存。</sub>
</p>

[離散化前後の比較](docs/conversion-comparison.md) で、選択した伝達関数の単位ステップ応答を変換前に確認できます。

<p align="center">
  <strong>26 — Drop the input. Compare the return.</strong><br>
  <a href="docs/media/held-comparison.gif"><img src="docs/media/held-comparison.gif" width="960" alt="2秒で入力を1から0に下げ、ZOHとTustinの変更後の応答を実WASMで比較し、変更後の最大差とRMS差、入力条件付きCSVと設定を保存・復元"></a><br>
  <sub>入力を途中で下げる。変更後の差を確かめて、入力条件ごと保存する。</sub>
</p>

[保持入力の比較](docs/conversion-comparison.md) では、入力の変更前・変更後・切り替えサンプルを設定できます。

<p align="center">
  <strong>27 — Sample the sine. Hold the input.</strong><br>
  <a href="docs/media/sine-comparison.gif"><img src="docs/media/sine-comparison.gif" width="960" alt="正弦波をサンプルして保持した入力で連続と離散を比較し、ZOHの一致とTustinの差を数値確認、周波数・位相・実際の入力系列付きCSVと設定を保存・復元"></a><br>
  <sub>正弦波をサンプルして保持。同じ入力で比較し、実行条件と入力系列を保存。</sub>
</p>

[保持正弦波の比較](docs/sine-comparison.md) で、周波数・振幅・位相・オフセットを設定できます。

<p align="center">
  <strong>28 — Data in. Held inputs out.</strong><br>
  <a href="docs/media/sequence-comparison.gif"><img src="docs/media/sequence-comparison.gif" width="960" alt="任意の101点入力系列をCSVから読み込み、Sampled Inputで保持してZOHとTustinを比較し、実入力・全応答・設定を保存と復元"></a><br>
  <sub>入力CSVを読み込む。値を順番に保持して、応答と差を確かめる。</sub>
</p>

[Sampled Inputと系列比較](docs/sampled-input.md) はベクトルの保持と、任意のサンプル入力による離散化比較に対応します。

<p align="center">
  <a href="docs/media/repeat-comparison.gif"><img src="docs/media/repeat-comparison.gif" width="960" alt="3行の入力CSVを繰り返し101点のZOHとTustin応答を比較し、周期番号と行番号を保存して設定を復元"></a><br>
  <b>短い入力を繰り返す。周期ごとの応答を比較する。</b><br>
  ベクトル対応の周期再生、比較点数の指定、周期・行番号付きCSV、設定の保存と復元。<br>
  <a href="docs/sampled-input.md">使い方と数値検証</a>
</p>

<p align="center">
  <a href="docs/media/vector-input-csv.gif"><img src="docs/media/vector-input-csv.gif" width="960" alt="2列のCSVをSampled Inputの設定欄に読み込み、適用してベクトルを周期再生し、最終行保持とUndo、入力表の保存を確認"></a><br>
  <b>データを読み込む。すべての成分を動かす。</b><br>
  複数列の入力CSV、設定欄での確認、適用とUndo、周期再生と最後の行の保持。<br>
  <a href="docs/sampled-input.md">入力表CSVの使い方</a>
</p>

<p align="center">
  <a href="docs/media/input-table-editor.gif"><img src="docs/media/input-table-editor.gif" width="960" alt="Sampled Inputの2成分をセルで編集し、行を追加・複製・削除して適用し、周期再生とCSV保存、Undoを確認"></a><br>
  <b>セルを編集する。行を組み立てる。波形を確かめる。</b><br>
  数値の直接入力、行追加・複製・削除、ページ移動、JSON／CSV同期。<br>
  <a href="docs/sampled-input.md">入力表の編集方法</a>
</p>

<p align="center">
  <a href="docs/media/input-preview.gif"><img src="docs/media/input-preview.gif" width="960" alt="適用前のSampled Inputを保持波形でプレビューし、繰り返しと最終行保持、成分と取得値カーソル、結果JSONを確認"></a><br>
  <b>適用前に、入力の波形を確かめる。</b><br>
  最大3周期の保持波形、成分表示、取得値カーソル、実行時の条件と全点の保存。<br>
  <a href="docs/sampled-input.md">入力プレビューの使い方</a>
</p>

<p align="center">
  <a href="docs/media/input-row-preview.gif"><img src="docs/media/input-row-preview.gif" width="960" alt="入力表の行とプレビューカーソルを連動し、周期と保持区間のマーカー、繰り返し終端と最終行保持を確認"></a><br>
  <b>行を選ぶ。波形の同じ場所を確かめる。</b><br>
  表とカーソルの双方向連動、ページ追従、保持区間と取得値のマーカー。<br>
  <a href="docs/sampled-input.md">表と波形を対応させる</a>
</p>

<p align="center">
  <a href="docs/media/input-generator.gif"><img src="docs/media/input-generator.gif" width="960" alt="成分ごとの定数・ステップ・ランプ・正弦波を入力表へ生成し、実WASMの保持波形を確認してモデルへ適用"></a><br>
  <b>入力を作る。波形を確かめる。</b><br>
  4種類の波形からベクトル入力表を生成。確認してからモデルへ適用。<br>
  <a href="docs/sampled-input.md">入力表を生成する</a>
</p>

<p align="center">
  <a href="docs/media/input-generator-file.gif"><img src="docs/media/input-generator-file.gif" width="960" alt="波形生成条件をJSONへ保存し、現在の入力表と実取得結果を保持して条件を読み込み、同じランプを再生成"></a><br>
  <b>条件を保存。次の入力に再利用。</b><br>
  生成条件だけを復元し、波形を確かめながら同じ入力を作り直す。<br>
  <a href="docs/sampled-input.md">生成条件を保存する</a>
</p>

<p align="center">
  <a href="docs/media/input-phase.gif"><img src="docs/media/input-phase.gif" width="960" alt="正弦波の成分別位相を指定して90度差の保持入力を作り、条件JSONの保存復元と旧条件の位相0互換を実WASMで確認"></a><br>
  <b>位相を変える。成分ごとの動きを見る。</b><br>
  90度差のベクトル入力、成分別の初期位相、生成条件の保存と再利用。<br>
  <a href="docs/sampled-input.md">位相付き入力を作る</a>
</p>

<p align="center">
  <a href="docs/media/input-periodic.gif"><img src="docs/media/input-periodic.gif" width="960" alt="パルス列・矩形波・三角波を入力表へ生成し、成分別デューティ比と境界、条件保存復元、モデル適用後の実WASM値を確認"></a><br>
  <b>切り替える。繰り返す。入力を試す。</b><br>
  パルス列・矩形波・三角波。成分別デューティ比と実際の保持入力を確認。<br>
  <a href="docs/sampled-input.md">周期波形の入力を作る</a>
</p>

<p align="center">
  <a href="docs/media/input-generation-plan.gif"><img src="docs/media/input-generation-plan.gif" width="960" alt="入力表を置換する前に時間・成分範囲・指定と実効デューティ比を確認し、生成後の実WASM取得値と照合"></a><br>
  <b>生成する前に、条件の違いを確かめる。</b><br>
  保持時間・予定範囲・実効デューティ比を確認。前の結果を保持して入力を調整。<br>
  <a href="docs/sampled-input.md">生成予定を確認する</a>
</p>

<p align="center">
  <a href="docs/media/dead-zone.gif"><img src="docs/media/dead-zone.gif" width="960" alt="2成分の入力をDead Zoneで処理し、範囲内の0出力、範囲外の超過分、設定変更とUndo／Redoを実WASMで確認"></a><br>
  <b>小さな変動は0に。超えた分だけ、先へ。</b><br>
  成分ごとの入力と出力を比較。非対称の不感帯を設定して保存。<br>
  <a href="docs/dead-zone.md">Dead Zoneを使う</a>
</p>

<p align="center">
  <a href="docs/media/rate-limiter.gif"><img src="docs/media/rate-limiter.gif" width="960" alt="2成分の目標値を離散Rate Limiterで処理し、上昇と下降の変化量・初期値・保持・Undo／Redoを実WASMで確認"></a><br>
  <b>目標値が変わっても、進み方を決められる。</b><br>
  上昇・下降速度を個別に設定。各周期の変化量を制限して保持。<br>
  <a href="docs/rate-limiter.md">Discrete Rate Limiterを使う</a>
</p>

<p align="center">
  <a href="docs/media/large-state-space.gif"><img src="docs/media/large-state-space.gif" width="960" alt="16状態の行列と全固有値を解析し、可制御性・可観測性の数値ランク、閾値付近と未解決、離散領域と設定復元を実WASMで確認"></a><br>
  <b>大きなモデルも、状態を残して確かめる。</b><br>
  最大16状態の解析。数値ランクと、その判定の限界も表示。<br>
  <a href="docs/structural-analysis.md">状態と数値ランクを確認する</a>
</p>

<p align="center">
  <a href="docs/media/mimo.gif"><img src="docs/media/mimo.gif" width="960" alt="2入力2出力の共有状態を解析し、全4経路の周波数応答・数値ランク・零応答・離散周期と設定復元を実WASMで確認"></a><br>
  <b>複数の入力から、複数の出力を確かめる。</b><br>
  共有状態を保持して解析。経路を切り替え、行列と全取得値を保存。<br>
  <a href="docs/mimo.md">複数入出力を解析する</a>
</p>

<p align="center">
  <a href="docs/media/matrix-block.gif"><img src="docs/media/matrix-block.gif" width="960" alt="StateSpaceブロックの行列と初期状態を設定し、結合した2成分の時間応答・離散の出力保持・MIMO解析と設定復元を実WASMで確認"></a><br>
  <b>行列からモデルを作り、そのまま動かす。</b><br>
  A・B・C・Dと初期状態を編集。連続・離散の時間応答から全経路の解析へ。<br>
  <a href="docs/matrix-block.md">StateSpaceブロックを使う</a>
</p>

<p align="center">
  <a href="docs/media/matrix-discretization.gif"><img src="docs/media/matrix-discretization.gif" width="960" alt="StateSpace行列をZOHとプリワープ付きTustinで離散化し、係数確認・適用・Undo・全取得点の独立式照合と保存復元を実WASMで確認"></a><br>
  <b>連続の行列を、確かめて離散モデルへ。</b><br>
  ZOHとTustinを選択。変換行列を確認し、時計と接続を保持して適用。<br>
  <a href="docs/matrix-discretization.md">行列を離散化する</a>
</p>

<p align="center">
  <a href="docs/media/matrix-comparison.gif"><img src="docs/media/matrix-comparison.gif" width="960" alt="行列の連続応答とZOH・Tustinの離散応答を同じベクトル入力で比較し、全成分の最大差・RMS・入力変更後の差分と条件復元を実WASMで確認"></a><br>
  <b>変換前と変換後、同じ入力で確かめる。</b><br>
  出力を切り替えて波形と差分を比較。実行条件と全取得値を保存。<br>
  <a href="docs/matrix-comparison.md">行列の応答を比較する</a>
</p>

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

画面上部の「ブロック図エディタ」でモデルを組めます。Constant、Step、Sine、Gain、Sum、Product、Saturation、Integrator、FirstOrder、TransferFunction、Delay、PID、Mux、Selector、Scope、Subsystem を配置・接続できます。ブロックのチェックで複数選択しサブシステム化、内部を編集できます。Undo / Redo、モデル保存・復元、Scope の複数系列とカーソル、CSV 出力にも対応します。作成途中の保存は未接続を許可し、実行時に不足・幅不一致・代数ループを検証します。

連続系の結合 RK4／可変刻み RK45 と整数倍の複数離散周期を混在させ、計算は中止可能な Web Worker で行います。詳細・保存形式・サンプルは [ブロック図仕様](docs/graph.md) を参照してください。

```bash
source scripts/rust-env.sh
cargo run --locked -p blocksim-core --example graph -- samples/graphs/mixed-pid.json
```

「シミュレーション設定」で RK45 の許容誤差・最小／最大内部刻みを保存できます。「パラメータ掃引」は1パラメータを最大32通りで計算し、評価窓の最小／最大・終点・IAE／ISE、微分評価回数を比較します。設定ファイルの保存／復元、全ケースの CSV、結果の基準保持とモデルへの適用に対応します。記録時計と離散時計は保持します。詳しくは [数値計算と掃引](docs/numerics.md) を参照してください。

「周波数応答 / Bode」では、連続線形モデルの信号源／Scope と各成分を選び、振幅・位相・複素応答を解析できます。明示した開ループの全ゲイン／位相交差と余裕、未解決区間、解析設定の保存／復元、全点・余裕 CSV と中止に対応します。未検出は無限大とせず、安定性の判定は行いません。TransferFunction は8次までの実係数、直接伝達、独立したベクトル成分を扱います。対応範囲と検証は [伝達関数と周波数応答](docs/frequency.md) を参照してください。

「伝達関数の極・零点 / 安定性」では、選択ブロックと仮定した単位負帰還の極・零点、Routh表による数値判定を確認できます。極零相殺はせず、境界／近傍・判定不能・求根未解決も表示します。複素平面・全根のCSV・設定保存／復元に対応します。ブロック図全体の安定性は判定しません。[仕様と数値検証](docs/pole-zero.md) を参照してください。

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
