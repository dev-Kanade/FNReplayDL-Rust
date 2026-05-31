# FNReplayDL-RustAPI

FortniteのトーナメントリプレイファイルをAPI経由でダウンロードし、`.replay`形式で再構築するAPIです。

## 特徴
- Rustによる高速なバイナリ処理
- マルチスレッドによる高速なチャンクダウンロード
- トークンキャッシュ機能
- 中断・再開機能（ダウンロード済みチャンクの自動スキップ）

## 利用方法

[最新リリース](https://github.com/dev-Kanade/FNReplayDL-Rust/releases)から最新のバイナリファイルをダウンロードしてください。</br>

### Linux/Debian
バイナリをダウンロードした場所に移動し、以下のコマンドを実行してください。

```bash
chmod +x ./fnreplaydl-debian
./fnreplaydl-debian
```

### Windows
ダウンロードした.exeファイルを実行してください。

### MacOS
ダウンロードしたバイナリを実行してください。

### 使い方

実行すると ポート`3000`でAPIが起動します。

```bash
curl -X GET "http://localhost:3000/api?match_id=xxxxxxxxxxxxxxx"
```
マッチIDがあっているとダウンロードが開始されます。

## 謝辞
このプロジェクトは[FLJP](https://github.com/Fortniteleakjp)さんの[FNReplayDL-Py](https://github.com/Fortniteleakjp/FNReplayDL-Py)をもとにRustでAPIに書き直したものです。
