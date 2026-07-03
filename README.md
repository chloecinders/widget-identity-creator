# Widget Identity Creator
A tiny desktop app I made for people struggling to apply an application identity for their widget. I tried to make this as idiot-proof as possible, fetching required data automatically using just the bot token and validating the identity data against the widget config and checking things like if the widget config is published.

Why is this a desktop app? Because the alternative would be to make a website which sends your Discord token through an owned proxy (which would then give the developer your bot token, making this unsafe). A tiny open source app is much more secure.

Please see https://chloecinders.com/blog/discord-widgets for the full Discord widget guide.

### Download
https://github.com/chloecinders/widget-identity-creator/releases

### Build from Source
Make sure you have the latest version of Rust installed: https://rustup.rs/. This includes both rustc and cargo. If you are on Linux also make sure you have the correct dependencies for eframe, the UI framework: https://crates.io/crates/eframe. Then run `cargo build --release`, which will deposit the binary into the target folder.

### macOS Note
Because this app is open source and not distributed via the Mac App Store or a paid Apple Developer certificate, macOS Gatekeeper may show a warning saying the app is damaged. To run it, strip the quarantine flag via Terminal:
```bash
xattr -cr /Applications/"Widget Identity Creator.app"
```

<img width="1018" height="925" alt="image" src="https://github.com/user-attachments/assets/4aef5744-6938-4e7b-8439-035549194fd3" />
