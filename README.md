This is a minimal reproducible demo of the possible deadlock in [sender.lock()] in the `grammers-client` crate.

To reproduce it, put the `session` file at the root of the project, and run the following command:

```sh
cargo run --bin deadlock
```

Please note that a main.log file will be created in the current directory, and it can be extremely verbose and large.

The logic to trigger the deadlock, inside [src/main.rs], is quite self-explanatory so please refer to it for more
details.

This bug is tested reproducible on the following platforms:

- Windows Server 2022, Rust 1.80.0
- Mate 60 Pro, HarmonyOS Next NEXT.0.0.71 (tested inside the [Homogram] APP, not this binary due to the lack of
  execution permission on its tmp directory, pending Huawei's update)

Some extra information:
I met this bug after I joined several groups with ~200K members. I'm not sure if it's related, but as such groups are
common in Telegram, it's worth mentioning.

[sender.lock()]: https://github.com/Lonami/grammers/blob/master/lib/grammers-client/src/client/net.rs#L419

[src/main.rs]: src/main.rs

[Homogram]: https://github.com/HomoArk/Homogram