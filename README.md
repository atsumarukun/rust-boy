# rust-boy

GameBoy エミュレータを Rust で作成。

本レポジトリは[Rust で作る GAME BOY エミュレータ](https://techbookfest.org/product/sBn8hcABDYBMeZxGvpWapf?productVariantID=2q95kwuw4iuRAkJea4BnKT)の作業レポジトリである。

## 開発環境

開発には Docker を利用。X-Server を介して GUI の表示を実現しているが、Compose ファイルは Linux 用に作成している。

| 環境    | バージョン      |
| ------- | --------------- |
| CPU     | Core i5-13600KF |
| OS      | ArchLinux       |
| Kernel  | 6.9.1-arch1-1   |
| Docker  | 26.1.3          |
| Compose | 2.27.0          |
