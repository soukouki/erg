# installサブコマンド

installでレジストリサイトに登録されたパッケージをインストールできる。
基本的な使い方はcargoなどのパッケージマネージャと同じ。

## 便利機能

* 似た名前のパッケージ名があり、そちらのほうが10倍以上ダウンロード数が多かった場合、間違えて入力したのではないかというサジェスチョンが出る。これにより、typo squattingを防止できる。
* パッケージサイズが大きい場合(50MB以上)、サイズを表示して本当にインストールするかサジェスチョンする。
* パッケージがduplicatedになっていた場合、代替のパッケージをサジェスチョンする。
