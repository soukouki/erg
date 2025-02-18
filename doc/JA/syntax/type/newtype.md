# Newtype pattern

ここでは、Rustでよく使われるnewtypeパターンのErg版を紹介します。

Ergはでは以下のように型のエイリアスを定義することができますが、これはあくまで同じ型を指します。

```erg
UserId = Int
```

なので、例えば`UserId`型の数値は8桁の正数、という仕様があったとしても、`Int`型と同じなので10でも-1でも入れられてしまうわけです。`Nat`にすれば-1は弾くことができますが、8桁の数という性質はErgの型システムのみでは表現できません。

また、例えばあるデータベースのシステムを設計する時、いくつかの種類のIDがあったとします。ユーザーID, 商品ID, 注文IDなどとIDの種類が増えてくると、関数に違う種類のIDを渡すというバグが発生する可能性があります。ユーザーIDと商品IDなどは構造的に等価であっても、意味論的には異なるわけです。

newtypeパターンはこのような場合に適したデザインパターンです。

```erg
UserId = Class {id = Nat}
UserId.
    new id: Nat =
        assert id.dights().len() == 8, else="UserId must be a positive number with length 8"
        UserId::__new__ {id;}

i = UserId.new(10000000)
print! i # <__main__.UserId object>
i + UserId.new(10000001) # TypeError: + is not implemented between `UserId` and `UserId`
```

コンストラクタが8桁の数という事前条件を保証してくれます。
この`UserId`は`Nat`の持つメソッドをすべて失ってしまうので、必要な演算を都度再定義する必要があります。
再定義するコストが見合わない場合は、継承を使う方がよいでしょう。逆にメソッドがなくなるという性質が望ましい場合もあるので、状況に応じて適切な方法を選んでください。
