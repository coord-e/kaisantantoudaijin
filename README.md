# 解散担当大臣

[![Actions Status](https://github.com/coord-e/kaisantantoudaijin/workflows/CI/badge.svg)](https://github.com/coord-e/kaisantantoudaijin/actions?workflow=CI)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](http://makeapullrequest.com)

Discord の通話が解散できないオタクを解散させる解散担当大臣

## Getting Started

```shell
$ export KAISANDAIJIN_DISCORD_TOKEN=<DISCORD BOT TOKEN>
$ docker-compose up -d
```

## Usage

メンションか `!kaisan` でコマンドが実行できます。

- `!kaisan help`: ヘルプ

### 解散コマンド

省略された場合、`TARGET` は全員になります。

- `!kaisan [TARGET] at TIME`: `TARGET` を `TIME` に解散する
- `!kaisan [TARGET] after DURATION`: `TARGET` を `DURATION` 後に解散する
- `!kaisan [TARGET] by TIME`: `TARGET` を `TIME` までのランダムな時間に解散する
- `!kaisan [TARGET] within DURATION`: `TARGET` を `DURATION` 後までのランダムな時間に解散する
- その他さまざまな糖衣構文

#### 解散コマンド例

- `@解散担当大臣 1時間30分後`
- `!kaisan me after 10min`
- `明日の一時半 @解散担当大臣`
- `!kaisan @someone at 10:30`

### 設定コマンド

設定には Manage Guild 権限が必要です。

- `!kaisan show-setting`: 設定表示
- `!kaisan timezone TIMEZONE`: タイムゾーンを設定
- `!kaisan require-permission BOOLEAN`: 他人を解散するのに Move Members 権限を必要とするか設定
- `!kaisan add-reminder N`: 今後の解散の `N` 分前にリマインドを設定
- `!kaisan remove-reminder N`: 今後の解散の `N` 分前のリマインドを削除
- `!kaisan remind-random BOOLEAN`: 解散時刻がランダムな場合にもリマインダを使うかどうか設定

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
