# 関数

## 基本関数

### if|T; U|(cond: Bool, then: T, else: U) -> T or U

### map|T; U|(i: Iterable T, f: T -> U) -> Map U

Pythonとは引数の順番が逆なので注意。

### log(x: Object, type: LogType = Info) -> None

`x`をデバッグ表示でログに残す。ログは、実行が終了した後にまとめて表示される。
絵文字対応ターミナルでは`type`に応じてプレフィックスがつく。

* type == Info: 💬
* type == Ok: ✅
* type == Warn: ⚠️
* type == Hint: 💡

### panic(msg: Str) -> Panic

msgを表示して停止する。
絵文字対応ターミナルでは🚨がプレフィックスに付く。

### discard|T|(x: ...T) -> NoneType

`x`を捨てる。戻り値を使用しないときなどに使う。`del`とは違い、変数`x`を参照できなくするわけではない。

```erg
p! x =
    # q!は何らかのNoneや()でない値を返すとする
    # 要らない場合は`discard`を使う
    discard q!(x)
    f x

discard True
assert True # OK
```

### import(path: Path) -> Module or CompilerPanic

モジュールをインポートする。モジュールが見つからない場合、コンパイルエラーを送出する。

### eval(code: Str) -> Object

codeをコードとして評価し返す。

### classof(object: Object) -> Class

`object`のクラスを返す。
ただしクラスは比較できないため、インスタンス判定がしたい場合は`classof(object) == Class`ではなく`object in Class`を使う。
コンパイル時に決定される構造型は`Typeof`で得られる。

## Iterator, Array生成系

### repeat|T|(x: T) -> RepeatIterator T

```erg
rep = repeat 1 # Repeater(1)
for! rep, i =>
    print! i
# 1 1 1 1 1 ...
```

### dup|T; N|(x: T, N: Nat) -> [T; N]

```erg
[a, b, c] = dup new(), 3
print! a # <Object object>
print! a == b # False
```

### cycle|T|(it: Iterable T) -> CycleIterator T

```erg
cycle([0, 1]).take 4 # [0, 1, 0, 1]
cycle("hello").take 3 # "hellohellohello"
```

## 定数式関数

### Class

クラスを新しく生成する。`Inherit`とは違い、`Class`を通すとベース型からは独立し、メソッドは失われる。
比較もできなくなるが、パターンマッチなどは行える。

```erg
C = Class {i = Int}
NewInt = Class Int
Months = Class 1..12
jan = Months.new(1)
jan + Months.new(2) # TypeError: `+` is not implemented for 'Months'
match jan:
    1 -> log "January"
    _ -> log "Other"
```

第二引数のImplは実装するトレイトである。

### Inherit

クラスを継承する。基底クラスのメソッドをそのまま使用できる。

### Trait

トレイトを新しく生成する。現在のところ、指定できるのはレコード型のみ。

### Typeof

引数の型を返す。実行時のクラスを得たい場合は`classof`を使う。
型指定に使うとWarningが出る。

```erg
x: Typeof i = ...
# TypeWarning: Typeof(i) == Int, please replace it
```

### Deprecated

デコレータとして使用する。型や関数が非推奨であると警告する。
