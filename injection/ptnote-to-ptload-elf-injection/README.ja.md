# PT_NOTE → PT_LOAD 置き換えによるELF感染方法

English Version: [README.md](README.md)

> 日本語は母語ではありません。文がおかしいところがあるかもしれないので、そういうところを指摘していただければ嬉しいです。宜しくお願いいたします！ (連絡: https://xoreaxe.ax/contact.txt)

[SymbolCrashのブログ](https://www.symbolcrash.com/2019/03/27/pt_note-to-pt_load-injection-in-elf/)を読みながら、ELFのプログラムヘッダの`PT_NOTE`を`PT_LOAD`に置き換えることでシェルコードのロード及び実行ができる技を知りました。掲載を読むときELFについてあんまりわかっていませんでしたが、この技は気になって、実装してみたので今回学んだことを共有していきたいと思います。

ELFファイルのメタデータの読み・書き込みが簡単にできるように、[mental_elf](https://github.com/d3npa/mental-elf)という、まだ未完全、小さなライブラリを作ってみました。ライブラリのコード自体は単純で読めばわかりやすいと思うので、ここでは詳しく説明しません。代わりに感染方法に集中して解説していきます。

## 概要

タイトルのどおりこの感染方法は、あるELF実行可能ファイル（以降ELFと呼ぶ）のプログラムヘッダーを編集し、`PT_NOTE`を`PT_LOAD`に置き換えます。感染の流れは次の3段階になります：

- シェルコードをELFの末尾に追加する
- 実行時シェルコードが決まった仮想アドレスに読み込まれるようにする
- シェルコードが最初に実行されるように、ELFのエントリポイントを書き換える

シェルコードが処理を終わったら本来のエントリポイントに処理を渡すように、感染時に元々のエントリポイントから `jmp` 命令を生成し、シェルコードをパッチする必要があります。

ELFの末尾に追加されたシェルコードは、`PT_LOAD`というプログラムヘッダーによって仮想メモリに読込できますが、新たなヘッダーをELFに投入してしまえばバイナリ内の他のオフセットを壊してしまうでしょう。[ELFの仕様](http://www.skyfree.org/linux/references/ELF_Format.pdf)によると、`PT_NOTE`という別のヘッダーがありますが、そのヘッダーはELF仕様で任意とされています。もし既存の`PT_NOTE`ヘッダーを置き換えれば、オフセットを壊さずに`PT_LOAD`を改竄することが出来るのです。

> Note information is optional.  The presence of note information does not affect a program’s ABI conformance, provided the information does not affect the program’s execution behavior.  Otherwise, the program does not conform to the ABI and has undefined behavior

この方法には、2つの欠点があります

- この実装はPIE(位置独立実行形式)なELFは対応されていない
- Go言語のランタイムは、バージョン情報を確認するため有効な`PT_NOTE`を期待するので書き換えできない

※PIEは、ccなら`-no-pie`、rustcなら`-C relocation-model=static`というコンパイラオプションで無効化出来ます。

# シェルコード

この例で提供したシェルコードはNASMで書いていますので、Makefileを実行する前に`nasm`がインストールされていることを予め確認してください。

この方法で使えるシェルコードを生成するにはいくつか注意しなければならない点があります。[AMD64 System V ABI](https://refspecs.linuxfoundation.org/elf/x86_64-abi-0.95.pdf)第3.4.1章では、プログラムの開始時(シェルコードの後本体のエントリポイントに処理を渡す時点)に`rbp`、`rsp`、`rdx`のレジスタが有効な値を持たなければならないと書いてあります。単に、シェルコードの冒頭でそれらのレジスタを`push`し、処理後に`pop`すればよいのです。自分のシェルコードは、`rbp`、`rsp`を触れないので、最後に`rdx`だけをゼロに戻しています。

また、シェルコードが処理を終わったら本体のエントリポイントに処理を渡すためには、本来のエントリポイントから`jmp`命令を作り、シェルコードに追加する必要があります。シェルコードは、上から下まで実行するように書くか、下記のように最後に空のラベルを用意してそれに`jmp`すれば、パッチはシェルコードの末尾に新しい命令を追加しただけで実行されるので便利です。

```nasm
main_tasks:     ; メインタスク
    ; ...
    jmp finish  ; シェルコードの末尾にジャンプ
other_tasks:    ; その他のタスク
    ; ...
finish:         ; からのラベル「終わり」
```

x86_64では、`jmp`命令に64ビットの引数を渡すことが不可能なので、一度64ビットなエントリポイントを`rax`に保存し、`jmp rax`を行います。下記は、そのようにシェルコードをバッチするRust言語のスニペットです。

```rust
fn patch_jump(shellcode: &mut Vec<u8>, entry_point: u64) {
    // エントリポイントをraxに
    shellcode.extend_from_slice(&[0x48u8, 0xb8u8]);
    shellcode.extend_from_slice(&entry_point.to_ne_bytes());
    // raxの値を移動先にしてジャンプ
    shellcode.extend_from_slice(&[0xffu8, 0xe0u8]);
}
```

## 感染プログラム

感染プログラムのソースコードは `src/main.rs` にあります。上から下までという形になっているので、概要を理解した上でソースコードを読めばわかりやすいかと思います。また、ライブラリの[mental_elf](https://github.com/d3npa/mental-elf)を利用していて、ファイル処理などほとんど抽象されているので、感染方法に着目できます。

メイン関数の流れは以下のようです:

- 対象ELFファイル、シェルコードファイルのCLI引数2つを取る
- ELFファイルのELFヘッダーとプログラムヘッダーを読み込む
- 本来のエントリポイントを使ってシェルコードに`jmp`命令を追加する
- プログラムヘッダーから`PT_NOTE`を取り、`PT_LOAD`に書き換える
- シェルコードの冒頭を指すようにELFのエントリポイントを書き換える
- 変更済みなヘッダーをELFファイルに書き込む

感染したELFファイルが実行すれば、まずELFローダーは、複数のセクションを仮想メモリに読み込みます。改竄した`PT_LOAD`も処理されるのでELFの末尾に追加したシェルコードも読み込まれます。ELFのエントリポイントがシェルコードの冒頭を指すので、シェルコードの実行が始まります。シェルコードの処理が終わったら、パッチした`jmp`命令が実行され、ELFの本来のエントリポイントに移動し、本来のプログラムが普通に実行されます。

```
$ make
cd files && make && cd ..
make[1]: Entering directory '/.../files'
rustc -C opt-level=z -C debuginfo=0 -C relocation-model=static target.rs
nasm -o shellcode.o shellcode.s
make[1]: Leaving directory '/.../files'
cargo run --release files/target files/shellcode.o
   Compiling mental_elf v0.1.0 (https://github.com/d3npa/mental-elf#0355d2d3)
   Compiling ptnote-to-ptload-elf-injection v0.1.0 (/...)
    Finished release [optimized] target(s) in 1.15s
     Running `target/release/ptnote-to-ptload-elf-injection files/target files/shellcode.o`
Found PT_NOTE section; converting to PT_LOAD
echo 'Done! Run target with: `./files/target`'
Done! Run target with: `./files/target`
$ ./files/target
dont tell anyone im here
hello world!
$
```

## 後書き

このプロジェクトで、ELFファイルの構造、Rust言語でのデータ読み込み、ヴィルすについて沢山学ぶことが出来ました。私を支えて、いろいろ教えてくれたtmp.outの皆様にも、ありがとうございます！♡

今回の掲載はここまでです。最後まで読んでいただき、ありがとうございました！